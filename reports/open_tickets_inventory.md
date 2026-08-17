# BourbakiMesh Open Tickets Inventory
**Generated:** Mon Aug 17 10:49:43 PM UTC 2026
**Repository:** https://github.com/tryggth/bourbakimesh

## Issue #28: feat(ml): R&D — In-Browser WebGPU & WebAssembly Latent MCTS Inference Engine
**URL:** https://github.com/tryggth/bourbakimesh/issues/28
**Labels:** enhancement, ml, ui, rfc

### Current Body
### Objective
Investigate and prototype client-side browser execution of BourbakiMuZero ($h_\theta, g_\theta, f_\theta$) via WebGPU and WebAssembly SIMD to enable zero-install proof visualizers (Epic #17) and voluntary P2P browser worker nodes.

---

### Architecture & Exploration Tracks

1. **ONNX Runtime Web (`ort-web` + WebGPU):**
   - Export PyTorch models ($h_\theta, g_\theta, f_\theta$) to optimized `.onnx` format via `torch.onnx.export`.
   - Test WebGPU shader dispatch and buffer allocation in browser workers.
   - Evaluate TypeScript-driven Latent MCTS tree traversal.

2. **Pure Rust Wasm Kernel (`candle` / `burn` + `wgpu` / `wasm-simd`):**
   - Port forward inference loops into `crates/bourbaki-kernel` targeting `wasm32-unknown-unknown`.
   - Measure dispatch latency for sequential batch=1 dynamics unrolling ($g_\theta, f_\theta$) comparing WebGPU against compiled WebAssembly SIMD on CPU.

3. **Voluntary P2P Swarm Worker (WebRTC / libp2p WebSocket):**
   - Connect browser clients to `/bourbaki/1.0.0/tasks` over WebSockets.
   - Execute MCTS search locally in background Web Workers and gossip verified proof blocks to `/bourbaki/1.0.0/proofs`.

---

### Key Research Questions
- What is the per-step dispatch overhead of WebGPU buffer readbacks during sequential MCTS rollouts vs Wasm SIMD?
- Can $h_\theta$ (graph transformer) execute on WebGPU while $g_\theta, f_\theta$ run in Wasm SIMD for optimal latency?
- What are the packaging constraints and cold-load times for downloading ~12MB model weights into IndexedDB cache?

---

### Deliverables
- [ ] **RFC 0003:** `rfcs/0003-webgpu-browser-inference.md` documenting runtime benchmarks and architecture.
- [ ] **ONNX Export Script:** `scripts/export_onnx.py` converting `bourbaki_v1.pt` to ONNX computation graphs.
- [ ] **Browser Benchmark Harness:** Minimal Vite/TypeScript harness measuring WebGPU vs Wasm sims/sec on CPU/GPU.

---

## Issue #17: epic(ui): Phase 5 — Real-Time Proof DAG Visualizer & Interactive Web UI
**URL:** https://github.com/tryggth/bourbakimesh/issues/17
**Labels:** epic, ui

### Current Body
### Objective
Develop an interactive Web UI dashboard and WebGL/Canvas visualization engine to render polarized Hyland-Ong dialogue trees, live proof DAGs, and cluster worker performance metrics.

---

### Sub-Milestones & Architecture
- **Web App (`ui/`):** React / TypeScript / Tailwind / Three.js or React-Flow.
- **Dialogue Graph Viewer:** Render active Proponent/Opponent moves, view scoping stacks, and justification pointers.
- **Proof DAG Visualizer:** Render content-addressed `ProofBlock` nodes, parent hashes, and validation statuses.
- **Backend Bridge:** FastAPI WebSocket streaming live MCTS search steps and P2P ledger blocks to browser clients.

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. **UI Scaffold (`ui/package.json`):** Initialize Vite + React + TypeScript + Tailwind + ReactFlow.
2. **WebSocket Gateway (`src/bourbakimesh/api/ws.py`):** Broadcast live dialogue traces and proof DAG updates over WebSockets.
3. **Dialogue Arena Visualizer Component:** Interactive node-link graph with polarized coloring (P: Blue, O: Orange) and justification edge arcs.
4. **End-to-End Smoke Test:** Run backend + frontend in dev mode and verify socket telemetry.

#### 2. Verification Commands
- `cd ui && npm test && npm run build`
- `.venv/bin/pytest tests/test_smoke.py`

#### 3. Commit
`feat(ui): implement real-time proof DAG visualizer and dialogue arena web UI (fixes #17)`
```


---

