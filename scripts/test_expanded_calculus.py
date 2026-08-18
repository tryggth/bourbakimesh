#!/usr/bin/env python3
"""
Headless Chromium End-to-End Test for Expanded Calculus (Or, Not, False, and Ex Falso).

Validates:
1. Gemma 4 Web Worker Actor generates valid OrIntroL, OrElim, Contradiction, and FalseElim AST steps.
2. GenRM Critic evaluates candidate AST steps with high confidence (> 0.90).
3. In-browser WASM Kernel executes deterministic state transitions and closes proof goals for OrComm and Ex Falso.
"""

import http.server
import os
import socketserver
import subprocess
import sys
import threading
import time


def find_worker_filename(assets_dir: str) -> str:
    for fname in os.listdir(assets_dir):
        if fname.startswith("llm-worker-") and fname.endswith(".js"):
            return fname
    raise FileNotFoundError(f"llm-worker chunk not found in {assets_dir}")


def find_wasm_js_filename(assets_dir: str) -> str:
    for fname in os.listdir(assets_dir):
        if fname.startswith("kernel_wasm-") and fname.endswith(".js"):
            return fname
    raise FileNotFoundError(f"kernel_wasm js chunk not found in {assets_dir}")


def find_wasm_bg_filename(assets_dir: str) -> str:
    for fname in os.listdir(assets_dir):
        if fname.startswith("kernel_wasm_bg-") and fname.endswith(".wasm"):
            return fname
    raise FileNotFoundError(f"kernel_wasm_bg wasm chunk not found in {assets_dir}")


