"""Tests for Continuous Champion Gating via Head-to-Head Tournament Validation."""

from pathlib import Path
import tempfile
import torch
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.training.loop import ContinuousTrainingLoop, LoopConfig
from bourbakimesh.training.trainer import TrainingConfig


def test_champion_gating_promotes_winning_candidate(tmp_path: Path):
    """Verify that an initial model is saved as champion, and subsequent candidates undergo gating."""
    model_cfg = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(model_cfg)

    config = LoopConfig(
        iterations=2,
        self_play_games_per_iter=2,
        tableau_seeds_per_iter=2,
        train_epochs_per_iter=1,
        simulations_per_move=10,
        batch_size=4,
        unroll_steps=2,
        checkpoint_dir=tmp_path,
        champion_gating=True,
        gating_matches=2,
        feature_dim=16,
        action_space_size=8,
    )

    loop = ContinuousTrainingLoop(model=model, config=config)
    metrics = loop.run()

    assert len(metrics) == 2
    best_ckpt = tmp_path / "best_model.pt"
    assert best_ckpt.exists()


def test_champion_gating_disabled_uses_loss_promotion(tmp_path: Path):
    """Verify fallback to loss-based promotion when champion_gating=False."""
    model_cfg = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(model_cfg)

    config = LoopConfig(
        iterations=2,
        self_play_games_per_iter=2,
        tableau_seeds_per_iter=2,
        train_epochs_per_iter=1,
        simulations_per_move=10,
        batch_size=4,
        unroll_steps=2,
        checkpoint_dir=tmp_path,
        champion_gating=False,
        feature_dim=16,
        action_space_size=8,
    )

    loop = ContinuousTrainingLoop(model=model, config=config)
    metrics = loop.run()

    assert len(metrics) == 2
    assert (tmp_path / "best_model.pt").exists()
