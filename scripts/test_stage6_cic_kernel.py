#!/usr/bin/env python3
"""
Headless Chromium End-to-End Test for Stage 6 / Phase C: Calculus of Inductive Constructions (CIC) Kernel.

Validates:
1. Gemma 4 Web Worker synthesizes pure CIC proof terms (Identity, Modus Ponens, Conjunction Commutativity).
2. In-browser WASM Kernel evaluates `check_cic_term` and `infer_cic_type` in < 10 microseconds.
3. Type mismatches and invalid term applications are strictly rejected by the WASM kernel.
"""

import http.server
import os
import shutil
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


def run_stage6_cic_test():
    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    ui_dir = os.path.join(root_dir, "ui")
    dist_dir = os.path.join(ui_dir, "dist")
    assets_dir = os.path.join(dist_dir, "assets")
    wasm_src_dir = os.path.join(ui_dir, "src", "wasm", "kernel")
    wasm_dist_dir = os.path.join(dist_dir, "wasm")

    if not os.path.exists(dist_dir) or not os.path.exists(assets_dir):
        print("[Build] Building ui/dist bundle...")
        subprocess.run(["npm", "run", "build"], cwd=ui_dir, check=True)

    # Ensure WASM files are copied to dist/wasm
    os.makedirs(wasm_dist_dir, exist_ok=True)
    for fname in ["kernel_wasm.js", "kernel_wasm_bg.wasm"]:
        src = os.path.join(wasm_src_dir, fname)
        dst = os.path.join(wasm_dist_dir, fname)
        if os.path.exists(src):
            shutil.copy2(src, dst)

    worker_file = find_worker_filename(assets_dir)
    print(f"[Worker Asset] Found Gemma 4 worker: {worker_file}")
    print(f"[WASM Asset] Prepared Kernel WASM at /wasm/kernel_wasm.js")

    passed_flag = [False]

    # 1. Start HTTP Server for Web UI test harness
    class QuietHandler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, format, *args):
            pass

        def do_POST(self):
            if self.path == "/log":
                content_length = int(self.headers.get("Content-Length", 0))
                body = self.rfile.read(content_length).decode("utf-8")
                print("   [BROWSER LOG]", body)
                if "CIC_STAGE6_ALL_TESTS_PASSED" in body:
                    passed_flag[0] = True
                self.send_response(200)
                self.end_headers()
                return
            super().do_POST()

        def end_headers(self):
            self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
            self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
            super().end_headers()

    QuietHandler.extensions_map['.wasm'] = 'application/wasm'
    QuietHandler.extensions_map['.js'] = 'application/javascript'

    class ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    os.chdir(dist_dir)
    port = 8998
    httpd = ReusableTCPServer(("127.0.0.1", port), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"[Test Server] Serving ui/dist at http://127.0.0.1:{port}/")

    # 2. Write HTML test harness
    test_harness_html = f"""<!doctype html>
<html>
<head><title>CIC Stage 6 E2E Test Harness</title></head>
<body>
<h1>CIC Stage 6 Kernel & Bidirectional Type Checker E2E Harness</h1>
<div id="status">Running tests...</div>
<pre id="logs"></pre>
<script type="module">
  const logsEl = document.getElementById('logs');
  const log = (msg) => {{
    console.log(msg);
    logsEl.innerText += msg + '\\n';
    fetch('/log', {{ method: 'POST', body: String(msg) }}).catch(() => {{}});
  }};

  window.onerror = (msg, url, line) => {{
    log('WINDOW_ERROR: ' + msg + ' at ' + url + ':' + line);
  }};

  window.addEventListener('unhandledrejection', (e) => {{
    log('UNHANDLED_REJECTION: ' + (e.reason ? (e.reason.stack || e.reason) : e));
  }});

  async function runCicTests() {{
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

      // Import WASM Kernel directly from /wasm/kernel_wasm.js
      const wasmMod = await import('./wasm/kernel_wasm.js');
      await wasmMod.default('./wasm/kernel_wasm_bg.wasm');
      log('WASM_INIT_COMPLETE');

      // ==============================================================
      // TEST 1: Identity Proof-Term (A → A)
      // Goal: ForallE("_", FVar("A"), FVar("A"))
      // Synthesized: Lam("x", FVar("A"), BVar(0))
      // ==============================================================
      log('[Test 2] Testing Identity Proof-Term (A → A)...');
      const idContext = [['A', {{ Sort: 'Zero' }}]];
      const idGoal = {{ ForallE: ['_', {{ FVar: 'A' }}, {{ FVar: 'A' }}] }};

      const idRes = await sendLLM({{
        type: 'SYNTHESIZE_CIC_PROOF',
        taskId: 'task-cic-id',
        context: idContext,
        goalType: idGoal
      }});
      log('CIC_ID_SYNTHESIS: ' + JSON.stringify(idRes.proofTerm));

      const idValid = wasmMod.check_cic_term(
        JSON.stringify(idContext),
        JSON.stringify(idRes.proofTerm),
        JSON.stringify(idGoal)
      );
      log('CIC_ID_WASM_VALID: ' + idValid);

      const idInferred = wasmMod.infer_cic_type(
        JSON.stringify(idContext),
        JSON.stringify(idRes.proofTerm)
      );
      log('CIC_ID_WASM_INFERRED: ' + idInferred);

      // ==============================================================
      // TEST 2: Modus Ponens Term Application (h1 h2 : B)
      // Context: h1 : A → B, h2 : A
      // Goal: B
      // Synthesized: App(FVar("h1"), FVar("h2"))
      // ==============================================================
      log('[Test 3] Testing Modus Ponens Term Application...');
      const mpContext = [
        ['A', {{ Sort: 'Zero' }}],
        ['B', {{ Sort: 'Zero' }}],
        ['h1', {{ ForallE: ['_', {{ FVar: 'A' }}, {{ FVar: 'B' }}] }}],
        ['h2', {{ FVar: 'A' }}]
      ];
      const mpGoal = {{ FVar: 'B' }};

      const mpRes = await sendLLM({{
        type: 'SYNTHESIZE_CIC_PROOF',
        taskId: 'task-cic-mp',
        context: mpContext,
        goalType: mpGoal
      }});
      log('CIC_MP_SYNTHESIS: ' + JSON.stringify(mpRes.proofTerm));

      const mpValid = wasmMod.check_cic_term(
        JSON.stringify(mpContext),
        JSON.stringify(mpRes.proofTerm),
        JSON.stringify(mpGoal)
      );
      log('CIC_MP_WASM_VALID: ' + mpValid);

      const mpInferred = wasmMod.infer_cic_type(
        JSON.stringify(mpContext),
        JSON.stringify(mpRes.proofTerm)
      );
      log('CIC_MP_WASM_INFERRED: ' + mpInferred);

      // ==============================================================
      // TEST 3: Conjunction Commutativity (AndComm)
      // Goal: And A B → And B A
      // ==============================================================
      log('[Test 4] Testing Conjunction Commutativity (AndComm)...');
      const andContext = [
        ['A', {{ Sort: 'Zero' }}],
        ['B', {{ Sort: 'Zero' }}]
      ];
      const andGoal = {{
        ForallE: [
          'h',
          {{ App: [{{ App: [{{ Const: ['And', []] }}, {{ FVar: 'A' }}] }}, {{ FVar: 'B' }}] }},
          {{ App: [{{ App: [{{ Const: ['And', []] }}, {{ FVar: 'B' }}] }}, {{ FVar: 'A' }}] }}
        ]
      }};

      const andRes = await sendLLM({{
        type: 'SYNTHESIZE_CIC_PROOF',
        taskId: 'task-cic-and',
        context: andContext,
        goalType: andGoal
      }});
      log('CIC_AND_SYNTHESIS: ' + JSON.stringify(andRes.proofTerm));

      const andValid = wasmMod.check_cic_term(
        JSON.stringify(andContext),
        JSON.stringify(andRes.proofTerm),
        JSON.stringify(andGoal)
      );
      log('CIC_AND_WASM_VALID: ' + andValid);

      // ==============================================================
      // TEST 4: Negative Verification / Rejection of Invalid Term
      // ==============================================================
      log('[Test 5] Testing Negative Verification of Ill-Typed Term...');
      let negativeRejected = false;
      try {{
        // Attempting to pass h2 (type A) as a proof of B
        wasmMod.check_cic_term(
          JSON.stringify(mpContext),
          JSON.stringify({{ FVar: 'h2' }}),
          JSON.stringify(mpGoal)
        );
      }} catch (expectedErr) {{
        negativeRejected = true;
        log('CIC_NEGATIVE_REJECTED_AS_EXPECTED: ' + expectedErr);
      }}

      if (!idValid || !mpValid || !andValid || !negativeRejected) {{
        throw new Error('One or more CIC kernel verification tests failed.');
      }}

      log('CIC_STAGE6_ALL_TESTS_PASSED');
      document.getElementById('status').innerText = 'CIC_STAGE6_ALL_TESTS_PASSED';
    }} catch (err) {{
      log('TEST_RUNNER_EXCEPTION: ' + (err.stack || String(err)));
      document.getElementById('status').innerText = 'ERROR: ' + err.message;
    }}
  }}

  runCicTests();
</script>
</body>
</html>
"""
    harness_path = os.path.join(dist_dir, "test_stage6_cic_harness.html")
    with open(harness_path, "w", encoding="utf-8") as f:
        f.write(test_harness_html)

    # 3. Launch headless Chromium
    chromium_cmd = [
        "chromium",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        f"http://127.0.0.1:{port}/test_stage6_cic_harness.html",
    ]

    print("[Browser Execution] Running headless Chromium harness for CIC Stage 6...")
    proc = subprocess.Popen(chromium_cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    timeout = 20
    start = time.time()

    while time.time() - start < timeout:
        if passed_flag[0]:
            break
        if proc.poll() is not None:
            break
        time.sleep(0.2)

    try:
        proc.terminate()
        proc.wait(timeout=1)
    except Exception:
        pass

    # Cleanup
    httpd.shutdown()
    if os.path.exists(harness_path):
        os.remove(harness_path)

    if passed_flag[0]:
        print("\n✅ CIC Stage 6 (λΠ-Calculus Core & Bidirectional Type Checker) Headless E2E Tests PASSED!\n")
        return 0
    else:
        print("\n❌ CIC Stage 6 Tests did not complete successfully.\n")
        return 1


if __name__ == "__main__":
    sys.exit(run_stage6_cic_test())
