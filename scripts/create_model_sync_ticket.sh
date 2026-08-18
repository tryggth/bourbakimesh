#!/usr/bin/env bash
set -euo pipefail

REPO="tryggth/bourbakimesh"
echo "Filing Automated Model Sync & Hot-Reload issue on ${REPO}..."

T35_URL=$(gh issue create --repo "$REPO" \
  --title "feat(mesh): Automated P2P Model Weight Hot-Reloading & Swarm Sync Protocol" \
  --body "### Objective
Implement decentralized model weight discovery, peer-to-peer chunk synchronization, and zero-downtime in-memory hot-reloading for both desktop daemon nodes (bourbaki-daemon) and in-browser Web Workers (prover.worker.ts).

### Deliverables
- [ ] **GossipSub Model Announcement Topic (\`crates/bourbaki-mesh/src/p2p.rs\`):** Add \`/bourbaki/1.0.0/models\` topic for broadcasting certified model updates.
- [ ] **Daemon Hot-Reloader (\`crates/bourbaki-mesh/src/worker.rs\`):** Fetch missing chunks via \`ModelChunker\`, verify Merkle root hash, and reload weights between task executions.
- [ ] **Browser Web Worker Hot-Swap (\`ui/src/workers/prover.worker.ts\`):** Invalidate IndexedDB cache on upgrade signal, download new ONNX graph, and instantiate new WebGPU session.
- [ ] **Integration Test Suite (\`crates/bourbaki-mesh/tests/model_sync_tests.rs\`):** Verify multi-node cluster automatically upgrades inference engine without dropping tasks." \
  --label "mesh,ui,enhancement")

echo "Created: $T35_URL"
echo "✅ Ticket #35 created on ${REPO}: ${T35_URL}"
