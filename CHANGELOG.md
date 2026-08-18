# Changelog

All notable changes to the BourbakiMesh project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0-alpha] - 2026-08-17

### Added
- **Game-Semantic Dialogue Arenas (`crates/bourbaki-ir`):**
  - Hyland-Ong / Lorenzen play traces with strict opponent/proponent alternation ($O \leftrightarrow P$).
  - Arena cut injection and well-bracketing stack discipline.
  - Property-based testing with `proptest` (17 tests).
- **Calculus of Inductive Constructions (CIC) Kernel & Strategy Extraction (`crates/bourbaki-kernel`):**
  - Deterministic 5-rule strategy extraction compiler $\mathcal{E}(\sigma) \to \text{Term}$.
  - Batch Mathlib corpus decompiler (`decompile_corpus` binary).
  - Universal multi-target code generators for **Lean 4, Coq (Gallina), Isabelle/HOL (Isar), and Dedukti** (31 tests).
- **Decentralized P2P Mesh & Byzantine Ledger (`crates/bourbaki-mesh`):**
  - Content-addressed `ProofBlock` DAG and `ProofLedger`.
  - libp2p GossipSub and Kademlia DHT swarm (`bourbaki-daemon` binary).
  - P2P model weight chunking (`ModelChunker`) and zero-downtime hot-reloading over `/bourbaki/1.0.0/models` (23 tests).
- **Python ML Dynamics & Latent MCTS (`src/bourbakimesh`):**
  - 25M-parameter `RelationalArenaTransformer` with pointer edge attention.
  - Polarity-inverting Latent MCTS search with Dirichlet exploration noise and temperature scaling ($\tau = 0.5$).
  - Verified Prioritized Experience Replay (PER) with 5.0x verified proof boost.
  - Automated head-to-head tournament engine and Bayesian Elo ranking (`bourbaki_v0.pt`, `bourbaki_v1.pt`, `bourbaki_v2.pt`) (69 tests).
- **Interactive Web UI & PWA (`ui/`):**
  - Auto-updating Progressive Web App on GitHub Pages with Workbox Service Worker precaching.
  - In-browser **"Contribute Cycles"** volunteer proving engine powered by ONNX Runtime WebGPU and WebAssembly SIMD (`prover.worker.ts`).
  - Ephemeral Web Crypto ECDSA P-256 node identity generation and signed proof block attestation (`cryptoIdentity.ts`).
  - Top-level swarm target theorem injection modal (`TargetManager.tsx`).
  - Offline IndexedDB proof DAG hydrator.
- **Formal Lean 4 Soundness Harness (`lean_target/`):**
  - Reference Lean 4 kernel verification harness.
  - Mechanized meta-theoretic soundness proof (`LeanTarget.MetaTheory`).
