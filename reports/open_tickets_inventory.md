# BourbakiMesh Open Tickets Inventory
**Generated:** Mon Aug 17 05:48:24 PM UTC 2026
**Repository:** https://github.com/tryggth/bourbakimesh

## Issue #23: test(bench): Scaled Self-Play Tournament & Elo Evaluation Harness
**URL:** https://github.com/tryggth/bourbakimesh/issues/23
**Labels:** ml, benchmarks

### Current Body
### Objective
Construct an automated head-to-head tournament evaluation harness to track empirical Elo ratings, theorem solve rates, and search efficiency gains across model generations (v0 vs v1 vs v2).

### Sub-Tasks & Deliverables
- [ ] **Head-to-Head Tournament Engine** (`src/bourbakimesh/benchmarks/tournament.py`): Pair checkpoints across standard Mathlib proposition sets with alternating Proponent/Opponent polarities.
- [ ] **Bayesian Elo Tracker**: Calculate posterior Elo ratings and confidence intervals for all promoted checkpoints.
- [ ] **Curriculum Solve-Rate Benchmark Suite**: Standardize 100-proposition held-out benchmark dataset across Tiers 1-3.
- [ ] **Automated Performance Dashboard**: Output markdown comparison matrices and latency graphs to reports/.

### Metrics to Track
- Solve rate by theorem difficulty tier (T1/T2/T3)
- MCTS sims/sec throughput per checkpoint
- Mean proof length (dialogue moves) per solved theorem
- Compute Simulation Equivalent (CSE) score per model generation

---

## Issue #22: feat(mesh): Implement standalone bourbaki-daemon with embedded MCTS worker
**URL:** https://github.com/tryggth/bourbakimesh/issues/22
**Labels:** mesh, rust, p2p

### Current Body
### Objective
Build a standalone binary CLI (`crates/bourbaki-mesh/src/bin/bourbaki-daemon.rs`) that joins the libp2p GossipSub network, claims open Mathlib subgoals, and evaluates them using embedded `bourbaki_v1.pt` MCTS search.

### Sub-Tasks & Deliverables
- [ ] **Daemon CLI Binary (`crates/bourbaki-mesh/src/bin/bourbaki-daemon.rs`):** Accept `--peer-port`, `--model-path`, `--simulations`, and `--max-tasks`.
- [ ] **Bidirectional Python FFI / IPC Worker:** Bridge Tokio P2P event loop with Python `BourbakiMuZero` engine via the existing `MeshIpcServer` TCP protocol.
- [ ] **Task Claim & Gossip Submission Loop:** Subscribe to `/bourbaki/1.0.0/tasks`, claim unproven subgoals, run Latent MCTS search, extract verified CIC proof blocks via `StrategyExtractor`, and gossip attested blocks to `/bourbaki/1.0.0/proofs`.
- [ ] **Byzantine Attestation Integration:** Apply `ProofAttestationEngine` validation before broadcasting candidate proofs.
- [ ] **Systemd / Docker Packaging:** Containerize daemon for headless edge server deployment.

### Dependencies
- `crates/bourbaki-mesh/src/p2p.rs` (P2PNode, BourbakiBehaviour)
- `crates/bourbaki-mesh/src/consensus.rs` (ProofAttestationEngine)
- `src/bourbakimesh/models.py` (BourbakiMuZero.load_from_checkpoint)
- Epic #16 (P2P Mesh Network)

---

## Issue #21: feat(infra): Multi-Tier Model Registry & Release Asset Distribution Pipeline
**URL:** https://github.com/tryggth/bourbakimesh/issues/21
**Labels:** ml, mesh, infrastructure

### Current Body
### Objective
Establish automated distribution, versioning, and download pipelines for trained BourbakiMuZero neural checkpoints across GitHub Releases, Hugging Face Hub, and decentralized P2P chunks.

### Sub-Tasks & Deliverables
- [ ] **GitHub Releases Integration:** Tag release `v0.1.0-alpha` attaching `bourbaki_v0.pt`, `bourbaki_v1.pt`, and `CHECKSUMS.txt`. *(completed — see [v0.1.0-alpha](https://github.com/tryggth/bourbakimesh/releases/tag/v0.1.0-alpha))*
- [ ] **Hugging Face Hub Mirror (`tryggth/bourbakimesh-muzero`):** Add automated model card generation and artifact sync script (`scripts/sync_huggingface.py`).
- [ ] **P2P Content-Addressed Model Chunks (`crates/bourbaki-mesh`):** Split serialized weights into SHA-256 content-addressed chunks for Bitswap-style GossipSub distribution among edge nodes.
- [ ] **Auto-Download CLI:** Implement `bourbakimesh.models.pull --checkpoint bourbaki_v1` with fallback from P2P → GitHub Release → Hugging Face.

### Context
- Release `v0.1.0-alpha` published: https://github.com/tryggth/bourbakimesh/releases/tag/v0.1.0-alpha
- Checksum manifest: `checkpoints/CHECKSUMS.txt`

---

## Issue #20: feat(ml): R&D — Neural and Game-Semantic Hinting Mechanisms for BourbakiMuZero
**URL:** https://github.com/tryggth/bourbakimesh/issues/20
**Labels:** enhancement, ml, rfc

### Current Body
Explore mechanisms to inject external mathematical hints (lemma suggestions, policy warping, arena graph cuts, and SMT clues) into BourbakiMuZero search while preserving zero-trust Lean 4 kernel verification. Tracked in rfcs/0002-neural-game-semantic-hinting.md.

---

## Issue #18: epic(kernel): Phase 6 — Universal Multi-Target Extraction (Coq, Isabelle, Dedukti)
**URL:** https://github.com/tryggth/bourbakimesh/issues/18
**Labels:** epic, kernel

### Current Body
Implement backend code emitters in crates/bourbaki-kernel targeting Coq, Isabelle/HOL, and Dedukti, proving semantic universality of the game-semantic IR.

---

## Issue #17: epic(ui): Phase 5 — Real-Time Proof DAG Visualizer & Interactive Web UI
**URL:** https://github.com/tryggth/bourbakimesh/issues/17
**Labels:** epic, ui

### Current Body
Develop interactive web dashboard and visualization engine to render polarized Hyland-Ong dialogue plays, live proof DAGs, and cluster worker metrics.

---

## Issue #16: epic(p2p): Phase 4 — Decentralized P2P Mesh Network & Byzantine-Resilient Ledger
**URL:** https://github.com/tryggth/bourbakimesh/issues/16
**Labels:** epic, mesh

### Current Body
Extend crates/bourbaki-mesh with libp2p gossip protocols, distributed proof validation consensus, and peer discovery across heterogeneous edge worker nodes.

---

