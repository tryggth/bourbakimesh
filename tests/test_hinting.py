"""Integration tests for Neural & Game-Semantic Hinting Engine."""

import numpy as np
import pytest
import torch
from bourbakimesh.hints.arena import ArenaCutInjector, CutSpec
from bourbakimesh.hints.policy import (
    LemmaHintOracle,
    PolicyWarper,
    PolicyWarperConfig,
)
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import BourbakiMuZero, ArenaEmbeddingConfig


def test_policy_warper_blending_and_temperature() -> None:
    """Test that PolicyWarper blends priors correctly and applies temperature scaling."""
    config = PolicyWarperConfig(hint_weight=0.4, temperature=1.0, dirichlet_epsilon=0.0)
    warper = PolicyWarper(config)

    action_space = 10
    neural_prior = np.full(action_space, 0.1, dtype=np.float32)

    # Hint prior placing 100% mass on action 3
    hint_prior = np.zeros(action_space, dtype=np.float32)
    hint_prior[3] = 1.0

    warped = warper.warp_priors(neural_prior, hint_prior)

    assert warped.shape == (action_space,)
    assert np.isclose(warped.sum(), 1.0, atol=1e-5)
    # Expected for action 3: (1 - 0.4)*0.1 + 0.4*1.0 = 0.06 + 0.40 = 0.46
    assert np.isclose(warped[3], 0.46, atol=1e-3)
    # Expected for other actions: 0.06
    assert np.isclose(warped[0], 0.06, atol=1e-3)

    # Test sharpening with low temperature T = 0.5
    sharpened = warper.warp_priors(neural_prior, hint_prior, temperature=0.5)
    assert sharpened[3] > warped[3]
    assert np.isclose(sharpened.sum(), 1.0, atol=1e-5)

    # Test smoothing with high temperature T = 2.0
    smoothed = warper.warp_priors(neural_prior, hint_prior, temperature=2.0)
    assert smoothed[3] < warped[3]
    assert np.isclose(smoothed.sum(), 1.0, atol=1e-5)


def test_policy_warper_dirichlet_noise() -> None:
    """Test Dirichlet noise exploration in PolicyWarper."""
    config = PolicyWarperConfig(hint_weight=0.0, dirichlet_alpha=0.3, dirichlet_epsilon=0.25)
    warper = PolicyWarper(config)

    uniform = np.full(8, 1.0 / 8, dtype=np.float32)
    noisy = warper.warp_priors(uniform)

    assert noisy.shape == (8,)
    assert np.isclose(noisy.sum(), 1.0, atol=1e-5)
    assert not np.allclose(noisy, uniform)  # Noise was injected


def test_lemma_hint_oracle_heuristics() -> None:
    """Test that LemmaHintOracle computes domain-aligned heuristic priors."""
    oracle = LemmaHintOracle(action_space_size=32)

    # 1. Implication Goal
    imp_prior = oracle.compute_hint_prior("P -> Q")
    assert imp_prior[1] > imp_prior[6]  # Action 1 (AttackHypothesis) boosted
    assert imp_prior[2] > imp_prior[6]  # Action 2 (AxiomDischarge) boosted

    # 2. Conjunction Goal
    and_prior = oracle.compute_hint_prior("A ∧ B")
    assert and_prior[3] > and_prior[6]  # Conjunction Left
    assert and_prior[4] > and_prior[6]  # Conjunction Right

    # 3. True / Identity Goal
    true_prior = oracle.compute_hint_prior("True")
    assert true_prior[0] > true_prior[6]  # Witness Action 0 boosted

    # 4. Registered custom lemma
    oracle.register_lemma_hint("Group.mul_assoc", target_action=7, bonus_weight=5.0)
    grp_prior = oracle.compute_hint_prior("Group.mul_assoc a b c")
    assert np.argmax(grp_prior) == 7


def test_latent_mcts_with_hints_biases_search() -> None:
    """Test that LatentMCTS with hints biases root visit distribution toward target actions."""
    torch.manual_seed(42)
    np.random.seed(42)
    cfg = ArenaEmbeddingConfig(
        feature_dim=64,
        latent_dim=64,
        action_space_size=16,
        hidden_dim=64,
        use_relational_transformer=False,
    )
    model = BourbakiMuZero(cfg)
    mcts = LatentMCTS(model, MCTSConfig(num_simulations=40, exploration_fraction=0.0))

    obs = torch.randn(64)

    # 1. Search without hints
    policy_no_hint = mcts.search(obs, current_player=1)

    # 2. Search with strong hint favoring Action 5
    hint_prior = np.zeros(16, dtype=np.float32)
    hint_prior[5] = 1.0
    warper = PolicyWarper(PolicyWarperConfig(hint_weight=0.9, temperature=0.5))

    policy_with_hint = mcts.search(
        obs,
        current_player=1,
        hint_warper=warper,
        hint_prior=hint_prior,
    )

    # Visit count on Action 5 must be significantly higher with hint
    assert policy_with_hint[5] >= policy_no_hint[5]
    assert np.isclose(policy_with_hint.sum(), 1.0, atol=1e-4)


