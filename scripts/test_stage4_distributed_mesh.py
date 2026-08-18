#!/usr/bin/env python3
"""
Automated Test Suite for Stage 4: Distributed Mesh Coordination & Global Proof DAG Synchronization.

Tests:
1. Spawns Rust mesh-coordinator server on ws://127.0.0.1:9001.
2. Connects edge worker in headless Chromium.
3. Injects AndComm goal (h0: And(A, B) ⊢ And(B, A)).
4. Edge worker autonomously leases tasks, computes step ASTs and GenRM scores, and submits back.
5. Coordinator validates transitions with kernel and advances global DAG.
6. Verifies global Proof DAG closes in Proven state across the network.
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

def run_stage4_test():
    dist_dir = os.path.abspath("ui/dist")
    assets_dir = os.path.join(dist_dir, "assets")

    if not os.path.exists(dist_dir) or not os.path.exists(assets_dir):
        print("Building UI bundle first...")
        subprocess.run(["npm", "run", "build"], cwd="ui", check=True)

    worker_file = find_worker_filename(assets_dir)
    if not worker_file:
        raise RuntimeError("Could not find compiled llm-worker chunk in ui/dist/assets/")

    # 1. Build and Launch Rust Mesh Coordinator
    print("[Rust Build] Building mesh-coordinator binary...")
    subprocess.run(["cargo", "build", "-p", "mesh-coordinator"], check=True)

    coordinator_bin = os.path.abspath("target/debug/mesh-coordinator")
    coordinator_port = 9001
    coordinator_addr = f"127.0.0.1:{coordinator_port}"

    print(f"[Coordinator] Starting Rust mesh coordinator on ws://{coordinator_addr}...")
    coordinator_proc = subprocess.Popen(
        [coordinator_bin, "--addr", coordinator_addr],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    time.sleep(1.0)
    if coordinator_proc.poll() is not None:
        out, err = coordinator_proc.communicate()
        raise RuntimeError(f"Coordinator failed to start:\nStdout: {out}\nStderr: {err}")

    # 2. Start HTTP server for Web UI test harness
    class QuietHandler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, format, *args):
            pass

    class ReusableTCPServer(socketserver.TCPServer):
        allow_reuse_address = True

    os.chdir(dist_dir)
    http_port = 8994
    httpd = ReusableTCPServer(("127.0.0.1", http_port), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"[Web Server] Serving ui/dist at http://127.0.0.1:{http_port}/")

    # 3. Write HTML test harness
    test_harness_html = f"""<!doctype html>
