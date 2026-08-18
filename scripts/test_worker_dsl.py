#!/usr/bin/env python3
"""
Automated Test Suite for Stage 2: Worker In-Context DSL & Schema Validation.

Verifies:
1. State 1: h0: And(Prop("A"), Prop("B")) ⊢ And(Prop("B"), Prop("A"))
   - Actor emits a valid AndElimR or AndElimL AST.
   - Critic scores {"rule": "AndElimR", "hyp": "h0"} >= 0.85.
   - Critic scores {"rule": "Exact", "hyp": "h0"} <= 0.15 (invalid step).
2. State 2: h0: And(A, B), h1: B, h2: A ⊢ And(B, A)
   - Actor emits {"rule": "AndIntro", "left": "h1", "right": "h2"}.
   - Critic scores this introduction >= 0.90.
"""

import http.server
import json
import os
import socketserver
import subprocess
import sys
import threading
import time

def find_worker_filename(dist_assets_dir):
    for f in os.listdir(dist_assets_dir):
        if f.startswith("llm-worker-") and f.endswith(".js"):
            return f
    return None

def run_worker_dsl_test():
    dist_dir = os.path.abspath("ui/dist")
    assets_dir = os.path.join(dist_dir, "assets")

    if not os.path.exists(dist_dir) or not os.path.exists(assets_dir):
        print("Building UI bundle first...")
        subprocess.run(["npm", "run", "build"], cwd="ui", check=True)

    worker_file = find_worker_filename(assets_dir)
    if not worker_file:
        raise RuntimeError("Could not find compiled llm-worker chunk in ui/dist/assets/")

    print(f"[Setup] Found compiled worker chunk: {worker_file}")

    class QuietHandler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, format, *args):
            pass

    class ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    os.chdir(dist_dir)
    port = 8991
    httpd = ReusableTCPServer(("127.0.0.1", port), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"[Test Server] Serving ui/dist at http://127.0.0.1:{port}/")

    test_harness_html = f"""<!doctype html>
<html>
<head><title>Worker DSL Schema Validation Test</title></head>
<body>
<h1>Worker DSL Schema Validation Harness</h1>
<div id="status">Running tests...</div>
<pre id="logs"></pre>
<script type="module">
  const logsEl = document.getElementById('logs');
  const log = (msg) => {{
    console.log(msg);
    logsEl.innerText += msg + '\\n';
  }};

  async function runAllDslTests() {{
    try {{
      log('[Test 1] Instantiating Web Worker...');
      const workerUrl = new URL('./assets/{worker_file}', window.location.href);
      const worker = new Worker(workerUrl, {{ type: 'module' }});

      const send = (msg) => new Promise((resolve) => {{
        const handler = (e) => {{
          if (e.data.type === 'TACTIC_PROGRESS') return;
          worker.removeEventListener('message', handler);
          resolve(e.data);
        }};
        worker.addEventListener('message', handler);
        worker.postMessage(msg);
      }});

      // 1. Init Worker
      log('[Test 1.1] Initializing Gemma 4 WebGPU Worker...');
      const initRes = await send({{ type: 'INIT_LLM', modelId: 'gemma-4-2b-it-q4f16-webgpu' }});
      log('INIT_LLM:' + JSON.stringify(initRes));

      // 2. State 1: h0: And(Prop("A"), Prop("B")) |- And(Prop("B"), Prop("A"))
      log('[Test 2] State 1: Testing Actor & Critic on h0: And(A, B) |- And(B, A)...');
      const state1_hyps = {{
        h0: {{ And: [{{ Prop: 'A' }}, {{ Prop: 'B' }}] }}
      }};
      const state1_target = {{
        And: [{{ Prop: 'B' }}, {{ Prop: 'A' }}]
      }};

      const s1_actor = await send({{
        type: 'GENERATE_TACTIC',
        taskId: 's1-actor',
        hyps: state1_hyps,
        target: state1_target,
        thinkingBudget: 128
      }});
      log('S1_ACTOR_RES:' + JSON.stringify(s1_actor));

      if (!s1_actor.isValidAst || !s1_actor.stepAst) {{
        throw new Error('S1 Actor failed to produce a valid AST: ' + JSON.stringify(s1_actor));
      }}
      log('S1_ACTOR_AST_RULE:' + s1_actor.stepAst.rule);
      if (s1_actor.stepAst.rule !== 'AndElimR' && s1_actor.stepAst.rule !== 'AndElimL' && s1_actor.stepAst.rule !== 'AndIntro') {{
        throw new Error('S1 Actor rule unexpected: ' + s1_actor.stepAst.rule);
      }}

      // Critic valid step: AndElimR(h0)
      const s1_critic_valid = await send({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 's1-critic-valid',
        hyps: state1_hyps,
        target: state1_target,
        candidateStep: {{ rule: 'AndElimR', hyp: 'h0' }}
      }});
      log('S1_CRITIC_VALID_SCORE:' + s1_critic_valid.score);
      if (s1_critic_valid.score < 0.85) {{
        throw new Error('S1 Critic score for AndElimR(h0) must be >= 0.85, got: ' + s1_critic_valid.score);
      }}

      // Critic invalid step: Exact(h0) on mismatched target
      const s1_critic_invalid = await send({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 's1-critic-invalid',
        hyps: state1_hyps,
        target: state1_target,
        candidateStep: {{ rule: 'Exact', hyp: 'h0' }}
      }});
      log('S1_CRITIC_INVALID_SCORE:' + s1_critic_invalid.score);
      if (s1_critic_invalid.score > 0.15) {{
        throw new Error('S1 Critic score for invalid Exact(h0) must be <= 0.15, got: ' + s1_critic_invalid.score);
      }}

      // 3. State 2: h0: And(A, B), h1: B, h2: A |- And(B, A)
      log('[Test 3] State 2: Testing Actor & Critic on h0, h1: B, h2: A |- And(B, A)...');
      const state2_hyps = {{
        h0: {{ And: [{{ Prop: 'A' }}, {{ Prop: 'B' }}] }},
        h1: {{ Prop: 'B' }},
        h2: {{ Prop: 'A' }}
      }};
      const state2_target = {{
        And: [{{ Prop: 'B' }}, {{ Prop: 'A' }}]
      }};

      const s2_actor = await send({{
        type: 'GENERATE_TACTIC',
        taskId: 's2-actor',
        hyps: state2_hyps,
        target: state2_target,
        thinkingBudget: 128
      }});
      log('S2_ACTOR_RES:' + JSON.stringify(s2_actor));
      if (!s2_actor.isValidAst || !s2_actor.stepAst) {{
        throw new Error('S2 Actor failed to produce a valid AST: ' + JSON.stringify(s2_actor));
      }}
      log('S2_ACTOR_AST:' + JSON.stringify(s2_actor.stepAst));
      if (s2_actor.stepAst.rule !== 'AndIntro' || s2_actor.stepAst.left !== 'h1' || s2_actor.stepAst.right !== 'h2') {{
        throw new Error('S2 Actor expected AndIntro(h1, h2), got: ' + JSON.stringify(s2_actor.stepAst));
      }}

      // Critic evaluation for AndIntro(h1, h2)
      const s2_critic_intro = await send({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 's2-critic-intro',
        hyps: state2_hyps,
        target: state2_target,
        candidateStep: {{ rule: 'AndIntro', left: 'h1', right: 'h2' }}
      }});
      log('S2_CRITIC_INTRO_SCORE:' + s2_critic_intro.score);
      if (s2_critic_intro.score < 0.90) {{
        throw new Error('S2 Critic score for AndIntro(h1, h2) must be >= 0.90, got: ' + s2_critic_intro.score);
      }}

      log('STAGE2_DSL_ALL_TESTS_PASSED');
      document.getElementById('status').innerText = 'SUCCESS';
    }} catch (err) {{
      log('STAGE2_DSL_TEST_FAILED: ' + (err.message || String(err)));
      document.getElementById('status').innerText = 'FAILED: ' + err.message;
    }}
  }}

  runAllDslTests();
</script>
</body>
</html>"""

    test_html_path = os.path.join(dist_dir, "test_dsl_harness.html")
    with open(test_html_path, "w") as f:
        f.write(test_harness_html)

    chromium_cmd = [
        "/usr/bin/chromium",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--enable-logging=stderr",
        "--v=1",
        f"http://127.0.0.1:{port}/test_dsl_harness.html"
    ]

    print("[Browser Execution] Running headless Chromium harness for Worker DSL Schema Validation...")
    proc = subprocess.Popen(chromium_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    timeout = 30
    start = time.time()
    output_lines = []

    while time.time() - start < timeout:
        if proc.poll() is not None:
            break
        time.sleep(0.5)

    try:
        proc.terminate()
        stdout, stderr = proc.communicate(timeout=2)
        full_log = (stdout or "") + (stderr or "")
        output_lines.extend(full_log.splitlines())
    except Exception as e:
        print("Error reading process:", e)

    httpd.shutdown()

    if os.path.exists(test_html_path):
        os.remove(test_html_path)

    print("\n--- CHROMIUM WORKER DSL TEST OUTPUT ---")
    for line in output_lines:
        if any(tag in line for tag in ["INIT_LLM", "S1_", "S2_", "STAGE2_", "CONSOLE"]):
            print("  ", line)

    has_pass = any("STAGE2_DSL_ALL_TESTS_PASSED" in l for l in output_lines)
    if has_pass:
        print("\n✅ Stage 2: Worker In-Context DSL & Schema Validation PASSED!")
        return 0
    else:
        print("\n❌ Stage 2 DSL Test did not complete successfully.")
        return 1

if __name__ == "__main__":
    sys.exit(run_worker_dsl_test())
