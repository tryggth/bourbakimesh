# BourbakiMesh Open Tickets Inventory
**Generated:** Mon Aug 17 05:52:38 PM UTC 2026
**Repository:** https://github.com/tryggth/bourbakimesh

## Issue #23: test(bench): Scaled Self-Play Tournament & Elo Evaluation Harness
**URL:** https://github.com/tryggth/bourbakimesh/issues/23
**Labels:** ml, benchmarks

### Current Body
### Objective
Construct an automated head-to-head tournament evaluation harness to track empirical Bayesian Elo ratings, theorem solve rates, and search efficiency gains across model generations (`bourbaki_v0.pt` vs `bourbaki_v1.pt` vs `v2`).

---

### Tournament Specifications
- **Engine (`src/bourbakimesh/benchmarks/tournament.py`):** Head-to-head pairing over standard 50-theorem proposition datasets with randomized Proponent/Opponent polarities.
- **Metrics Tracked:**
  - Bayesian Elo rating and 95% confidence intervals.
  - Tier-by-tier solve rate (Foundations, Implications, Algebraic).
  - Compute Simulation Equivalent (CSE) score.
  - Average proof length (ply count) and extraction latency.
- **Reporting:** Automated Markdown summary table and JSON export saved to `reports/`.

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. **Tournament Engine (`src/bourbakimesh/benchmarks/tournament.py`):**
   - Implement `ModelTournament` running paired games between model checkpoints on standard proposition suites.
2. **Bayesian Elo Tracker (`src/bourbakimesh/benchmarks/elo.py`):**
   - Compute maximum likelihood Elo ratings from tournament game win/loss/draw matrices.
3. **Tournament CLI (`src/bourbakimesh/benchmarks/tournament_cli.py`):**
   - CLI accepting `--models checkpoints/bourbaki_v0.pt checkpoints/bourbaki_v1.pt --simulations 100 --output reports/tournament_v0_v1.json`.
4. **Integration Test (`tests/test_tournament.py`):**
   - Verify paired games, score tabulation, and Elo calculation.

#### 2. Verification Commands
- `.venv/bin/pytest tests/test_tournament.py`
- `.venv/bin/python -m bourbakimesh.benchmarks.tournament_cli --models checkpoints/bourbaki_v0.pt checkpoints/bourbaki_v1.pt --output reports/tournament_v0_v1.json`