<html>
<head><title>Stage 4 Distributed Mesh Test Harness</title></head>
<body>
<h1>Stage 4 Distributed Mesh Coordination Test</h1>
<div id="status">Connecting to mesh coordinator...</div>
<pre id="logs"></pre>
<script type="module">
  const logsEl = document.getElementById('logs');
  const log = (msg) => {{
    console.log(msg);
    logsEl.innerText += msg + '\\n';
  }};

  async function runStage4MeshTest() {{
    try {{
      log('[Stage 4 Test 1] Initializing Gemma 4 Web Worker...');
      const workerUrl = new URL('./assets/{worker_file}', window.location.href);
      const llmWorker = new Worker(workerUrl, {{ type: 'module' }});

      const sendLLM = (msg) => new Promise((resolve) => {{
        const handler = (e) => {{
          if (e.data.type === 'TACTIC_PROGRESS') return;
          llmWorker.removeEventListener('message', handler);
          resolve(e.data);
        }};
        llmWorker.addEventListener('message', handler);
        llmWorker.postMessage(msg);
      }});

      await sendLLM({{ type: 'INIT_LLM', modelId: 'gemma-4-2b-it-q4f16-webgpu' }});
      log('LLM_WORKER_READY');

      log('[Stage 4 Test 2] Connecting WebSocket to ws://{coordinator_addr}...');
      const ws = new WebSocket('ws://{coordinator_addr}');

      await new Promise((resolve, reject) => {{
        ws.onopen = resolve;
        ws.onerror = reject;
        setTimeout(() => reject(new Error('WebSocket connection timeout')), 3000);
      }});
      log('WS_CONNECTED');

      let reqId = 0;
      const pending = new Map();
      ws.onmessage = (e) => {{
        try {{
          const msg = JSON.parse(e.data);
          if (msg.id && pending.has(msg.id)) {{
            const {{ resolve, reject }} = pending.get(msg.id);
            pending.delete(msg.id);
            if (msg.error) reject(new Error(msg.error.message));
            else resolve(msg.result);
          }}
          if (msg.method === 'mesh_dag_updated') {{
            log('BROADCAST_DAG_SYNC: ' + JSON.stringify(msg.params));
          }}
        }} catch(err) {{
          console.error(err);
        }}
      }};

      const callRpc = (method, params = {{}}) => new Promise((resolve, reject) => {{
        const id = 'req-' + (++reqId);
        pending.set(id, {{ resolve, reject }});
        ws.send(JSON.stringify({{ jsonrpc: '2.0', id, method, params }}));
      }});

      // 1. Register Worker
      const regRes = await callRpc('mesh_register_worker', {{
        worker_id: 'test-edge-worker-01',
        model: 'gemma-4-2b-it-q4f16-webgpu',
        vram_limit_mb: 4096,
        throughput_tok_s: 46.2
      }});
      log('WORKER_REGISTERED: ' + JSON.stringify(regRes));

      // 2. Post AndComm goal to global DAG
      log('[Stage 4 Test 3] Injecting AndComm goal: h0: And(A, B) |- And(B, A)...');
      const postRes = await callRpc('mesh_post_goal', {{
        theorem_name: 'Mathlib.Logic.And.comm',
        hyps: {{ h0: {{ And: [{{ Prop: 'A' }}, {{ Prop: 'B' }}] }} }},
        target: {{ And: [{{ Prop: 'B' }}, {{ Prop: 'A' }}] }}
      }});
      log('GOAL_POSTED: ' + JSON.stringify(postRes));
      const rootId = postRes.root_id;

      // 3. Autonomous worker loop over the network
      let stepCount = 0;
      let dagStatus = 'Open';

      while (stepCount < 6 && dagStatus !== 'Proven') {{
        const task = await callRpc('mesh_pull_task', {{ worker_id: 'test-edge-worker-01' }});
        if (!task) {{
          await new Promise((r) => setTimeout(r, 100));
          continue;
        }}

        stepCount++;
        log(`STEP ${{stepCount}} LEASED TASK: ${{task.task_id}} (Node: ${{task.node_id}})`);

        // Generate step using Gemma 4 Actor
        const actorRes = await sendLLM({{
          type: 'GENERATE_TACTIC',
          taskId: 'mesh-step-' + stepCount,
          hyps: task.hyps,
          target: task.target,
          thinkingBudget: 128
        }});
        log(`STEP ${{stepCount}} ACTOR AST: ` + JSON.stringify(actorRes.stepAst));

        // Evaluate GenRM Critic score
        const criticRes = await sendLLM({{
          type: 'EVALUATE_CANDIDATE',
          taskId: 'mesh-eval-' + stepCount,
          hyps: task.hyps,
          target: task.target,
          candidateStep: actorRes.stepAst
        }});
        log(`STEP ${{stepCount}} CRITIC SCORE: ` + criticRes.score);

        // Submit verified step to coordinator
        const submitRes = await callRpc('mesh_submit_result', {{
          task_id: task.task_id,
          worker_id: 'test-edge-worker-01',
          step_ast: actorRes.stepAst,
          genrm_score: criticRes.score,
          thinking_trace: actorRes.reasoningTrace
        }});
        log(`STEP ${{stepCount}} SUBMIT RESULT: ` + JSON.stringify(submitRes));

        if (submitRes && submitRes.status === 'Proven') {{
          dagStatus = 'Proven';
          log('DAG_STATUS_PROVEN');
          break;
        }}

        const dag = await callRpc('mesh_get_dag');
        const rootNode = dag.nodes[rootId];
        if (rootNode && rootNode.status === 'Proven') {{
          dagStatus = 'Proven';
          log('ROOT_NODE_PROVEN');
          break;
        }}
      }}

      // Final check of coordinator DAG
      const finalDag = await callRpc('mesh_get_dag');
      log('FINAL_COORDINATOR_DAG: ' + JSON.stringify(finalDag));

      const telemetry = await callRpc('mesh_get_telemetry');
      log('COORDINATOR_TELEMETRY: ' + JSON.stringify(telemetry));

      if (stepCount > 4) throw new Error('Proof required too many network steps: ' + stepCount);
      if (dagStatus !== 'Proven') throw new Error('Proof did not reach Proven status');
      if (telemetry.total_tasks_resolved < 2) throw new Error('Coordinator did not record task resolutions');

      log('STAGE4_DISTRIBUTED_MESH_ALL_TESTS_PASSED');
      document.getElementById('status').innerText = 'SUCCESS';
    }} catch(err) {{
      log('STAGE4_DISTRIBUTED_MESH_FAILED: ' + (err.message || String(err)));
      document.getElementById('status').innerText = 'FAILED: ' + err.message;
    }}
  }}

  runStage4MeshTest();
</script>
</body>
</html>"""

    test_html_path = os.path.join(dist_dir, "test_stage4_harness.html")
    with open(test_html_path, "w") as f:
        f.write(test_harness_html)

    # 4. Launch Headless Chromium
    chromium_cmd = [
        "/usr/bin/chromium",
        "--headless=new",
        "--disable-gpu",
        "--no-sandbox",
        "--enable-logging=stderr",
        "--v=1",
        f"http://127.0.0.1:{http_port}/test_stage4_harness.html"
    ]

    print("[Browser Execution] Running headless Chromium harness for Stage 4 Distributed Mesh...")
    proc = subprocess.Popen(chromium_cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    timeout = 45
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
    coordinator_proc.terminate()
    try:
        coordinator_proc.wait(timeout=2)
    except Exception:
        coordinator_proc.kill()

    if os.path.exists(test_html_path):
        os.remove(test_html_path)

    print("\n--- CHROMIUM STAGE 4 DISTRIBUTED MESH TEST OUTPUT ---")
    for line in output_lines:
        print("  ", line)

    has_pass = any("STAGE4_DISTRIBUTED_MESH_ALL_TESTS_PASSED" in l for l in output_lines)
    if has_pass:
        print("\n✅ Stage 4: Distributed Mesh Coordination & Global Proof DAG Synchronization PASSED!")
        return 0
    else:
        print("\n❌ Stage 4 Test did not complete successfully.")
        return 1

if __name__ == "__main__":
    sys.exit(run_stage4_test())
