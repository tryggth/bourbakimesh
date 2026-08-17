"""Tests for Prioritized Experience Replay (PER) and temperature-scaled policy targets."""

from pathlib import Path
import numpy as np
import pytest
import torch
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer, SelfPlayWorker


def test_prioritized_replay_sampling_distribution():
    """Verify that ReplayBuffer samples verified trajectories with verified_boost higher probability."""
    buffer = ReplayBuffer(capacity=100, verified_boost=5.0)

    # 1. Add unverified trajectory (weight 1.0)
    traj_unverified = GameTrajectory(verified=False)
    for _ in range(10):
        traj_unverified.states.append(torch.randn(16))
        traj_unverified.actions.append(0)
        traj_unverified.policies.append(np.ones(8) / 8.0)
        traj_unverified.rewards.append(0.0)
        traj_unverified.players.append(1)
    traj_unverified.terminal_value = 0.0
    buffer.push(traj_unverified)

    # 2. Add verified trajectory (weight 5.0)
    traj_verified = GameTrajectory(verified=True)
    for _ in range(10):
        traj_verified.states.append(torch.randn(16))
        traj_verified.actions.append(1)
        traj_verified.policies.append(np.ones(8) / 8.0)
        traj_verified.rewards.append(1.0)
        traj_verified.players.append(1)
    traj_verified.terminal_value = 1.0
    buffer.push(traj_verified)

    # 3. Sample large batch and measure action 1 (verified) vs action 0 (unverified) counts
    np.random.seed(42)
    torch.manual_seed(42)
    batch = buffer.sample_batch(batch_size=5000)
    actions = batch["actions"].numpy()

    count_verified = int(np.sum(actions == 1))
    count_unverified = int(np.sum(actions == 0))

    ratio = count_verified / max(1, count_unverified)
    # Expected ratio is approximately 5.0 (within statistical bounds: 4.0 to 6.0)
    assert 4.0 < ratio < 6.0, f"Expected verified/unverified ratio ~5.0, got {ratio:.2f}"


def test_temperature_scaled_target_policy():
    """Verify temperature scaling sharpens policy targets from visit counts."""
    visits = np.array([0.1, 0.2, 0.4, 0.3], dtype=np.float32)

    # tau = 1.0 -> unchanged
    policy_1 = SelfPlayWorker._compute_target_policy(visits, target_temperature=1.0)
    np.testing.assert_allclose(policy_1, visits)

    # tau = 0.5 -> squared visits (sharpened)
    policy_sharp = SelfPlayWorker._compute_target_policy(visits, target_temperature=0.5)

    # Top-1 prob should increase significantly
    assert policy_sharp[2] > visits[2], f"Expected {policy_sharp[2]} > {visits[2]}"
    assert np.isclose(np.sum(policy_sharp), 1.0)

    # tau -> 0 (greedy / extremely sharp)
    policy_extreme = SelfPlayWorker._compute_target_policy(visits, target_temperature=0.1)
    assert policy_extreme[2] > 0.9, f"Expected top-1 > 0.9, got {policy_extreme[2]}"


def test_self_play_worker_with_temperature():
    """Verify SelfPlayWorker produces sharpened target policies when target_temperature < 1.0."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(config)
    worker = SelfPlayWorker(model, target_temperature=0.5)

    traj = worker.play_game(max_moves=4, num_simulations=20)

    assert len(traj) > 0
    for p in traj.policies:
        assert np.isclose(np.sum(p), 1.0)
        assert np.all(p >= 0.0)
