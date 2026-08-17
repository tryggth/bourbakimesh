"""PyTest test suite for BourbakiMuZero K-step recurrent training pipeline."""

import os
import tempfile
import numpy as np
import pytest
import torch
from torch.utils.data import DataLoader
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer
from bourbakimesh.training import (
    BourbakiTrainer,
    ReplayDataset,
    TrajectoryWindow,
    TrainingConfig,
)


@pytest.fixture
def synthetic_replay_buffer():
    """Construct a synthetic ReplayBuffer containing multi-step dialogue trajectories."""
    buffer = ReplayBuffer(capacity=50)
    feature_dim = 16
    action_space_size = 8

    for traj_idx in range(10):
        traj = GameTrajectory()
        traj_len = 8

        for step in range(traj_len):
            obs = torch.randn(feature_dim, dtype=torch.float32)
            action = step % action_space_size
            policy = np.zeros(action_space_size, dtype=np.float32)
            policy[action] = 0.9
            policy += 0.1 / action_space_size
            reward = 0.1 * step
            player = 1 if step % 2 == 0 else -1

            traj.states.append(obs)
            traj.actions.append(action)
            traj.policies.append(policy)
            traj.rewards.append(reward)
            traj.players.append(player)

        traj.terminal_value = 1.0
        buffer.push(traj)

    return buffer


def test_replay_dataset_slicing_and_collation(synthetic_replay_buffer):
    """Verify ReplayDataset creates valid K-step unroll windows and collates properly."""
    unroll_steps = 4
    feature_dim = 16
    action_space_size = 8

    dataset = ReplayDataset.from_replay_buffer(
        synthetic_replay_buffer,
        unroll_steps=unroll_steps,
        feature_dim=feature_dim,
        action_space_size=action_space_size,
    )

    assert len(dataset) > 0

    window = dataset[0]
    assert isinstance(window, TrajectoryWindow)
    assert window.obs_0.shape == (feature_dim,)
    assert window.actions.shape == (unroll_steps,)
    assert window.target_policies.shape == (unroll_steps + 1, action_space_size)
    assert window.target_values.shape == (unroll_steps + 1, 1)
    assert window.target_rewards.shape == (unroll_steps, 1)
    assert window.mask.shape == (unroll_steps + 1,)

    dataloader = DataLoader(
        dataset, batch_size=4, shuffle=False, collate_fn=ReplayDataset.collate_fn
    )
    batch = next(iter(dataloader))

    assert batch["obs_0"].shape == (4, feature_dim)
    assert batch["actions"].shape == (4, unroll_steps)
    assert batch["target_policies"].shape == (4, unroll_steps + 1, action_space_size)
    assert batch["target_values"].shape == (4, unroll_steps + 1, 1)
    assert batch["target_rewards"].shape == (4, unroll_steps, 1)
    assert batch["masks"].shape == (4, unroll_steps + 1)


def test_single_step_optimization_and_loss_decrease(synthetic_replay_buffer):
    """Verify trainer updates weights and reduces loss over multiple steps on fixed batch."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(config)
    trainer = BourbakiTrainer(
        model,
        TrainingConfig(batch_size=8, learning_rate=5e-3, unroll_steps=3),
    )

    dataset = ReplayDataset.from_replay_buffer(
        synthetic_replay_buffer,
        unroll_steps=3,
        feature_dim=16,
        action_space_size=8,
    )
    dataloader = DataLoader(
        dataset, batch_size=8, shuffle=False, collate_fn=ReplayDataset.collate_fn
    )
    batch = next(iter(dataloader))

    initial_res = trainer.train_step(batch)

    # Train several iterations on the same batch
    for _ in range(15):
        final_res = trainer.train_step(batch)

    assert final_res.total_loss < initial_res.total_loss


def test_k_step_gradient_flow(synthetic_replay_buffer):
    """Verify gradients propagate through all recurrent dynamics transitions g_theta."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(config)
    trainer = BourbakiTrainer(
        model,
        TrainingConfig(batch_size=4, unroll_steps=5),
    )

    dataset = ReplayDataset.from_replay_buffer(
        synthetic_replay_buffer,
        unroll_steps=5,
        feature_dim=16,
        action_space_size=8,
    )
    dataloader = DataLoader(
        dataset, batch_size=4, shuffle=False, collate_fn=ReplayDataset.collate_fn
    )
    batch = next(iter(dataloader))

    model.zero_grad()
    loss, _ = trainer.compute_loss(batch)
    loss.backward()

    # Check that dynamics network parameters have non-zero gradients
    dynamics_has_grad = any(
        p.grad is not None and p.grad.abs().sum() > 0
        for p in model.dynamics.parameters()
    )
    assert dynamics_has_grad

    # Check representation network parameters have non-zero gradients
    representation_has_grad = any(
        p.grad is not None and p.grad.abs().sum() > 0
        for p in model.representation.parameters()
    )
    assert representation_has_grad


def test_checkpoint_save_and_load():
    """Verify saving and loading model checkpoints restores exact network predictions."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(config)
    trainer = BourbakiTrainer(model, TrainingConfig())

    model.eval()
    obs = torch.randn(1, 16)
    with torch.no_grad():
        orig_s0, orig_p, orig_v = model.initial_inference(obs)

    with tempfile.NamedTemporaryFile(suffix=".pt", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        trainer.save_checkpoint(tmp_path, extra_meta={"test_flag": True})
        assert os.path.exists(tmp_path)

        # Create fresh model with different initial weights
        new_model = BourbakiMuZero(config)
        new_trainer = BourbakiTrainer(new_model, TrainingConfig())
        meta = new_trainer.load_checkpoint(tmp_path)
        new_model.eval()

        assert meta["extra_meta"]["test_flag"] is True

        with torch.no_grad():
            new_s0, new_p, new_v = new_model.initial_inference(obs)

        torch.testing.assert_close(orig_s0, new_s0)
        torch.testing.assert_close(orig_p, new_p)
        torch.testing.assert_close(orig_v, new_v)
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)
