"""Unit and integration tests for head-to-head tournament engine and Bayesian Elo tracker."""

import json
from pathlib import Path
import tempfile
import pytest
import torch
from bourbakimesh.benchmarks.elo import EloTracker
from bourbakimesh.benchmarks.tournament import (
    ModelTournament,
    TournamentProposition,
    TournamentReport,
)
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero


def test_elo_tracker_synthetic_convergence():
    """Verify EloTracker computes correct ranking and win rates on synthetic match records."""
    tracker = EloTracker(base_rating=1500.0, prior_std=300.0)

    # Model A vs Model B: A wins 8 out of 10
    for _ in range(8):
        tracker.add_match("model_a", "model_b", score_a=1.0, tier=1, num_plies=4, duration_ms=20.0)
    for _ in range(2):
        tracker.add_match("model_a", "model_b", score_a=0.0, tier=1, num_plies=6, duration_ms=25.0)

    # Model B vs Model C: B wins 7 out of 10
    for _ in range(7):
        tracker.add_match("model_b", "model_c", score_a=1.0, tier=2, num_plies=5, duration_ms=30.0)
    for _ in range(3):
        tracker.add_match("model_b", "model_c", score_a=0.0, tier=2, num_plies=5, duration_ms=30.0)

    summaries = tracker.get_summary()

    assert "model_a" in summaries
    assert "model_b" in summaries
    assert "model_c" in summaries

    # Elo ordering: A > B > C
    assert summaries["model_a"].elo > summaries["model_b"].elo
    assert summaries["model_b"].elo > summaries["model_c"].elo

    # Win rates
    assert summaries["model_a"].win_rate == 0.8
    assert summaries["model_a"].wins == 8
    assert summaries["model_a"].losses == 2
    assert summaries["model_a"].elo_stderr > 0.0
    assert summaries["model_a"].ci_lower < summaries["model_a"].elo < summaries["model_a"].ci_upper


def test_model_tournament_paired_games():
    """Verify ModelTournament runs paired matches with alternating polarities without errors."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model_a = BourbakiMuZero(config)
    model_b = BourbakiMuZero(config)

    props = [
        TournamentProposition("Test.Prop1", tier=1),
        TournamentProposition("Test.Prop2", tier=2),
    ]

    tournament = ModelTournament(
        models={"v0": model_a, "v1": model_b},
        propositions=props,
        simulations=10,
        device="cpu",
        max_moves=6,
    )

    report = tournament.run_tournament()

    assert report.total_games == 4  # 2 propositions * 2 games per pair
    assert "v0" in report.summaries
    assert "v1" in report.summaries
    assert len(report.games) == 4

    # Verify JSON serialization
    report_dict = report.to_dict()
    assert report_dict["total_games"] == 4
    assert "summaries" in report_dict
    assert "games" in report_dict


def test_tournament_against_real_checkpoints():
    """Verify tournament execution against trained v0 and v1 checkpoints."""
    v0_path = Path("checkpoints/bourbaki_v0.pt")
    v1_path = Path("checkpoints/bourbaki_v1.pt")

    if not (v0_path.exists() and v1_path.exists()):
        pytest.skip("Trained checkpoints not found in checkpoints/")

    model_v0 = BourbakiMuZero.load_from_checkpoint(v0_path, map_location="cpu")
    model_v1 = BourbakiMuZero.load_from_checkpoint(v1_path, map_location="cpu")

    props = [
        TournamentProposition("Mathlib.Logic.Basic.id", tier=1),
        TournamentProposition("Mathlib.Logic.Basic.modus_ponens", tier=2),
    ]

    tournament = ModelTournament(
        models={"bourbaki_v0": model_v0, "bourbaki_v1": model_v1},
        propositions=props,
        simulations=15,
        device="cpu",
        max_moves=6,
    )

    report = tournament.run_tournament()

    assert report.total_games == 4
    assert "bourbaki_v0" in report.summaries
    assert "bourbaki_v1" in report.summaries
    for s in report.summaries.values():
        assert s.total_games == 4
        assert s.mean_plies > 0
        assert s.mean_latency_ms > 0
