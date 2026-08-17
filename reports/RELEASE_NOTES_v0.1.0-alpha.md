# BourbakiMesh v0.1.0-alpha — Neural Baseline & Self-Play Fine-Tuning

This initial alpha release delivers the core polyglot monorepo (Rust, Lean 4, Python) and the first generation of trained **BourbakiMuZero** neural theorem-proving models.

### Included Checkpoints
- **`bourbaki_v0.pt`** (12 MB): Cold-start baseline trained on synthetic tableau proofs and foundational Mathlib curriculum tiers.
- **`bourbaki_v1.pt`** (12 MB): Self-play fine-tuned model (40 iterations, 120 sims/move) achieving 715.0 MCTS sims/sec on CPU and a **1.430x Compute Simulation Equivalent (CSE)** score.
- **`CHECKSUMS.txt`**: SHA-256 integrity verification hashes.

### Key Subsystem Milestones
- **Game-Semantic IR (`bourbaki-ir`)**: Polarized Hyland-Ong dialogue arena semantics, P-view/O-view calculation, and well-bracketed stack discipline.
- **CIC Kernel & Strategy Extractor (`bourbaki-kernel`)**: Constructive strategy compilation $\mathcal{E}(\sigma)$, Lean 4 code emitter, and zero-trust Lean 4 kernel verification harness.
- **Distributed Proof Swarm (`bourbaki-mesh`)**: P2P `libp2p` (v0.53) GossipSub topics (`/bourbaki/1.0.0/tasks`, `/bourbaki/1.0.0/proofs`), Kademlia DHT peer routing, and Byzantine proof block attestation against $\bot$ falsifications.
- **Relational Neural Dynamics (`bourbakimesh`)**: 25M-parameter Relational Arena Transformer ($h_\theta$), recurrent latent dynamics ($g_\theta$), and polarity-inverting Latent MCTS ($f_\theta$).
- **Curriculum Ingestion Engine**: Automated Lean 4 theorem export metaprogram (`Export.lean`), binary corpus decompiler, and 3-tier topological difficulty indexer.
- **Monorepo Test Matrix**: **87 / 87 passed tests** across Rust, Python, and Lean 4.

### Model Download Quickstart
```bash
gh release download v0.1.0-alpha --dir checkpoints/
```