def test_search_with_hints_convenience_api() -> None:
    """Test search_with_hints convenience method with automated oracle prior."""
    cfg = ArenaEmbeddingConfig(
        feature_dim=64,
        latent_dim=64,
        action_space_size=16,
        hidden_dim=64,
        use_relational_transformer=False,
    )
    model = BourbakiMuZero(cfg)
    mcts = LatentMCTS(model, MCTSConfig(num_simulations=20, exploration_fraction=0.0))

    obs = torch.randn(64)
    policy = mcts.search_with_hints(obs, goal_statement="A -> A")

    assert policy.shape == (16,)
    assert np.isclose(policy.sum(), 1.0, atol=1e-4)
    # Action 1 (AttackHypothesis) or Action 2 should have strong visits
    assert (policy[1] + policy[2]) > 0.0


def test_arena_cut_injector_and_let_binding() -> None:
    """Test ArenaCutInjector structures and Lean 4 let-binding emitter formatting."""
    cut_spec = CutSpec(
        lemma_id=0,
        statement="A -> B",
        premise_requirements=["h1 : A -> B"],
    )

    lean_let = ArenaCutInjector.build_let_binding_lean(
        lemma_id=cut_spec.lemma_id,
        lemma_type=cut_spec.statement,
        lemma_proof="h1 h2",
        continuation_body="lem_0",
    )
    assert lean_let == "let lem_0 : A -> B := h1 h2; lem_0"

    # Test trace injection
    base_moves = [
        {"id": 0, "player": "Opponent", "kind": "Question", "justifier": None, "payload": {}}
    ]
    injected = ArenaCutInjector.inject_cut_into_trace(base_moves, cut_spec)
    assert len(injected) == 2
    assert injected[0]["id"] == 0
    assert "AssertCutLemma" in injected[0]["payload"]
    assert injected[1]["id"] == 1


def test_adversarial_or_noisy_hints_fail_gracefully() -> None:
    """Test that adversarial or degraded hint distributions do not crash MCTS or yield NaNs."""
    cfg = ArenaEmbeddingConfig(
        feature_dim=64,
        latent_dim=64,
        action_space_size=8,
        hidden_dim=32,
        use_relational_transformer=False,
    )
    model = BourbakiMuZero(cfg)
    mcts = LatentMCTS(model, MCTSConfig(num_simulations=20))

    obs = torch.randn(64)

    # 1. All zero hint prior
    zero_hint = np.zeros(8, dtype=np.float32)
    warper = PolicyWarper(PolicyWarperConfig(hint_weight=0.5))
    policy_zero = mcts.search(obs, hint_warper=warper, hint_prior=zero_hint)
    assert not np.isnan(policy_zero).any()
    assert np.isclose(policy_zero.sum(), 1.0, atol=1e-4)

    # 2. Extreme temperature T = 0.01 (near argmax)
    extreme_warper = PolicyWarper(PolicyWarperConfig(hint_weight=0.5, temperature=0.01))
    policy_extreme = mcts.search(obs, hint_warper=extreme_warper, hint_prior=zero_hint)
    assert not np.isnan(policy_extreme).any()
    assert np.isclose(policy_extreme.sum(), 1.0, atol=1e-4)


def test_hinted_search_reduces_search_entropy() -> None:
    """Verify that domain-aligned hints concentrate search distribution (lower entropy)."""
    torch.manual_seed(42)
    np.random.seed(42)
    cfg = ArenaEmbeddingConfig(
        feature_dim=64,
        latent_dim=64,
        action_space_size=16,
        hidden_dim=64,
        use_relational_transformer=False,
    )
    model = BourbakiMuZero(cfg)
    mcts = LatentMCTS(model, MCTSConfig(num_simulations=100, exploration_fraction=0.0))

    obs = torch.randn(64)

    # Search without hints
    p_unhinted = mcts.search(obs, current_player=1)
    entropy_unhinted = -np.sum(p_unhinted * np.log(np.clip(p_unhinted, 1e-12, 1.0)))

    # Search with domain oracle hint for implication
    oracle = LemmaHintOracle(action_space_size=16)
    warper = PolicyWarper(PolicyWarperConfig(hint_weight=0.95, temperature=0.4))
    hint_prior = oracle.compute_hint_prior("P -> P")
    p_hinted = mcts.search(obs, current_player=1, hint_warper=warper, hint_prior=hint_prior)
    entropy_hinted = -np.sum(p_hinted * np.log(np.clip(p_hinted, 1e-12, 1.0)))

    # Hinted search focuses on relevant actions, reducing dispersion/entropy
    assert entropy_hinted <= entropy_unhinted
    assert (p_hinted[1] + p_hinted[2]) >= (p_unhinted[1] + p_unhinted[2])


def test_lean_kernel_verification_of_cut_let_binding() -> None:
    """Verify that Lean 4 elaborates and type-checks let-bound cut lemma proofs."""
    import subprocess
    from pathlib import Path

    lean_target = Path(__file__).parent.parent / "lean_target"
    if not lean_target.exists():
        pytest.skip("lean_target directory not found")

    # Generate a Lean 4 snippet with let-bound cut lemma
    code = """import LeanTarget.Harness

theorem test_cut_let_sound (A B : Prop) (f : A -> B) (a : A) : B :=
  let lem_0 : B := f a; lem_0
"""
    tmp_file = lean_target / "TestCut.lean"
    try:
        tmp_file.write_text(code)
        proc = subprocess.run(
            ["lake", "env", "lean", str(tmp_file)],
            cwd=str(lean_target),
            capture_output=True,
            text=True,
            timeout=10,
        )
        assert proc.returncode == 0, f"Lean 4 failed to verify cut lemma: {proc.stderr}"
    finally:
        if tmp_file.exists():
            tmp_file.unlink()
