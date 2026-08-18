#!/usr/bin/env python3
"""
Integration test for Gemma 4 Edge Dual-Mode Web Worker in Headless Chromium.

Validates:
1. Web Worker instantiation and INIT_LLM response.
2. Actor Mode autoregressive tactic generation with <think>...</think> reasoning trace.
3. Critic Mode (GenRM Verifier) next-token logprob scoring: S_GenRM = p(Yes) / [p(Yes) + p(No)].
4. VRAM memory safety (< 4,096 MB) and KV-cache sliding_window_size: -1 override.
"""

import http.server
import json
import os
import socketserver
import subprocess
import sys
import threading
import time

def run_integration_test():
    dist_dir = os.path.abspath("ui/dist")
    if not os.path.exists(dist_dir):
        print("ui/dist not found, building first...")
        subprocess.run(["npm", "run", "build"], cwd="ui", check=True)

    port = 8765
    os.chdir(dist_dir)

    class QuietHandler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, format, *args):
            pass

    httpd = socketserver.TCPServer(("127.0.0.1", port), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"[Test Server] Serving ui/dist at http://127.0.0.1:{port}/")

    # Create HTML test harness to exercise llm-worker.ts inside real browser environment
    test_harness_html = f"""<!doctype html>
<html>
<head><title>Gemma 4 Edge Worker Test</title></head>
<body>
<h1>Testing Gemma 4 Edge Dual-Mode Web Worker</h1>
<div id="results">Running...</div>
<script type="module">
  async function test() {{
    const results = [];
    const log = (msg) => {{
      console.log(msg);
      results.push(msg);
    }};

    try {{
      // Locate worker asset bundle
      const workerUrl = new URL('./assets/llm-worker-D-AlRclX.js', window.location.href);
      const worker = new Worker(workerUrl, {{ type: 'module' }});

      const send = (msg) => new Promise((resolve) => {{
        const handler = (e) => {{
          if (e.data.type === 'TACTIC_PROGRESS') return; // ignore progress in simple promise
          worker.removeEventListener('message', handler);
          resolve(e.data);
        }};
        worker.addEventListener('message', handler);
        worker.postMessage(msg);
      }});

      // 1. Test INIT_LLM
      log('[1] Initializing Gemma 4 WebGPU worker...');
      const initRes = await send({{ type: 'INIT_LLM', modelId: 'gemma-4-2b-it-q4f16-webgpu' }});
      log('INIT_RES:' + JSON.stringify(initRes));

      // Invariants check
      if (initRes.vramAllocatedMB > 4096) throw new Error('VRAM allocation exceeded 4 GB limit!');
      if (initRes.slidingWindowSize !== -1) throw new Error('sliding_window_size must be -1 override!');

      // 2. Test Actor Mode (Tactic Search + Thinking Budget)
      log('[2] Testing Actor Mode (generateTactic with thinking budget)...');
      const actorRes = await send({{
        type: 'GENERATE_TACTIC',
        taskId: 'test-actor-1',
        theoremName: 'Mathlib.Logic.And.intro',
        goalState: 'A -> B -> A ∧ B',
        hypotheses: ['(A : Prop)', '(B : Prop)', '(a : A)', '(b : B)'],
        thinkingBudget: 256
      }});
      log('ACTOR_RES:' + JSON.stringify(actorRes));

      if (!actorRes.tacticAst || actorRes.tacticAst.length === 0) throw new Error('Actor mode failed to synthesize tactic AST!');
      if (!actorRes.reasoningTrace || actorRes.reasoningTrace.length === 0) throw new Error('Actor mode missing reasoning trace!');

      // 3. Test Critic Mode (GenRM Verifier logprob scoring)
      log('[3] Testing Critic Mode (GenRM evaluation)...');
      const criticValidRes = await send({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'test-critic-valid',
        theoremName: 'Mathlib.Logic.And.intro',
        goalState: 'A -> B -> A ∧ B',
        candidateTactic: 'apply And.intro'
      }});
      log('CRITIC_VALID_RES:' + JSON.stringify(criticValidRes));

      if (criticValidRes.score < 0.7) throw new Error('GenRM score for valid tactic is unexpectedly low: ' + criticValidRes.score);

      const criticFlawedRes = await send({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'test-critic-flawed',
        theoremName: 'Mathlib.Logic.And.intro',
        goalState: 'A -> B -> A ∧ B',
        candidateTactic: 'sorry'
      }});
      log('CRITIC_FLAWED_RES:' + JSON.stringify(criticFlawedRes));

      if (criticFlawedRes.score > 0.1) throw new Error('GenRM score for sorry tactic should be near zero, got: ' + criticFlawedRes.score);

      // 4. Test Telemetry
      log('[4] Fetching Worker Telemetry...');
      const telemetryRes = await send({{ type: 'GET_TELEMETRY' }});
      log('TELEMETRY_RES:' + JSON.stringify(telemetryRes));

      log('ALL_TESTS_PASSED');
      document.getElementById('results').innerText = 'SUCCESS';
    }} catch (err) {{
      log('TEST_FAILED: ' + (err.message || String(err)));
      document.getElementById('results').innerText = 'FAILED: ' + err.message;
    }}
  }}
  test();
</script>
</body>
</html>"""

    test_html_path = os.path.join(dist_dir, "test_worker_harness.html")
    with open(test_html_path, "w") as f:
        f.write(test_harness_html)

    # Launch Headless Chromium
    chromium_cmd = [
        "/usr/bin/chromium",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--enable-logging=stderr",
        "--v=1",
        f"http://127.0.0.1:{port}/test_worker_harness.html"
    ]

    print("[Browser Execution] Running headless Chromium harness...")
    proc = subprocess.Popen(chromium_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    timeout = 15
    start = time.time()
    passed = False
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

    # Clean up test HTML
    if os.path.exists(test_html_path):
        os.remove(test_html_path)

    print("\n--- CHROMIUM WORKER TEST OUTPUT ---")
    for line in output_lines:
        if any(tag in line for tag in ["INIT_RES", "ACTOR_RES", "CRITIC_VALID_RES", "CRITIC_FLAWED_RES", "TELEMETRY_RES", "ALL_TESTS_PASSED", "TEST_FAILED", "CONSOLE"]):
            print("  ", line)

    has_pass = any("ALL_TESTS_PASSED" in l for l in output_lines)
    if has_pass:
        print("\n✅ Web Worker Gemma 4 Edge Dual-Mode Integration Test PASSED!")
        return 0
    else:
        print("\n❌ Web Worker Test did not complete successfully.")
        return 1

if __name__ == "__main__":
    sys.exit(run_integration_test())
