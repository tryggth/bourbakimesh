"""Latent Monte Carlo Tree Search (MCTS) module compatibility layer."""

from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig, Node as SearchNode, Node

__all__ = ["LatentMCTS", "MCTSConfig", "SearchNode", "Node"]
