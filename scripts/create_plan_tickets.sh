#!/usr/bin/env bash
set -euo pipefail

REPO="tryggth/bourbakimesh"
echo "Creating GitHub issues for roadmap and R&D plan on ${REPO}..."

# ──────────────────────────────────────────────────
# NOTE: Issue #20 (Neural & Game-Semantic Hinting Engine) already exists.
# See: https://github.com/tryggth/bourbakimesh/issues/20
# ──────────────────────────────────────────────────

# 1. Ticket: Multi-Tier Model Registry & Asset Distribution
gh issue create --repo "$REPO" \
  --title "feat(infra): Multi-Tier Model Registry & Release Asset Distribution Pipeline" \
  --body "### Objective
Establish automated distribution, versioning, and download pipelines for trained BourbakiMuZero neural checkpoints across GitHub Releases, Hugging Face Hub, and decentralized P2P chunks.

### Sub-Tasks & Deliverables
- [ ] **GitHub Releases Integration:** Tag release \`v0.1.0-alpha\` attaching \`bourbaki_v0.pt\`, \`bourbaki_v1.pt\`, and \`CHECKSUMS.txt\`. *(completed — see [v0.1.0-alpha](https://github.com/tryggth/bourbakimesh/releases/tag/v0.1.0-alpha))*
- [ ] **Hugging Face Hub Mirror (\`tryggth/bourbakimesh-muzero\`):** Add automated model card generation and artifact sync script (\`scripts/sync_huggingface.py\`).
- [ ] **P2P Content-Addressed Model Chunks (\`crates/bourbaki-mesh\`):** Split serialized weights into SHA-256 content-addressed chunks for Bitswap-style GossipSub distribution among edge nodes.
- [ ] **Auto-Download CLI:** Implement \`bourbakimesh.models.pull --checkpoint bourbaki_v1\` with fallback from P2P → GitHub Release → Hugging Face.

### Context
- Release \`v0.1.0-alpha\` published: https://github.com/tryggth/bourbakimesh/releases/tag/v0.1.0-alpha
- Checksum manifest: \`checkpoints/CHECKSUMS.txt\`" \
  --label "infrastructure,ml,mesh"

echo "Created Registry Issue."

# 2. Ticket: Standalone P2P Daemon
gh issue create --repo "$REPO" \
  --title "feat(mesh): Implement standalone bourbaki-daemon with embedded MCTS worker" \
  --body "### Objective
Build a standalone binary CLI (\`crates/bourbaki-mesh/src/bin/bourbaki-daemon.rs\`) that joins the libp2p GossipSub network, claims open Mathlib subgoals, and evaluates them using embedded \`bourbaki_v1.pt\` MCTS search.

### Sub-Tasks & Deliverables
- [ ] **Daemon CLI Binary (\`crates/bourbaki-mesh/src/bin/bourbaki-daemon.rs\`):** Accept \`--peer-port\`, \`--model-path\`, \`--simulations\`, and \`--max-tasks\`.
- [ ] **Bidirectional Python FFI / IPC Worker:** Bridge Tokio P2P event loop with Python \`BourbakiMuZero\` engine via the existing \`MeshIpcServer\` TCP protocol.
- [ ] **Task Claim & Gossip Submission Loop:** Subscribe to \`/bourbaki/1.0.0/tasks\`, claim unproven subgoals, run Latent MCTS search, extract verified CIC proof blocks via \`StrategyExtractor\`, and gossip attested blocks to \`/bourbaki/1.0.0/proofs\`.
- [ ] **Byzantine Attestation Integration:** Apply \`ProofAttestationEngine\` validation before broadcasting candidate proofs.
- [ ] **Systemd / Docker Packaging:** Containerize daemon for headless edge server deployment.

### Dependencies
- \`crates/bourbaki-mesh/src/p2p.rs\` (P2PNode, BourbakiBehaviour)
- \`crates/bourbaki-mesh/src/consensus.rs\` (ProofAttestationEngine)
- \`src/bourbakimesh/models.py\` (BourbakiMuZero.load_from_checkpoint)
- Epic #16 (P2P Mesh Network)" \
  --label "mesh,rust,p2p"

echo "Created Daemon Issue."

# 3. Ticket: Scaled Tournament & Elo Evaluation
gh issue create --repo "$REPO" \
  --title "test(bench): Scaled Self-Play Tournament & Elo Evaluation Harness" \
  --body "### Objective
Construct an automated head-to-head tournament evaluation harness to track empirical Elo ratings, theorem solve rates, and search efficiency gains across model generations (v0 vs v1 vs v2).

### Sub-Tasks & Deliverables
- [ ] **Head-to-Head Tournament Engine (\`src/bourbakimesh/benchmarks/tournament.py\`):** Pair checkpoints across standard Mathlib proposition sets with alternating Proponent/Opponent polarities.
- [ ] **Bayesian Elo Tracker:** Calculate posterior Elo ratings and confidence intervals for all promoted checkpoints.
- [ ] **Curriculum Solve-Rate Benchmark Suite:** Standardize 100-proposition held-out benchmark dataset across Tiers 1–3.
- [ ] **Automated Performance Dashboard:** Output markdown comparison matrices and latency graphs to \`reports/\`.

### Metrics to Track
- Solve rate by theorem difficulty tier (T1/T2/T3)
- MCTS sims/sec throughput per checkpoint
- Mean proof length (dialogue moves) per solved theorem
- Compute Simulation Equivalent (CSE) score per model generation

### Dependencies
- \`checkpoints/bourbaki_v0.pt\`, \`checkpoints/bourbaki_v1.pt\`
- \`src/bourbakimesh/benchmarks/bench_engine.py\` (BenchmarkRunner)
- \`data/curriculum/\` tiered datasets" \
  --label "benchmarks,ml"

echo "Created Tournament Issue."

echo ""
echo "✅ All plan tickets created successfully."
echo "  Hinting R&D:    https://github.com/tryggth/bourbakimesh/issues/20 (pre-existing)"
echo "  Model Registry: (see output above)"
echo "  P2P Daemon:     (see output above)"
echo "  Tournament:     (see output above)"
