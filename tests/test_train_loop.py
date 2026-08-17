"""Integration test suite for BourbakiMuZero closed-loop continuous training orchestrator."""

import os
import shutil
import tempfile
from pathlib import Path
import pytest
import torch
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer
from bourbakimesh.training import (
    ContinuousTrainingLoop,
    IterationMetrics,
    LoopConfig,
)


@pytest.fixture
def temp_checkpoint_dir():
    """Create a temporary directory for test model checkpoints."""
    tmp_dir = tempfile.mkdtemp()
    yield Path(tmp_dir)
    if os.path.exists(tmp_dir):
        shutil.rmtree(tmp_dir)


def test_continuous_loop_end_to_end(temp_checkpoint_dir):
    """Verify running a 2-iteration loop synthesizes seeds, generates self-play, trains, and saves checkpoints."""
    config = LoopConfig(
        iterations=2,
        self_play_games_per_iter=3,
        tableau_seeds_per_iter=3,
        train_epochs_per_iter=1,
        simulations_per_move=10,
        batch_size=8,
        unroll_steps=2,
        checkpoint_dir=temp_checkpoint_dir,
        feature_dim=16,
        action_space_size=8,
    )

    model_config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(model_config)

    loop = ContinuousTrainingLoop(model=model, config=config)
    history = loop.run()

    assert len(history) == 2
    assert history[0].iteration == 1
    assert history[1].iteration == 2

    # Check buffer accumulation
    assert history[0].buffer_total_trajectories > 0
    assert history[1].buffer_total_trajectories >= history[0].buffer_total_trajectories
    assert history[1].buffer_total_steps > 0

    # Check checkpoint emissions
    iter1_ckpt = temp_checkpoint_dir / "checkpoint_iter_1.pt"
    iter2_ckpt = temp_checkpoint_dir / "checkpoint_iter_2.pt"
    best_ckpt = temp_checkpoint_dir / "best_model.pt"

    assert iter1_ckpt.exists()
    assert iter2_ckpt.exists()
    assert best_ckpt.exists()


def test_model_checkpoint_promotion_and_reloading(temp_checkpoint_dir):
    """Verify best promoted checkpoint is reloadable and produces valid predictions."""
    config = LoopConfig(
        iterations=1,
        self_play_games_per_iter=2,
        tableau_seeds_per_iter=2,
        train_epochs_per_iter=1,
        simulations_per_move=10,
        batch_size=4,
        unroll_steps=2,
        checkpoint_dir=temp_checkpoint_dir,
        feature_dim=16,
        action_space_size=8,
    )

    model_config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(model_config)

    loop = ContinuousTrainingLoop(model=model, config=config)
    metrics = loop.run_iteration(1)

    assert metrics.promoted is True

    best_ckpt_path = temp_checkpoint_dir / "best_model.pt"
    assert best_ckpt_path.exists()

    # Load promoted model into a new network
    fresh_model = BourbakiMuZero(model_config)
    checkpoint = torch.load(best_ckpt_path, map_location="cpu")
    fresh_model.load_state_dict(checkpoint["model_state_dict"])

    test_obs = torch.randn(1, 16)
    s0, policy, value = fresh_model.initial_inference(test_obs)

    assert s0.shape == (1, 32)
    assert policy.shape == (1, 8)
    assert value.shape == (1, 1)
    assert not torch.isnan(policy).any()
    assert not torch.isnan(value).any()


def test_replay_buffer_capacity_bounding():
    """Verify ReplayBuffer maintains fixed capacity bound under heavy continuous game ingestion."""
    buffer = ReplayBuffer(capacity=5)

    for i in range(15):
        traj = GameTrajectory()
        traj.states.append(torch.randn(16))
        traj.actions.append(i % 8)
        traj.policies.append(torch.zeros(8).numpy())
        traj.rewards.append(0.0)
        traj.players.append(1)
        traj.terminal_value = 1.0
        buffer.push(traj)

    assert len(buffer) == 5
    assert len(buffer.trajectories) == 5
