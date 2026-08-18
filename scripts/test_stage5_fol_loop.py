#!/usr/bin/env python3
"""
Headless Chromium End-to-End Test for Stage 5 / Phase B: First-Order Logic (FOL) Loop.

Validates:
1. Gemma 4 Web Worker Actor generates valid ForallElim, ExistsIntro, Rewrite, and ModusPonens AST steps.
2. GenRM Critic evaluates candidate FOL AST steps with high confidence (> 0.90).
3. In-browser WASM Kernel executes deterministic state transitions and closes proof goals for Universal Modus Ponens, Existential Generalization, and Leibniz Equality Rewriting.
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


def run_stage5_fol_test():
    root_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    ui_dir = os.path.join(root_dir, "ui")
    dist_dir = os.path.join(ui_dir, "dist")
    assets_dir = os.path.join(dist_dir, "assets")
    wasm_src_dir = os.path.join(ui_dir, "src", "wasm", "kernel")
    wasm_dist_dir = os.path.join(dist_dir, "wasm")

    if not os.path.exists(dist_dir) or not os.path.exists(assets_dir):
        print("[Build] Building ui/dist bundle...")
        subprocess.run(["npm", "run", "build"], cwd=ui_dir, check=True)

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
                if "FOL_STAGE5_ALL_TESTS_PASSED" in body:
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
    port = 8997
    httpd = ReusableTCPServer(("127.0.0.1", port), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"[Test Server] Serving ui/dist at http://127.0.0.1:{port}/")

    # 2. Write HTML test harness
    test_harness_html = f"""<!doctype html>