#### 3. Commit
`test(bench): implement automated head-to-head tournament and Bayesian Elo evaluator (fixes #23)`
```


---

## Issue #22: feat(mesh): Implement standalone bourbaki-daemon with embedded MCTS worker
**URL:** https://github.com/tryggth/bourbakimesh/issues/22
**Labels:** mesh, rust, p2p

### Current Body
### Objective
Build a standalone binary CLI (`crates/bourbaki-mesh/src/bin/bourbaki-daemon.rs`) that joins the libp2p GossipSub network, claims open Mathlib subgoals, and evaluates them using an embedded or IPC-bridged `bourbaki_v1.pt` MCTS search worker.

---

### Component Specifications
- **Binary:** `bourbaki-daemon`
- **Arguments:** `--peer-port <PORT>`, `--model-path <PATH>`, `--simulations <N>`, `--max-tasks <N>`, `--bootstrap-nodes <MULTIADDR>`
- **Gossip Subscriptions:**
  - Topic `/bourbaki/1.0.0/tasks`: Ingest open goal obligations.
  - Topic `/bourbaki/1.0.0/proofs`: Broadcast verified `ProofBlock` records.
- **Worker Execution Loop:** Task Ingestion $\to$ MCTS Search $\to$ Strategy Extractor $\to$ Zero-Trust Lean Kernel Check $\to$ Proof Attestation Engine $\to$ Gossip Broadcast.

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. **Daemon Binary (`crates/bourbaki-mesh/src/bin/bourbaki-daemon.rs`):**
   - Implement CLI argument parsing with `clap`.
   - Initialize `P2PNode`, `ProofLedger`, and `ProofAttestationEngine`.
2. **Worker Bridge Orchestrator (`crates/bourbaki-mesh/src/worker.rs`):**
   - Spawn Python MCTS inference worker or connect via Unix Domain Socket IPC.
   - Run task claim, proof generation, and verification loop.
3. **Integration Test (`crates/bourbaki-mesh/tests/daemon_integration_tests.rs`):**
   - Launch two daemon nodes on loopback; Node 1 emits task, Node 2 solves and returns verified proof block.

#### 2. Verification Commands
- `cargo test -p bourbaki-mesh --test daemon_integration_tests`
- `cargo check --workspace`

#### 3. Commit
`feat(mesh): implement standalone bourbaki-daemon and P2P proof search worker (fixes #22)`
```


---

## Issue #21: feat(infra): Multi-Tier Model Registry & Release Asset Distribution Pipeline
**URL:** https://github.com/tryggth/bourbakimesh/issues/21
**Labels:** ml, mesh, infrastructure

### Current Body
### Objective
Establish automated distribution, versioning, and download pipelines for trained BourbakiMuZero neural checkpoints across GitHub Releases, Hugging Face Hub, and decentralized P2P chunks.

---

### Distribution Architecture
1. **GitHub Releases:** Release `v0.1.0-alpha` published with `bourbaki_v0.pt`, `bourbaki_v1.pt`, and `CHECKSUMS.txt`.
2. **Hugging Face Hub:** Auto-sync pipeline for model cards, configs, and safetensors (`tryggth/bourbakimesh-muzero`).
3. **P2P Chunk Distribution:** Splitting weights into 1MB SHA-256 content-addressed chunks in `crates/bourbaki-mesh/src/chunks.rs`.
4. **Unified Pull CLI:** `python -m bourbakimesh.models.pull --checkpoint bourbaki_v1` with fallback hierarchy: Local Cache $\to$ P2P Mesh $\to$ GitHub Release $\to$ Hugging Face.

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. **Model Pull CLI (`src/bourbakimesh/models/pull.py`):**
   - Implement automated checkpoint downloader fetching assets from GitHub Releases or Hugging Face if missing locally.
2. **Hugging Face Sync Script (`scripts/sync_huggingface.py`):**
   - Convert `.pt` state dicts to `.safetensors`, generate dynamic model card with CSE benchmarks, and push to HF Hub.
3. **P2P Weight Chunker (`crates/bourbaki-mesh/src/chunks.rs`):**
   - Implement `ModelChunker` creating 1MB content-addressed chunk DAG with Merkle root verification.
4. **Integration Test (`tests/test_model_registry.py`):**
   - Verify Merkle chunk assembly and CLI fallback downloads.

#### 2. Verification Commands
- `.venv/bin/pytest tests/test_model_registry.py`
- `cargo test -p bourbaki-mesh`

#### 3. Commit
`feat(infra): implement multi-tier model pull CLI, HF sync, and P2P weight chunking (fixes #21)`
```


---

## Issue #20: feat(ml): R&D — Neural and Game-Semantic Hinting Mechanisms for BourbakiMuZero
**URL:** https://github.com/tryggth/bourbakimesh/issues/20
**Labels:** enhancement, ml, rfc

### Current Body
### Objective
Design and implement a pluggable hinting architecture allowing external mathematical heuristics, SMT solvers, and asynchronous LLM teachers (e.g. Qwen-2.5-Math) to bias Latent MCTS search without violating zero-trust Lean 4 kernel soundness.

---

### Architectural Invariants
1. **Search Guidance Only:** Hints modify action priors ($\pi_{\text{hint}}$), state transitions, or initial arena graphs, but all extracted CIC terms $\mathcal{E}(\sigma)$ must pass unmodified zero-trust validation in Lean 4.
2. **Four Hint Modalities:**
   - **Policy Prior Warping:** Injecting external move priors into root MCTS: $\pi_{\text{root}}(a) \propto \pi_\theta(a)^{1-\lambda} \cdot \pi_{\text{hint}}(a)^\lambda$.
   - **Arena Cut Injection:** Adding auxiliary Proponent lemma hypotheses $\phi \implies \psi$ with typed justification pointers to the game graph.
   - **Conditioned Latent Dynamics:** Conditioning $g_\theta(s, a, h_{\text{hint}})$ on a dense mathematical intent vector.
   - **SMT Refutation Clues:** Using first-order tableau refutation branches to prune Opponent search subtrees.

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. **Design RFC-0002 (`rfcs/0002-neural-game-semantic-hinting.md`):**
   - Formalize mathematical formulations for prior warping and arena cut injections.
2. **Policy Prior Warper (`src/bourbakimesh/hints/policy.py`):**
   - Implement `PolicyWarper` with temperature scaling, Dirichlet blending, and support for external action distributions.
3. **MCTS Integration (`src/bourbakimesh/latent_mcts.py`):**
   - Add optional `hint_warper: Optional[PolicyWarper]` to `LatentMCTS.search()`.
4. **Integration Test Suite (`tests/test_hinting.py`):**
   - Test that injected valid lemma hints accelerate search convergence to target theorems.
   - Test that injecting adversarial/false hints fails gracefully without causing invalid proof extraction.

#### 2. Verification Commands
- `.venv/bin/pytest tests/test_hinting.py`
- `.venv/bin/pytest tests/`
- `cargo test --workspace`

#### 3. Commit
`feat(ml): implement neural policy prior warping and hinting engine (fixes #20)`
```


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

## Issue #16: epic(p2p): Phase 4 — Decentralized P2P Mesh Network & Byzantine-Resilient Ledger
**URL:** https://github.com/tryggth/bourbakimesh/issues/16
**Labels:** epic, mesh

### Current Body
### Objective
Complete Phase 4 macro epic: establish decentralized P2P gossipsub network, Kademlia DHT routing, Byzantine proof validation consensus, and edge node proof aggregation.

---

### Sub-Milestones & Status
- [x] Libp2p GossipSub & Kademlia DHT integration (`crates/bourbaki-mesh/src/p2p.rs`).
- [x] Byzantine-resilient proof attestation gate (`crates/bourbaki-mesh/src/consensus.rs`).
- [x] P2P swarm integration test suite (15 clean tests).
- [ ] Standalone daemon binary (`bourbaki-daemon` — Issue #22).
- [ ] Edge telemetry stream (`/bourbaki/1.0.0/telemetry`).

---

### Embedded `agy` Execution Blueprint

```markdown
#### 1. Implementation Tasks
1. Finalize and verify all sub-tickets under Epic #16 (#22 daemon, telemetry streams).
2. Execute multi-node loopback cluster benchmarks measuring proof propagation latency under 10+ simulated peers.
3. Update `STATUS_REPORT.md` and monorepo verification matrix.

#### 2. Verification Commands
- `cargo test -p bourbaki-mesh --test p2p_swarm_tests`
- `cargo test --workspace`

#### 3. Commit
`feat(mesh): complete Phase 4 decentralized P2P mesh network and ledger (fixes #16)`
```


---

