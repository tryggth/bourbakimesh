"""Latent Monte Carlo Tree Search engine over dialogue game arena states."""

from __future__ import annotations
import math
from typing import Dict, List, Optional
import numpy as np
import torch
from pydantic import BaseModel, Field


class MCTSConfig(BaseModel):
    """Configuration for Latent MCTS search."""

    num_simulations: int = Field(default=800, ge=1)
    c_puct: float = Field(default=1.25, gt=0.0)
    dirichlet_alpha: float = Field(default=0.3, gt=0.0)
    exploration_fraction: float = Field(default=0.25, ge=0.0, le=1.0)
    temperature: float = Field(default=1.0, ge=0.0)


class SearchNode:
    """A node in the latent dialogue game search tree."""

    def __init__(
        self,
        prior: float = 0.0,
        parent: Optional[SearchNode] = None,
        action: Optional[int] = None,
    ) -> None:
        self.prior: float = prior
        self.parent: Optional[SearchNode] = parent
        self.action: Optional[int] = action
        self.children: Dict[int, SearchNode] = {}
        self.visit_count: int = 0
        self.value_sum: float = 0.0
        self.hidden_state: Optional[torch.Tensor] = None

    @property
    def value(self) -> float:
        """Mean action value Q(s, a)."""
        if self.visit_count == 0:
            return 0.0
        return self.value_sum / self.visit_count

    def ucb_score(self, c_puct: float, total_parent_visits: int) -> float:
        """Compute Upper Confidence Bound for Trees (PUCT) score."""
        u_score = (
            c_puct
            * self.prior
            * (math.sqrt(total_parent_visits) / (1 + self.visit_count))
        )
        return self.value + u_score

    def select_child(self, c_puct: float) -> tuple[int, SearchNode]:
        """Select child node with maximum PUCT score."""
        best_score = -float("inf")
        best_action = -1
        best_child = None
        for action, child in self.children.items():
            score = child.ucb_score(c_puct, self.visit_count)
            if score > best_score:
                best_score = score
                best_action = action
                best_child = child
        if best_child is None:
            raise ValueError("No child nodes available for selection")
        return best_action, best_child

    def expand(self, action_priors: Dict[int, float], hidden_state: torch.Tensor) -> None:
        """Expand node with child action priors and latent state."""
        self.hidden_state = hidden_state
        for action, prior in action_priors.items():
            self.children[action] = SearchNode(prior=prior, parent=self, action=action)

    def backpropagate(self, value: float) -> None:
        """Backpropagate value estimate up the tree."""
        current: Optional[SearchNode] = self
        while current is not None:
            current.visit_count += 1
            current.value_sum += value
            # Alternate value perspective for zero-sum dialogue game
            value = -value
            current = current.parent


class LatentMCTS:
    """Latent MCTS orchestrator for self-play dialogue games."""

    def __init__(self, config: Optional[MCTSConfig] = None) -> None:
        self.config = config or MCTSConfig()

    def search(self, root_state: torch.Tensor, num_actions: int = 10) -> np.ndarray:
        """Perform MCTS search and return visit distribution over actions."""
        root = SearchNode()
        uniform_prior = {a: 1.0 / num_actions for a in range(num_actions)}
        root.expand(uniform_prior, root_state)

        for _ in range(self.config.num_simulations):
            node = root
            # Traverse tree
            while node.children:
                _, node = node.select_child(self.config.c_puct)

            # Simulated evaluation value
            sim_value = float(np.tanh(np.random.randn()))
            node.backpropagate(sim_value)

        # Compute visit count distribution
        visits = np.array(
            [root.children[a].visit_count if a in root.children else 0 for a in range(num_actions)],
            dtype=np.float32,
        )
        total_visits = visits.sum()
        if total_visits > 0:
            return visits / total_visits
        return np.ones(num_actions, dtype=np.float32) / num_actions
