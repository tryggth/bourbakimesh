#!/usr/bin/env bash
set -euo pipefail

REPO="tryggth/bourbakimesh"
echo "Creating fix tickets for v2 recovery on ${REPO}..."

create_issue_with_retry() {
  local title="$1"
  local body="$2"
  local labels="$3"
  local max_attempts=5
  local attempt=1
  local delay=2

  while [ $attempt -le $max_attempts ]; do
    if url=$(gh issue create --repo "$REPO" --title "$title" --body "$body" --label "$labels" 2>/dev/null); then
      echo "$url"
      return 0
    else
      echo "⚠️ Attempt $attempt failed for '$title', retrying in ${delay}s..." >&2
      sleep $delay
      delay=$((delay * 2))
      attempt=$((attempt + 1))
    fi
  done

  echo "❌ Failed to create issue '$title' after $max_attempts attempts." >&2
  return 1
}

# 1. Ticket #24: Verified-Proof Prioritized Replay
T24_URL=$(create_issue_with_retry \
  "feat(training): Implement Verified-Proof Prioritized Experience Replay (PER)" \
  "### Objective
Fix policy flattening and anchor starvation by weighting Lean 4-verified proof traces (5x weight) in \`ReplayBuffer\` and enforcing a steady synthetic tableau anchor budget.

### Deliverables
- [ ] Add \`verified_boost: float = 5.0\` to \`ReplayBuffer\` sampling in \`src/bourbakimesh/self_play.py\`.
- [ ] Tag verified game semantic strategies from Lean 4 verification harness.
- [ ] Enforce default 8–10 tableau seeds per iteration in training loop." \
  "ml,training,enhancement")
echo "Created: $T24_URL"
sleep 2

# 2. Ticket #25: Temperature-Scaled Targets
T25_URL=$(create_issue_with_retry \
  "feat(ml): Temperature-Scaled Target Distributions and Policy Sharpness Control" \
  "### Objective
Prevent policy prior flattening (where root top-1 collapsed to 0.03) by applying temperature scaling $\tau = 0.5$ to MCTS visit targets.

### Deliverables
- [ ] Implement $\pi_{\text{target}}(a) \propto N(s, a)^{1/\tau}$ in \`LatentMCTS\` / \`SelfPlayWorker\`.
- [ ] Add policy entropy penalty / temperature argument in \`LoopConfig\`." \
  "ml,training")
echo "Created: $T25_URL"
sleep 2

# 3. Ticket #26: Champion Gating
T26_URL=$(create_issue_with_retry \
  "feat(training): Continuous Champion Gating via Head-to-Head Tournament Validation" \
  "### Objective
Ensure \`best_model.pt\` is only updated if candidate checkpoint beats incumbent champion in head-to-head tournament evaluation.

### Deliverables
- [ ] Integrate \`ModelTournament\` evaluation directly into \`ContinuousTrainingLoop\`.
- [ ] Reject checkpoint promotion if win rate $\le 50\%$ against incumbent." \
  "training,benchmarks")
echo "Created: $T26_URL"
sleep 2

# 4. Ticket #27: Train bourbaki_v2.pt
T27_URL=$(create_issue_with_retry \
  "feat(ml): Train and Certify bourbaki_v2.pt with Calibrated Gated Pipeline" \
  "### Objective
Execute calibrated 50-iteration fine-tuning run starting from \`bourbaki_v0.pt\` using PER, sharpened policy targets, and champion gating to produce \`bourbaki_v2.pt\` (>1650 Elo)." \
  "ml,training")
echo "Created: $T27_URL"

echo "✅ Tickets created successfully."
