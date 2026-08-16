"""Smoke tests for BourbakiMesh Python modules."""

import torch
import networkx as nx
import numpy as np
from bourbakimesh.mcts.latent_mcts import LatentMCTS, MCTSConfig, SearchNode
from bourbakimesh.dynamics.arena_game import (
    ArenaState,
    DialogueMove,
    DialoguePolarity,
    MoveKind,
)


def test_imports_and_torch():
    """Verify PyTorch and numpy operations work cleanly."""
    x = torch.randn(4, 4)
    y = x @ x.T
    assert y.shape == (4, 4)
    assert isinstance(y.numpy(), np.ndarray)


def test_arena_state_and_graph():
    """Verify game-semantic arena moves and justification graph construction."""
    state = ArenaState()
    assert state.current_polarity == DialoguePolarity.OPPONENT

    m1 = DialogueMove(
        move_id=0,
        polarity=DialoguePolarity.OPPONENT,
        kind=MoveKind.QUESTION,
        label="Q: A -> B",
    )
    state.add_move(m1)
    assert state.current_polarity == DialoguePolarity.PROPONENT

    m2 = DialogueMove(
        move_id=1,
        polarity=DialoguePolarity.PROPONENT,
        kind=MoveKind.ASSERTION,
        label="A: B",
        target_move_id=0,
    )
    state.add_move(m2)

    g = state.to_graph()
    assert isinstance(g, nx.DiGraph)
    assert g.number_of_nodes() == 2
    assert g.has_edge(1, 0)


def test_latent_mcts_smoke():
    """Verify Latent MCTS returns valid probability distribution."""
    config = MCTSConfig(num_simulations=50)
    mcts = LatentMCTS(config)
    state = torch.randn(1, 32)
    action_dist = mcts.search(state, num_actions=5)
    assert len(action_dist) == 5
    assert np.isclose(action_dist.sum(), 1.0, atol=1e-4)
