#!/usr/bin/env bash
set -euo pipefail

REPO="tryggth/bourbakimesh"
echo "Filing Target Theorem Injection issue on ${REPO}..."

T34_URL=$(gh issue create --repo "$REPO" \
  --title "feat(mesh): Top-Level Target Theorem Injection & Swarm Focus Protocol" \
  --body "### Objective
Implement the end-to-end mechanism for operators and web users to set the active top-level target proof being tackled, elaborating Lean 4 propositions into root game arenas, broadcasting high-priority focus directives across libp2p GossipSub, and tracking swarm convergence in real-time.

### Deliverables
- [ ] **Target Injection API (\`src/bourbakimesh/api/server.py\`):** Endpoint \`POST /api/target/set\` and \`GET /api/target/current\` accepting raw Lean 4 code or Mathlib theorem names.
- [ ] **Swarm Directive Protocol (\`src/bourbakimesh/api/\` & \`crates/bourbaki-mesh\`):** Broadcast \`swarm_target_set\` over WebSockets and GossipSub.
- [ ] **Web UI Target Bar (\`ui/src/components/TargetManager.tsx\`):** Prominent header input bar, Mathlib selector, and real-time subgoal resolution progress banner." \
  --label "mesh,ui,enhancement")

echo "Created: $T34_URL"
echo "✅ Ticket #34 created on ${REPO}: ${T34_URL}"
