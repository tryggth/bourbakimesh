# BourbakiMesh Open Tickets Inventory
**Generated:** Mon Aug 17 10:15:51 PM UTC 2026
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

## Issue #18: epic(kernel): Phase 6 — Universal Multi-Target Extraction (Coq, Isabelle, Dedukti)
**URL:** https://github.com/tryggth/bourbakimesh/issues/18
**Labels:** epic, kernel

### Current Body
### Objective
Implement backend code emitters in `crates/bourbaki-kernel` targeting Coq (Gallina), Isabelle/HOL (Isar), and Dedukti, proving the semantic universality of the game-semantic arena IR.

---

### Universal Target Architecture
```
                         ┌─► Lean 4 (.lean) [Verified]
                         │
 Game-Semantic Strategy ─┼─► Coq / Gallina (.v)
      σ : P-Strategy     │
                         ├─► Isabelle / Isar (.thy)
                         │
                         └─► Dedukti (.dk)
```
- **Coq Emitter (`crates/bourbaki-kernel/src/emitters/coq.rs`):** Compile strategy trees to Gallina match/fixpoint terms.
- **Isabelle Emitter (`crates/bourbaki-kernel/src/emitters/isabelle.rs`):** Compile strategy trees to Isar proof scripts.
- **Dedukti Emitter (`crates/bourbaki-kernel/src/emitters/dedukti.rs`):** Compile strategy trees to $\lambda\Pi$-calculus modulo rewriting.

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. **Target Emitter Trait (`crates/bourbaki-kernel/src/emitter.rs`):**
   - Define `pub trait ProofEmitter { fn emit_strategy(&self, strategy: &StrategyTree) -> Result<String, EmissionError>; }`.
2. **Implement Backends:**
   - `CoqEmitter` (`crates/bourbaki-kernel/src/emitters/coq.rs`).
   - `IsabelleEmitter` (`crates/bourbaki-kernel/src/emitters/isabelle.rs`).
   - `DeduktiEmitter` (`crates/bourbaki-kernel/src/emitters/dedukti.rs`).
3. **Cross-Kernel Test Suite (`crates/bourbaki-kernel/tests/multi_target_tests.rs`):**
   - Lower canonical strategy trees (Identity, Modus Ponens, Negation Elimination) and verify syntax validity across all three formats.

#### 2. Verification Commands
- `cargo test -p bourbaki-kernel --test multi_target_tests`
- `cargo check --workspace`

#### 3. Commit
`feat(kernel): implement universal multi-target strategy emitters for Coq, Isabelle, and Dedukti (fixes #18)`
```


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

