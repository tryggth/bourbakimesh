"""Latent Monte Carlo Tree Search (MCTS) engine over dialogue game arena states."""

from __future__ import annotations
import math
from typing import Dict, Optional, Tuple
import numpy as np
import torch
import torch.nn.functional as F
from pydantic import BaseModel, Field
from bourbakimesh.models import BourbakiMuZero
from bourbakimesh.hints.policy import PolicyWarper, LemmaHintOracle


class MCTSConfig(BaseModel):
    """Configuration parameters for Latent MCTS search."""

    num_simulations: int = Field(default=100, ge=1)
    c_puct: float = Field(default=1.25, gt=0.0)
    dirichlet_alpha: float = Field(default=0.3, gt=0.0)
    exploration_fraction: float = Field(default=0.25, ge=0.0, le=1.0)
    temperature: float = Field(default=1.0, ge=0.0)
    discount: float = Field(default=0.99, ge=0.0, le=1.0)


class Node:
    """A search tree node in the latent dialogue arena."""

    def __init__(
        self,
        prior: float = 0.0,
        parent: Optional[Node] = None,
        action: Optional[int] = None,
        player: int = 1,
    ) -> None:
        self.prior: float = prior
        self.parent: Optional[Node] = parent
        self.action: Optional[int] = action
        self.player: int = player  # +1 for Proponent (P), -1 for Opponent (O)
        self.children: Dict[int, Node] = {}
        self.visit_count: int = 0
        self.value_sum: float = 0.0
        self.reward: float = 0.0
        self.latent_state: Optional[torch.Tensor] = None

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

    def select_child(self, c_puct: float) -> Tuple[int, Node]:
        """Select child with highest PUCT score."""
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

    def expand(
        self,
        action_priors: Dict[int, float],
        latent_state: torch.Tensor,
        next_player: int,
        reward: float = 0.0,
    ) -> None:
        """Expand node with child action priors and latent state."""
        self.latent_state = latent_state
        self.reward = reward
        for action, prior in action_priors.items():
            self.children[action] = Node(
                prior=prior,
                parent=self,
                action=action,
                player=next_player,
            )

    def backpropagate(self, value: float) -> None:
        """Backpropagate value estimate up the tree with perspective inversion."""
        current: Optional[Node] = self
        leaf_player = self.player

        while current is not None:
            current.visit_count += 1
            # Perspective adjustment: positive for same player, negative for opponent
            perspective = 1.0 if current.player == leaf_player else -1.0
            current.value_sum += perspective * value
            current = current.parent


class LatentMCTS:
    """Latent MCTS orchestrator powered by BourbakiMuZero neural dynamics."""

    def __init__(
        self,
        model: BourbakiMuZero,
        config: Optional[MCTSConfig] = None,
    ) -> None:
        self.model = model
        self.config = config or MCTSConfig()

    def search(
        self,
        root_obs_or_latent: torch.Tensor,
        current_player: int = 1,
        num_simulations: Optional[int] = None,
        is_latent: bool = False,
        hint_warper: Optional[PolicyWarper] = None,
        hint_prior: Optional[np.ndarray] = None,
    ) -> np.ndarray:
        """Execute latent MCTS search and return visit count distribution over actions."""
        sims = num_simulations or self.config.num_simulations
        c_puct = self.config.c_puct
        action_space_size = self.model.config.action_space_size

        if root_obs_or_latent.dim() == 1:
            root_input = root_obs_or_latent.unsqueeze(0)
        else:
            root_input = root_obs_or_latent

        self.model.eval()
        with torch.no_grad():
            if is_latent:
                latent_root = root_input
                policy_logits, value_tensor = self.model.prediction(latent_root)
            else:
                latent_root, policy_logits, value_tensor = self.model.initial_inference(root_input)

            raw_priors = F.softmax(policy_logits[0], dim=-1).cpu().numpy()

            # Apply PolicyWarper if provided, otherwise default Dirichlet noise
            if hint_warper is not None:
                priors = hint_warper.warp_priors(raw_priors, hint_prior)
            elif self.config.exploration_fraction > 0:
                noise = np.random.dirichlet([self.config.dirichlet_alpha] * action_space_size)
                eps = self.config.exploration_fraction
                priors = (1 - eps) * raw_priors + eps * noise
            else:
                priors = raw_priors

            root = Node(player=current_player)
            action_priors = {a: float(priors[a]) for a in range(action_space_size)}
            root.expand(action_priors, latent_root, next_player=-current_player)
            root.backpropagate(float(value_tensor[0].item()))

            for _ in range(sims):
                node = root
                search_path = [node]

                # Selection: traverse down tree until reaching an unexpanded node
                while node.children and all(c.visit_count > 0 for c in node.children.values()):
                    action, node = node.select_child(c_puct)
                    search_path.append(node)

                # Select an unvisited child if available
                if node.children:
                    unvisited = [c for c in node.children.values() if c.visit_count == 0]
                    if unvisited:
                        node = unvisited[0]
                        search_path.append(node)

                # Recurrent inference / expansion
                parent = node.parent
                if parent is not None and parent.latent_state is not None:
                    action_tensor = torch.tensor([node.action], dtype=torch.long)
                    next_latent, reward, policy_logits, value_tensor = self.model.recurrent_inference(
                        parent.latent_state, action_tensor
                    )
                    child_priors = F.softmax(policy_logits[0], dim=-1).cpu().numpy()
                    action_priors_dict = {a: float(child_priors[a]) for a in range(action_space_size)}
                    next_player = -node.player
                    node.expand(action_priors_dict, next_latent, next_player, float(reward[0].item()))
                    leaf_value = float(value_tensor[0].item())
                else:
                    leaf_value = 0.0

                node.backpropagate(leaf_value)

            # Compute visit count distribution
            visit_counts = np.array(
                [root.children[a].visit_count if a in root.children else 0 for a in range(action_space_size)],
                dtype=np.float32,
            )

            total_visits = visit_counts.sum()
            if total_visits > 0:
                if self.config.temperature == 0:
                    best_action = np.argmax(visit_counts)
                    policy = np.zeros(action_space_size, dtype=np.float32)
                    policy[best_action] = 1.0
                    return policy
                else:
                    powered = visit_counts ** (1.0 / max(self.config.temperature, 1e-4))
                    return powered / powered.sum()
            else:
                return np.ones(action_space_size, dtype=np.float32) / action_space_size

    def search_with_hints(
        self,
        root_obs_or_latent: torch.Tensor,
        goal_statement: str,
        oracle: Optional[LemmaHintOracle] = None,
        warper: Optional[PolicyWarper] = None,
        current_player: int = 1,
        num_simulations: Optional[int] = None,
        is_latent: bool = False,
    ) -> np.ndarray:
        """Execute latent MCTS search using automated oracle hint prior computation."""
        warper = warper or PolicyWarper()
        oracle = oracle or LemmaHintOracle(action_space_size=self.model.config.action_space_size)
        hint_prior = oracle.compute_hint_prior(goal_statement)
        return self.search(
            root_obs_or_latent=root_obs_or_latent,
            current_player=current_player,
            num_simulations=num_simulations,
            is_latent=is_latent,
            hint_warper=warper,
            hint_prior=hint_prior,
        )

