"""Tier 3b Adversarial Inconsistency Hunter targeting False (Contradiction)."""

from __future__ import annotations
from dataclasses import dataclass
from typing import Optional
import numpy as np
import torch
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import BourbakiMuZero


@dataclass
class HuntResult:
    """Outcome of an adversarial refutation search."""

    goal: str
    num_simulations: int
    proponent_value: float
    opponent_value: float
    refuted: bool
    candidate_trace: Optional[list[int]] = None


class FalseInconsistencyHunter:
    """Adversarial refutation search engine hunting for potential False inconsistencies."""

    def __init__(
        self,
        model: BourbakiMuZero,
        mcts_config: Optional[MCTSConfig] = None,
    ) -> None:
        self.model = model
        self.config = mcts_config or MCTSConfig(
            num_simulations=100,
            exploration_fraction=0.3,
            c_puct=1.5,
        )
        self.mcts = LatentMCTS(model, self.config)

    def hunt_falsehood(
        self,
        goal_statement: str = "False",
        num_simulations: Optional[int] = None,
    ) -> HuntResult:
        """Execute deep adversarial MCTS search against a false or contradictory proposition."""
        sims = num_simulations or self.config.num_simulations
        feature_dim = self.model.config.feature_dim

        # Representation of contradictory proposition
        contradiction_obs = torch.zeros(1, feature_dim, dtype=torch.float32)
        contradiction_obs[0, 0] = -1.0  # False marker embedding

        # Execute MCTS search from Proponent (+1) perspective
        policy = self.mcts.search(
            contradiction_obs,
            current_player=1,
            num_simulations=sims,
            is_latent=False,
        )

        with torch.no_grad():
            s0, _, prop_value_tensor = self.model.initial_inference(contradiction_obs)
            prop_val = float(prop_value_tensor[0].item())
            opp_val = -prop_val

        # Refutation criteria: Opponent dominates or Proponent value is negative
        refuted = opp_val >= prop_val or prop_val <= 0.0

        candidate_actions = [int(np.argmax(policy))]

        return HuntResult(
            goal=goal_statement,
            num_simulations=sims,
            proponent_value=prop_val,
            opponent_value=opp_val,
            refuted=refuted,
            candidate_trace=candidate_actions,
        )
