"""PyTest integration test suite for BourbakiMuZero checkpoint loading and architecture auto-inference."""

import os
import tempfile
from pathlib import Path
import pytest
import torch
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero


def test_load_from_checkpoint_auto_inference():
    """Verify load_from_checkpoint accurately detects custom network dimensions from raw state_dict."""
    custom_config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=64,
        num_res_blocks=1,
        transformer_layers=2,
        transformer_heads=4,
    )
    original_model = BourbakiMuZero(custom_config)
    original_model.eval()

    obs = torch.randn(1, 16)
    with torch.no_grad():
        orig_s0, orig_p, orig_v = original_model.initial_inference(obs)

    with tempfile.NamedTemporaryFile(suffix=".pt", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        # Save only raw state_dict without metadata to test inference
        torch.save({"model_state_dict": original_model.state_dict()}, tmp_path)

        loaded_model = BourbakiMuZero.load_from_checkpoint(tmp_path, map_location="cpu")
        assert not loaded_model.training
        assert loaded_model.config.feature_dim == 16
        assert loaded_model.config.latent_dim == 32
        assert loaded_model.config.action_space_size == 8
        assert loaded_model.config.hidden_dim == 64
        assert loaded_model.config.num_res_blocks == 1
        assert loaded_model.config.transformer_layers == 2
        assert loaded_model.config.transformer_heads == 4

        with torch.no_grad():
            new_s0, new_p, new_v = loaded_model.initial_inference(obs)

        torch.testing.assert_close(orig_s0, new_s0)
        torch.testing.assert_close(orig_p, new_p)
        torch.testing.assert_close(orig_v, new_v)
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)


def test_load_from_checkpoint_v0_and_mcts_search():
    """Verify loading promoted baseline checkpoints/bourbaki_v0.pt and running MCTS search."""
    v0_path = Path("checkpoints/bourbaki_v0.pt")
    if not v0_path.exists():
        pytest.skip("checkpoints/bourbaki_v0.pt not found on disk")

    model = BourbakiMuZero.load_from_checkpoint(v0_path, map_location="cpu")
    assert not model.training
    assert model.config.feature_dim == 32
    assert model.config.latent_dim == 64
    assert model.config.action_space_size == 16

    # Test initial inference
    obs = torch.randn(1, 32)
    with torch.no_grad():
        s0, p0, v0 = model.initial_inference(obs)

    assert s0.shape == (1, 64)
    assert p0.shape == (1, 16)
    assert v0.shape == (1, 1)

    # Test MCTS search with loaded model
    mcts_config = MCTSConfig(num_simulations=10, exploration_fraction=0.0)
    mcts = LatentMCTS(model, mcts_config)
    policy = mcts.search(obs, current_player=1, num_simulations=10)

    assert len(policy) == 16
    assert abs(policy.sum() - 1.0) < 1e-4
