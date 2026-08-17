#!/usr/bin/env bash
set -euo pipefail

REPO="tryggth/bourbakimesh"
echo "Filing Volunteer In-Browser Prover issues on ${REPO}..."

# 1. Check if #29 is already created
T29_URL="https://github.com/tryggth/bourbakimesh/issues/29"
echo "Found: $T29_URL (Issue #30 in roadmap: In-Browser WebGPU Web Worker)"

# 2. Web Crypto Node Identity (Roadmap #31)
T30_URL=$(gh issue create --repo "$REPO" \
  --title "feat(crypto): Web Crypto Ephemeral Ed25519 Node Identity & Signed Proof Attestation" \
  --body "### Objective
Generate ephemeral cryptographic node identities in browser memory using the Web Crypto API (\`crypto.subtle\`) to sign candidate proof blocks without user accounts.

### Deliverables
- [ ] Ephemeral Ed25519/ECDSA keypair generator in \`ui/src/services/cryptoIdentity.ts\`.
- [ ] Cryptographic signing of extracted CIC strategy hashes into candidate \`ProofBlock\` JSON payloads.
- [ ] Zero-trust verification compatibility with Rust \`ProofAttestationEngine\`." \
  --label "ui,mesh,enhancement")

echo "Created: $T30_URL"

# 3. UI Controls & Throttle (Roadmap #32)
T31_URL=$(gh issue create --repo "$REPO" \
  --title "feat(ui): 'Contribute Cycles' Toggle, Resource Throttle Slider & Contributor Dashboard" \
  --body "### Objective
Build the volunteer contribution UI components including the 'Contribute Cycles' activation button, power throttle controls (Eco/Balanced/Full), battery/tab-visibility thermal protection, and real-time contributor statistics.

### Deliverables
- [ ] 'Contribute Cycles' activation card and resource slider in \`ui/src/components/VolunteerPanel.tsx\`.
- [ ] Page Visibility and Battery Status API listeners to automatically pause compute on low power or hidden tabs.
- [ ] Contributor metrics panel showing local MCTS sims/sec throughput and theorems solved." \
  --label "ui,enhancement")

echo "Created: $T31_URL"

# 4. Relay Gateway Bridge (Roadmap #33)
T32_URL=$(gh issue create --repo "$REPO" \
  --title "feat(mesh): WebSocket/WebRTC Gateway Bridge for Browser Worker Task Claiming" \
  --body "### Objective
Extend \`bourbakimesh.api.server\` and \`crates/bourbaki-mesh\` to bridge libp2p GossipSub topics (\`/bourbaki/1.0.0/tasks\` and \`/bourbaki/1.0.0/proofs\`) directly to browser workers over secure WebSockets.

### Deliverables
- [ ] WebSocket handler in FastAPI bridge routing task announcements to connected browser workers.
- [ ] Task claim and proof block submission endpoint with zero-trust Lean 4 verification gate.
- [ ] P2P worker attestation test suite verifying browser-submitted proofs." \
  --label "mesh,rust,enhancement")

echo "Created: $T32_URL"

echo "✅ Tickets created on ${REPO}:"
echo "  - In-Browser WebGPU Web Worker: $T29_URL"
echo "  - Web Crypto Node Identity:     $T30_URL"
echo "  - UI Controls & Throttle:       $T31_URL"
echo "  - Relay Gateway Bridge:         $T32_URL"
