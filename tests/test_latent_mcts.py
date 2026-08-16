"""PyTest test suite for BourbakiMuZero neural dynamics and Latent MCTS self-play."""

import numpy as np
import pytest
import torch
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig, Node
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer, SelfPlayWorker


@pytest.fixture
def mini_config():
    """Compact model configuration for fast test execution."""
    return ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=64,
        num_res_blocks=2,
    )


@pytest.fixture
def mini_model(mini_config):
    """Instantiated BourbakiMuZero test model."""
    torch.manual_seed(42)
    return BourbakiMuZero(mini_config)


def test_muzero_forward_shapes(mini_model):
    """Verify representation, dynamics, and prediction networks produce correct tensor shapes."""
    batch_size = 4
    obs = torch.randn(batch_size, 16)

    # Initial inference
    s0, policy_logits, value = mini_model.initial_inference(obs)
    assert s0.shape == (batch_size, 32)
    assert policy_logits.shape == (batch_size, 8)
    assert value.shape == (batch_size, 1)

    # Check unit-sphere normalization of latent embeddings
    norms = torch.norm(s0, p=2, dim=-1)
    assert torch.allclose(norms, torch.ones(batch_size), atol=1e-3)

    # Recurrent inference
    actions = torch.tensor([0, 1, 2, 3], dtype=torch.long)
    s1, reward, p1, v1 = mini_model.recurrent_inference(s0, actions)
    assert s1.shape == (batch_size, 32)
    assert reward.shape == (batch_size, 1)
    assert p1.shape == (batch_size, 8)
    assert v1.shape == (batch_size, 1)


def test_latent_mcts_search_distribution(mini_model):
    """Verify MCTS search produces valid probability distributions."""
    config = MCTSConfig(num_simulations=40, exploration_fraction=0.2)
    mcts = LatentMCTS(mini_model, config)

    obs = torch.randn(16)
    policy = mcts.search(obs, current_player=1)

    assert len(policy) == 8
    assert np.isclose(policy.sum(), 1.0, atol=1e-4)
    assert all(p >= 0.0 for p in policy)


def test_polarity_inversion_backpropagation():
    """Verify Opponent perspective correctly negates Proponent value estimates during backprop."""
    # Root: Proponent (+1)
    root = Node(player=1)
    root.latent_state = torch.zeros(1, 32)

    # Child: Opponent (-1)
    child = Node(player=-1, parent=root, action=0, prior=0.5)
    root.children[0] = child
    child.latent_state = torch.zeros(1, 32)

    # Grandchild: Proponent (+1)
    grandchild = Node(player=1, parent=child, action=1, prior=0.5)
    child.children[1] = grandchild

    # Backpropagate a positive value +1.0 evaluated from Grandchild (Proponent's view)
    grandchild.backpropagate(1.0)

    # Grandchild (P) got +1.0
    assert grandchild.visit_count == 1
    assert np.isclose(grandchild.value_sum, 1.0)

    # Child (O) must receive inverted perspective -1.0
    assert child.visit_count == 1
    assert np.isclose(child.value_sum, -1.0)

    # Root (P) must receive +1.0
    assert root.visit_count == 1
    assert np.isclose(root.value_sum, 1.0)


def test_self_play_trajectory_and_replay_buffer(mini_model):
    """Verify self-play game simulation and experience replay buffer sampling."""
    config = MCTSConfig(num_simulations=15)
    worker = SelfPlayWorker(mini_model, config)

    # Execute a self-play game
    trajectory = worker.play_game(max_moves=6, num_simulations=15)

    assert len(trajectory.actions) > 0
    assert len(trajectory.states) == len(trajectory.actions)
    assert len(trajectory.policies) == len(trajectory.actions)
    assert len(trajectory.rewards) == len(trajectory.actions)
    assert len(trajectory.players) == len(trajectory.actions)

    # Verify experience replay buffer
    buffer = ReplayBuffer(capacity=50)
    buffer.push(trajectory)
    assert len(buffer) == 1
    assert buffer.total_steps() == len(trajectory.actions)

    # Sample batch from replay buffer
    batch = buffer.sample_batch(batch_size=4)
    assert "states" in batch
    assert "actions" in batch
    assert "target_policies" in batch
    assert "target_values" in batch

    assert batch["states"].shape[-1] == mini_model.config.feature_dim
    assert batch["target_policies"].shape[-1] == mini_model.config.action_space_size
    assert batch["target_values"].shape[-1] == 1
