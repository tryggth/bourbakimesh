"""PyTest integration test suite for RelationalArenaTransformer and BourbakiMuZero Phase 2 neural core."""

import pytest
import torch
import torch.nn.functional as F
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import (
    ArenaEmbeddingConfig,
    BourbakiMuZero,
    RelationalArenaTransformer,
    RelationalMultiheadAttention,
    RelationalTransformerLayer,
)


def test_relational_transformer_forward_pass_variable_lengths():
    """Verify forward execution over variable-length dialogue graphs (lengths 5, 25, 100)."""
    feature_dim = 16
    latent_dim = 32
    config = ArenaEmbeddingConfig(
        feature_dim=feature_dim,
        latent_dim=latent_dim,
        hidden_dim=64,
        transformer_layers=2,
        transformer_heads=4,
        max_seq_len=128,
    )
    model = RelationalArenaTransformer(config)

    for seq_len in [5, 25, 100]:
        batch_size = 2
        obs = torch.randn(batch_size, seq_len, feature_dim)
        rel_matrix = torch.randint(0, 5, (batch_size, seq_len, seq_len))
        polarities = torch.randint(-1, 2, (batch_size, seq_len))

        s0 = model(obs, relation_matrix=rel_matrix, polarities=polarities)

        assert s0.shape == (batch_size, latent_dim)
        # Check normalized to unit sphere
        norm = torch.norm(s0, p=2, dim=-1)
        torch.testing.assert_close(norm, torch.ones_like(norm), atol=1e-4, rtol=1e-4)


def test_relational_sensitivity_to_justification_pointers():
    """Verify modifying justification pointer relations changes the output latent embedding s_0."""
    feature_dim = 16
    latent_dim = 32
    config = ArenaEmbeddingConfig(
        feature_dim=feature_dim,
        latent_dim=latent_dim,
        hidden_dim=64,
        transformer_layers=2,
        transformer_heads=4,
    )
    model = RelationalArenaTransformer(config)
    model.eval()

    seq_len = 4
    # Identical move payloads
    obs = torch.randn(1, seq_len, feature_dim)

    # Relation graph A: Sequential only
    rel_matrix_a = torch.zeros((1, seq_len, seq_len), dtype=torch.long)
    rel_matrix_a[0, 0, 1] = 1
    rel_matrix_a[0, 1, 2] = 1
    rel_matrix_a[0, 2, 3] = 1

    # Relation graph B: With cross-justification pointer (move 3 enables move 0)
    rel_matrix_b = rel_matrix_a.clone()
    rel_matrix_b[0, 3, 0] = 2  # Relation 2: Justification Pointer

    with torch.no_grad():
        s0_a = model(obs, relation_matrix=rel_matrix_a)
        s0_b = model(obs, relation_matrix=rel_matrix_b)

    # Embeddings must differ due to relational bias
    diff = torch.norm(s0_a - s0_b, p=2).item()
    assert diff > 1e-4, f"Expected relational difference, got {diff}"


def test_gradient_flow_and_recurrent_unrolling():
    """Verify backward pass and gradient flow through RelationalArenaTransformer and BourbakiMuZero."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=64,
        transformer_layers=2,
        transformer_heads=4,
        use_relational_transformer=True,
    )
    model = BourbakiMuZero(config)

    batch_size = 2
    obs = torch.randn(batch_size, 4, config.feature_dim)
    actions = torch.randint(0, config.action_space_size, (batch_size, 5))

    s_k, p_0, v_0 = model.initial_inference(obs)
    loss = p_0.sum() + v_0.sum()

    for k in range(5):
        s_k, r_k, p_k, v_k = model.recurrent_inference(s_k, actions[:, k])
        loss = loss + p_k.sum() + v_k.sum() + r_k.sum()

    model.zero_grad()
    loss.backward()

    # Check gradients in transformer representation layers
    has_grad = any(
        p.grad is not None and p.grad.abs().sum() > 0
        for p in model.representation.parameters()
    )
    assert has_grad


def test_latent_mcts_compatibility_with_relational_transformer():
    """Verify LatentMCTS search executes seamlessly with RelationalArenaTransformer backbone."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=64,
        transformer_layers=2,
        transformer_heads=4,
        use_relational_transformer=True,
    )
    model = BourbakiMuZero(config)
    mcts_config = MCTSConfig(num_simulations=15, exploration_fraction=0.0)
    mcts = LatentMCTS(model, mcts_config)

    # 1. 2D observation
    obs_2d = torch.randn(1, config.feature_dim)
    policy_2d = mcts.search(obs_2d, current_player=1, num_simulations=15)
    assert len(policy_2d) == config.action_space_size
    assert abs(policy_2d.sum() - 1.0) < 1e-4

    # 2. 3D sequence observation
    obs_3d = torch.randn(1, 6, config.feature_dim)
    policy_3d = mcts.search(obs_3d, current_player=-1, num_simulations=15)
    assert len(policy_3d) == config.action_space_size
    assert abs(policy_3d.sum() - 1.0) < 1e-4