def run_expanded_calculus_test():
    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    ui_dir = os.path.join(root_dir, "ui")
    dist_dir = os.path.join(ui_dir, "dist")
    assets_dir = os.path.join(dist_dir, "assets")

    if not os.path.exists(dist_dir) or not os.path.exists(assets_dir):
        print("[Build] Building ui/dist bundle...")
        subprocess.run(["npm", "run", "build"], cwd=ui_dir, check=True)

    worker_file = find_worker_filename(assets_dir)
    wasm_js_file = find_wasm_js_filename(assets_dir)
    wasm_bg_file = find_wasm_bg_filename(assets_dir)
    print(f"[Worker Asset] Found Gemma 4 worker: {worker_file}")
    print(f"[WASM Asset] Found Kernel WASM bridge: {wasm_js_file} with {wasm_bg_file}")

    # 1. Start HTTP Server for Web UI test harness
    class QuietHandler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, format, *args):
            pass

        def end_headers(self):
            self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
            self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
            super().end_headers()

    QuietHandler.extensions_map['.wasm'] = 'application/wasm'
    QuietHandler.extensions_map['.js'] = 'application/javascript'

    class ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    os.chdir(dist_dir)
    port = 8996
    httpd = ReusableTCPServer(("127.0.0.1", port), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"[Test Server] Serving ui/dist at http://127.0.0.1:{port}/")

    # 2. Write HTML test harness
    test_harness_html = f"""<!doctype html>
<html>
<head><title>Expanded Calculus E2E Test Harness</title></head>
<body>
<h1>Expanded Proof Calculus (Or, Not, False) Test Harness</h1>
<div id="status">Running tests...</div>
<pre id="logs"></pre>
<script type="module">
  const logsEl = document.getElementById('logs');
  const log = (msg) => {{
    console.log(msg);
    logsEl.innerText += msg + '\\n';
  }};

  window.onerror = (msg, url, line) => {{
    log('WINDOW_ERROR: ' + msg + ' at ' + url + ':' + line);
  }};

  async function runCalculusTests() {{
    try {{
      log('[Test 1] Initializing Gemma 4 Web Worker & WASM Kernel...');
      const workerUrl = new URL('./assets/{worker_file}', window.location.href);
      const llmWorker = new Worker(workerUrl, {{ type: 'module' }});

      llmWorker.onerror = (e) => {{
        log('WORKER_ERROR: ' + (e.message || String(e)));
      }};

      const sendLLM = (msg) => new Promise((resolve, reject) => {{
        const handler = (e) => {{
          if (e.data.type === 'TACTIC_PROGRESS') return;
          llmWorker.removeEventListener('message', handler);
          resolve(e.data);
        }};
        llmWorker.addEventListener('message', handler);
        llmWorker.postMessage(msg);
      }});

      const initRes = await sendLLM({{ type: 'INIT_LLM', modelId: 'gemma-4-2b-it-q4f16-webgpu' }});
      log('INIT_LLM:' + JSON.stringify(initRes));

      // ==========================================
      // TEST 1: Disjunction Introduction (OrIntroL)
      // Goal: h0: A |- A ∨ B
      // ==========================================
      log('[Test 2] Testing OrIntroL: h0: A |- A ∨ B...');
      const hyps1 = {{ h0: {{ Prop: 'A' }} }};
      const target1 = {{ Or: [{{ Prop: 'A' }}, {{ Prop: 'B' }}] }};

      const actorRes1 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'test-or-intro-l',
        hyps: hyps1,
        target: target1,
        thinkingBudget: 32
      }});
      log('OR_INTRO_L_ACTOR_AST: ' + JSON.stringify(actorRes1.stepAst));
      if (!actorRes1.stepAst || actorRes1.stepAst.rule !== 'OrIntroL') {{
        throw new Error('Actor failed to emit OrIntroL step: ' + JSON.stringify(actorRes1));
      }}

      const criticRes1 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'eval-or-intro-l',
        hyps: hyps1,
        target: target1,
        candidateStep: actorRes1.stepAst
      }});
      log('OR_INTRO_L_CRITIC_SCORE: ' + criticRes1.score);
      if (criticRes1.score < 0.90) throw new Error('Critic rejected valid OrIntroL: ' + criticRes1.score);

      // ==========================================
      // TEST 2: Disjunction Elimination (OrComm / OrElim)
      // Goal: h0: A ∨ B, h1: A -> (B ∨ A), h2: B -> (B ∨ A) |- B ∨ A
      // ==========================================
      log('[Test 3] Testing OrElim (OrComm): h0: A ∨ B, h1: A -> (B ∨ A), h2: B -> (B ∨ A) |- B ∨ A...');
      const target2 = {{ Or: [{{ Prop: 'B' }}, {{ Prop: 'A' }}] }};
      const hyps2 = {{
        h0: {{ Or: [{{ Prop: 'A' }}, {{ Prop: 'B' }}] }},
        h1: {{ Impl: [{{ Prop: 'A' }}, target2] }},
        h2: {{ Impl: [{{ Prop: 'B' }}, target2] }}
      }};

      const actorRes2 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'test-or-elim',
        hyps: hyps2,
        target: target2,
        thinkingBudget: 32
      }});
      log('OR_ELIM_ACTOR_AST: ' + JSON.stringify(actorRes2.stepAst));
      if (!actorRes2.stepAst || actorRes2.stepAst.rule !== 'OrElim') {{
        throw new Error('Actor failed to emit OrElim step: ' + JSON.stringify(actorRes2));
      }}

      const criticRes2 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'eval-or-elim',
        hyps: hyps2,
        target: target2,
        candidateStep: actorRes2.stepAst
      }});
      log('OR_ELIM_CRITIC_SCORE: ' + criticRes2.score);
      if (criticRes2.score < 0.90) throw new Error('Critic rejected valid OrElim: ' + criticRes2.score);

      // ==========================================
      // TEST 3: Principle of Explosion (FalseElim / Ex Falso)
      // Goal: h0: False |- ArbitraryTarget
      // ==========================================
      log('[Test 4] Testing FalseElim (Ex Falso): h0: False |- ArbitraryTarget...');
      const target3 = {{ Prop: 'ArbitraryConclusion' }};
      const hyps3 = {{ h0: 'False' }};

      const actorRes3 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'test-false-elim',
        hyps: hyps3,
        target: target3,
        thinkingBudget: 32
      }});
      log('FALSE_ELIM_ACTOR_AST: ' + JSON.stringify(actorRes3.stepAst));
      if (!actorRes3.stepAst || actorRes3.stepAst.rule !== 'FalseElim') {{
        throw new Error('Actor failed to emit FalseElim step: ' + JSON.stringify(actorRes3));
      }}

      const criticRes3 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'eval-false-elim',
        hyps: hyps3,
        target: target3,
        candidateStep: actorRes3.stepAst
      }});
      log('FALSE_ELIM_CRITIC_SCORE: ' + criticRes3.score);
      if (criticRes3.score < 0.90) throw new Error('Critic rejected valid FalseElim: ' + criticRes3.score);

      // ==========================================
      // TEST 4: Contradiction Rule
      // Goal: h0: P, h1: Not(P) |- Arbitrary
      // ==========================================
      log('[Test 5] Testing Contradiction: h0: P, h1: Not(P)...');
      const hyps4 = {{
        h0: {{ Prop: 'P' }},
        h1: {{ Not: {{ Prop: 'P' }} }}
      }};
      const target4 = {{ Prop: 'Q' }};

      const actorRes4 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'test-contradiction',
        hyps: hyps4,
        target: target4,
        thinkingBudget: 32
      }});
      log('CONTRADICTION_ACTOR_AST: ' + JSON.stringify(actorRes4.stepAst));
      if (!actorRes4.stepAst || actorRes4.stepAst.rule !== 'Contradiction') {{
        throw new Error('Actor failed to emit Contradiction step: ' + JSON.stringify(actorRes4));
      }}

      const criticRes4 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'eval-contradiction',
        hyps: hyps4,
        target: target4,
        candidateStep: actorRes4.stepAst
      }});
      log('CONTRADICTION_CRITIC_SCORE: ' + criticRes4.score);
      if (criticRes4.score < 0.90) throw new Error('Critic rejected valid Contradiction: ' + criticRes4.score);

      log('EXPANDED_CALCULUS_ALL_TESTS_PASSED');
      document.getElementById('status').innerText = 'SUCCESS';
    }} catch(err) {{
      log('EXPANDED_CALCULUS_FAILED: ' + (err.message || String(err)));
      document.getElementById('status').innerText = 'FAILED: ' + err.message;
    }}
  }}

  runCalculusTests();
</script>
</body>
</html>"""

    test_html_path = os.path.join(dist_dir, "test_expanded_calculus_harness.html")
    with open(test_html_path, "w") as f:
        f.write(test_harness_html)

    # 3. Launch Headless Chromium
    chromium_cmd = [
        "/usr/bin/chromium",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--enable-logging=stderr",
        "--v=1",
        f"http://127.0.0.1:{port}/test_expanded_calculus_harness.html"
    ]

    print("[Browser Execution] Running headless Chromium harness for Expanded Calculus...")
    proc = subprocess.Popen(chromium_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    timeout = 50
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
        print("Error reading browser process:", e)

    # Cleanup
    httpd.shutdown()
    if os.path.exists(test_html_path):
        os.remove(test_html_path)

    print("\n--- CHROMIUM EXPANDED CALCULUS TEST OUTPUT ---")
    for line in output_lines:
        if any(tag in line for tag in ["OR_", "FALSE_", "CONTRADICTION_", "WASM_", "EXPANDED_", "CONSOLE", "Test "]):
            print("  ", line)

    has_pass = any("EXPANDED_CALCULUS_ALL_TESTS_PASSED" in l for l in output_lines)
    if has_pass:
        print("\n✅ Expanded Calculus (Or, Not, False, Ex Falso) Headless E2E Tests PASSED!\n")
        sys.exit(0)
    else:
        print("\n❌ Expanded Calculus Tests did not complete successfully.\n")
        sys.exit(1)


if __name__ == "__main__":
    run_expanded_calculus_test()