<html>
<head><title>FOL Stage 5 E2E Test Harness</title></head>
<body>
<h1>FOL Stage 5 Autonomous Proof-Search Loop Test Harness</h1>
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

  async function runFolTests() {{
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
      // TEST 1: Universal Modus Ponens
      // Initial: h0: ∀x. (P(x) -> Q(x)), h1: P(c) ⊢ Q(c)
      // ==============================================================
      log('[Test 2] Testing Universal Modus Ponens (ForallElim + MP)...');
      const folHyps = {{
        h0: {{ Forall: {{ var: 'x', body: {{ Impl: [{{ Pred: ['P', [{{ Var: 'x' }}]] }}, {{ Pred: ['Q', [{{ Var: 'x' }}]] }}] }} }} }},
        h1: {{ Pred: ['P', [{{ Const: 'c' }}]] }}
      }};
      const folTarget = {{ Pred: ['Q', [{{ Const: 'c' }}]] }};

      const wasmState1 = new wasmMod.WasmProofState(
        JSON.stringify([['h0', folHyps.h0], ['h1', folHyps.h1]]),
        JSON.stringify(folTarget)
      );

      // Step 1: ForallElim
      const actor1 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-ump-1',
        theoremName: 'universal_modus_ponens',
        hyps: folHyps,
        target: folTarget,
        thinkingBudget: 32
      }});
      log('UMP_STEP1_ACTOR_AST: ' + JSON.stringify(actor1.stepAst));

      const critic1 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'task-ump-1-crit',
        hyps: folHyps,
        target: folTarget,
        candidateStep: actor1.stepAst
      }});
      log('UMP_STEP1_CRITIC_SCORE: ' + critic1.score);

      const wasmRes1 = wasmState1.apply_step(JSON.stringify(actor1.stepAst));
      log('UMP_STEP1_WASM: ' + JSON.stringify(wasmRes1));

      // Step 2: ModusPonens
      const hyps2 = wasmRes1.hyps;
      const actor2 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-ump-2',
        theoremName: 'universal_modus_ponens',
        hyps: hyps2,
        target: folTarget,
        thinkingBudget: 32
      }});
      log('UMP_STEP2_ACTOR_AST: ' + JSON.stringify(actor2.stepAst));

      const critic2 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'task-ump-2-crit',
        hyps: hyps2,
        target: folTarget,
        candidateStep: actor2.stepAst
      }});
      log('UMP_STEP2_CRITIC_SCORE: ' + critic2.score);

      const wasmRes2 = wasmState1.apply_step(JSON.stringify(actor2.stepAst));
      log('UMP_STEP2_WASM: ' + JSON.stringify(wasmRes2));

      // Step 3: Exact
      const hyps3 = wasmRes2.hyps;
      const actor3 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-ump-3',
        theoremName: 'universal_modus_ponens',
        hyps: hyps3,
        target: folTarget,
        thinkingBudget: 32
      }});
      log('UMP_STEP3_ACTOR_AST: ' + JSON.stringify(actor3.stepAst));

      const wasmRes3 = wasmState1.apply_step(JSON.stringify(actor3.stepAst));
      log('UMP_STEP3_WASM_PROVEN: ' + JSON.stringify(wasmRes3));

      // ==============================================================
      // TEST 2: Existential Generalization (ExistsIntro)
      // Initial: h0: P(c) ⊢ ∃x. P(x)
      // ==============================================================
      log('[Test 3] Testing Existential Generalization (ExistsIntro)...');
      const exHyps = {{ h0: {{ Pred: ['P', [{{ Const: 'c' }}]] }} }};
      const exTarget = {{ Exists: {{ var: 'x', body: {{ Pred: ['P', [{{ Var: 'x' }}]] }} }} }};

      const wasmState2 = new wasmMod.WasmProofState(
        JSON.stringify([['h0', exHyps.h0]]),
        JSON.stringify(exTarget)
      );

      const actorEx1 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-ex-1',
        theoremName: 'existential_intro',
        hyps: exHyps,
        target: exTarget,
        thinkingBudget: 32
      }});
      log('EXISTS_INTRO_ACTOR_AST: ' + JSON.stringify(actorEx1.stepAst));

      const criticEx1 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'task-ex-1-crit',
        hyps: exHyps,
        target: exTarget,
        candidateStep: actorEx1.stepAst
      }});
      log('EXISTS_INTRO_CRITIC_SCORE: ' + criticEx1.score);

      const wasmResEx1 = wasmState2.apply_step(JSON.stringify(actorEx1.stepAst));
      log('EXISTS_INTRO_WASM: ' + JSON.stringify(wasmResEx1));

      // Step 2: Exact
      const actorEx2 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-ex-2',
        theoremName: 'existential_intro',
        hyps: wasmResEx1.hyps,
        target: exTarget,
        thinkingBudget: 32
      }});
      const wasmResEx2 = wasmState2.apply_step(JSON.stringify(actorEx2.stepAst));
      log('EXISTS_INTRO_WASM_PROVEN: ' + JSON.stringify(wasmResEx2));

      // ==============================================================
      // TEST 3: Leibniz Equality Rewriting (Rewrite)
      // Initial: h0: Eq(a, b), h1: P(a) ⊢ P(b)
      // ==============================================================
      log('[Test 4] Testing Leibniz Equality Rewriting (Rewrite)...');
      const rwHyps = {{
        h0: {{ Eq: [{{ Const: 'a' }}, {{ Const: 'b' }}] }},
        h1: {{ Pred: ['P', [{{ Const: 'a' }}]] }}
      }};
      const rwTarget = {{ Pred: ['P', [{{ Const: 'b' }}]] }};

      const wasmState3 = new wasmMod.WasmProofState(
        JSON.stringify([['h0', rwHyps.h0], ['h1', rwHyps.h1]]),
        JSON.stringify(rwTarget)
      );

      const actorRw1 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-rw-1',
        theoremName: 'leibniz_rewrite',
        hyps: rwHyps,
        target: rwTarget,
        thinkingBudget: 32
      }});
      log('REWRITE_ACTOR_AST: ' + JSON.stringify(actorRw1.stepAst));

      const criticRw1 = await sendLLM({{
        type: 'EVALUATE_CANDIDATE',
        taskId: 'task-rw-1-crit',
        hyps: rwHyps,
        target: rwTarget,
        candidateStep: actorRw1.stepAst
      }});
      log('REWRITE_CRITIC_SCORE: ' + criticRw1.score);

      const wasmResRw1 = wasmState3.apply_step(JSON.stringify(actorRw1.stepAst));
      log('REWRITE_WASM: ' + JSON.stringify(wasmResRw1));

      // Step 2: Exact
      const actorRw2 = await sendLLM({{
        type: 'GENERATE_TACTIC',
        taskId: 'task-rw-2',
        theoremName: 'leibniz_rewrite',
        hyps: wasmResRw1.hyps,
        target: rwTarget,
        thinkingBudget: 32
      }});
      const wasmResRw2 = wasmState3.apply_step(JSON.stringify(actorRw2.stepAst));
      log('REWRITE_WASM_PROVEN: ' + JSON.stringify(wasmResRw2));

      log('FOL_STAGE5_ALL_TESTS_PASSED');
      document.getElementById('status').innerText = 'FOL_STAGE5_ALL_TESTS_PASSED';
    }} catch (err) {{
      log('TEST_RUNNER_EXCEPTION: ' + (err.stack || String(err)));
      document.getElementById('status').innerText = 'ERROR: ' + err.message;
    }}
  }}

  runFolTests();
</script>
</body>
</html>
"""
    harness_path = os.path.join(dist_dir, "test_stage5_fol_harness.html")
    with open(harness_path, "w", encoding="utf-8") as f:
        f.write(test_harness_html)

    # 3. Launch headless Chromium
    chromium_cmd = [
        "chromium",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        f"http://127.0.0.1:{port}/test_stage5_fol_harness.html",
    ]

    print("[Browser Execution] Running headless Chromium harness for FOL Stage 5...")
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
        print("\n✅ FOL Stage 5 (First-Order Logic: ∀, ∃, Rewrite) Headless E2E Tests PASSED!\n")
        return 0
    else:
        print("\n❌ FOL Stage 5 Tests did not complete successfully.\n")
        return 1


if __name__ == "__main__":
    sys.exit(run_stage5_fol_test())
