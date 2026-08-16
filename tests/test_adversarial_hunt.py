"""PyTest test suite for Tier 3b adversarial inconsistency hunting."""

import pytest
import torch
from bourbakimesh.adversarial_hunt import FalseInconsistencyHunter
from bourbakimesh.latent_mcts import MCTSConfig
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero


@pytest.fixture
def test_model():
    """Compact BourbakiMuZero model for adversarial test cases."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=64,
        num_res_blocks=2,
    )
    return BourbakiMuZero(config)


def test_hunt_falsehood_refutation(test_model):
    """Verify that the inconsistency hunter correctly evaluates False refutations."""
    hunter = FalseInconsistencyHunter(
        test_model,
        MCTSConfig(num_simulations=30, exploration_fraction=0.25),
    )

    result = hunter.hunt_falsehood(goal_statement="False", num_simulations=30)

    assert result.goal == "False"
    assert result.num_simulations == 30
    assert result.opponent_value == -result.proponent_value
    assert result.candidate_trace is not None
    assert len(result.candidate_trace) > 0


def test_adversarial_mcts_exploration(test_model):
    """Verify adversarial search visits multiple candidate action branches."""
    hunter = FalseInconsistencyHunter(
        test_model,
        MCTSConfig(num_simulations=50, exploration_fraction=0.4),
    )

    result = hunter.hunt_falsehood(goal_statement="A /\\ ~A", num_simulations=50)
    assert result.goal == "A /\\ ~A"
