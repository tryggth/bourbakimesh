"""PyTest test suite for semantic tableau solver and dialogue seed corpus generator."""

import os
import tempfile
import json
import pytest
from bourbakimesh.bootstrap import (
    Atom,
    And,
    Implies,
    Not,
    Or,
    TableauSolver,
    TableauToDialogueTranspiler,
    SeedCorpusGenerator,
)
from bourbakimesh.self_play import ReplayBuffer


def test_tableau_solver_tautologies():
    """Verify semantic tableau solver proves core propositional tautologies."""
    solver = TableauSolver()

    # 1. Identity: A -> A
    a = Atom("A")
    id_f = Implies(a, a)
    t1 = solver.prove(id_f)
    assert t1 is not None
    assert t1.closed

    # 2. Weakening: A -> B -> A
    b = Atom("B")
    weak_f = Implies(a, Implies(b, a))
    t2 = solver.prove(weak_f)
    assert t2 is not None
    assert t2.closed

    # 3. Double Negation elimination helper: (A ∨ ¬A)
    em_f = Or(a, Not(a))
    t3 = solver.prove(em_f)
    assert t3 is not None
    assert t3.closed


def test_transpiler_strategy_and_trace():
    """Verify tableau-to-dialogue transpilation emits compliant StrategyTree and PlayTrace dicts."""
    solver = TableauSolver()
    transpiler = TableauToDialogueTranspiler()

    a = Atom("A")
    b = Atom("B")
    formula = Implies(a, Implies(b, a))

    tableau = solver.prove(formula)
    assert tableau is not None

    strategy_dict = transpiler.transpile_to_strategy(formula, tableau)
    assert "root" in strategy_dict
    root = strategy_dict["root"]
    assert root["current_move"]["player"] == "Proponent"
    assert len(root["children"]) > 0

    trace_dict = transpiler.transpile_to_play_trace(formula, tableau)
    assert "moves" in trace_dict
    moves = trace_dict["moves"]
    assert len(moves) >= 3
    assert moves[0]["player"] == "Proponent"
    assert moves[1]["player"] == "Opponent"


def test_seed_corpus_generator_and_buffer():
    """Verify batch tautology synthesis and ReplayBuffer population."""
    generator = SeedCorpusGenerator()
    buffer = ReplayBuffer(capacity=100)

    inserted = generator.populate_replay_buffer(buffer, count=40, feature_dim=16, action_space_size=8)
    assert inserted == 40
    assert len(buffer) == 40

    batch = buffer.sample_batch(batch_size=8)
    assert batch["states"].shape == (8, 16)
    assert batch["target_policies"].shape == (8, 8)
    assert batch["target_values"].shape == (8, 1)


def test_export_dataset_json():
    """Verify exporting synthesized seed strategies to JSON format."""
    generator = SeedCorpusGenerator()

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        generator.export_dataset_json(tmp_path, count=10)
        assert os.path.exists(tmp_path)
        with open(tmp_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert len(data) == 10
        assert all("root" in item for item in data)
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)
