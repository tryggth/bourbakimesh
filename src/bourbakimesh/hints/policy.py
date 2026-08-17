"""Policy Prior Warping and Lemma Hint Oracle for Latent MCTS."""

from __future__ import annotations
import math
from typing import Dict, List, Optional
import numpy as np
import torch
import torch.nn.functional as F
from pydantic import BaseModel, Field


class PolicyWarperConfig(BaseModel):
    """Configuration for policy prior warping and exploration noise."""

    hint_weight: float = Field(default=0.3, ge=0.0, le=1.0)
    temperature: float = Field(default=1.0, gt=0.0)
    dirichlet_alpha: float = Field(default=0.3, gt=0.0)
    dirichlet_epsilon: float = Field(default=0.0, ge=0.0, le=1.0)


class PolicyWarper:
    """Warps and blends neural policy priors with external game-semantic hints."""

    def __init__(self, config: Optional[PolicyWarperConfig] = None) -> None:
        self.config = config or PolicyWarperConfig()

    @property
    def hint_weight(self) -> float:
        return self.config.hint_weight

    @property
    def temperature(self) -> float:
        return self.config.temperature

    def warp_priors(
        self,
        neural_prior: np.ndarray,
        hint_prior: Optional[np.ndarray] = None,
        temperature: Optional[float] = None,
        hint_weight: Optional[float] = None,
    ) -> np.ndarray:
        """Blend neural root prior with an external hint prior and apply temperature scaling.

        pi_warped(a) = (1 - lambda) * pi_theta(a) + lambda * pi_hint(a)
        """
        neural_arr = np.asarray(neural_prior, dtype=np.float32)
        lam = hint_weight if hint_weight is not None else self.config.hint_weight
        temp = temperature if temperature is not None else self.config.temperature

        # 1. Linear probability blending
        if hint_prior is not None and lam > 0.0:
            hint_arr = np.asarray(hint_prior, dtype=np.float32)
            if hint_arr.shape != neural_arr.shape:
                raise ValueError(
                    f"Shape mismatch: neural_prior shape {neural_arr.shape} vs hint_prior shape {hint_arr.shape}"
                )
            # Ensure valid non-negative probabilities
            hint_norm = np.clip(hint_arr, 0.0, None)
            hint_sum = hint_norm.sum()
            if hint_sum > 0:
                hint_norm = hint_norm / hint_sum
            else:
                hint_norm = np.ones_like(hint_arr) / len(hint_arr)

            blended = (1.0 - lam) * neural_arr + lam * hint_norm
        else:
            blended = neural_arr.copy()

        # 2. Temperature scaling using numerically stable log-softmax
        temp = max(temp, 1e-4)
        if temp != 1.0:
            log_p = np.log(np.clip(blended, 1e-12, 1.0))
            log_p_scaled = (log_p - np.max(log_p)) / temp
            exp_p = np.exp(log_p_scaled)
            exp_sum = exp_p.sum()
            if exp_sum > 0 and not np.isnan(exp_sum):
                scaled = exp_p / exp_sum
            else:
                scaled = np.zeros_like(blended)
                scaled[np.argmax(blended)] = 1.0
        else:
            scaled = blended / blended.sum()

        # 3. Dirichlet exploration noise injection
        if self.config.dirichlet_epsilon > 0.0:
            noise = np.random.dirichlet([self.config.dirichlet_alpha] * len(scaled))
            eps = self.config.dirichlet_epsilon
            scaled = (1.0 - eps) * scaled + eps * noise

        final_sum = scaled.sum()
        if final_sum > 0 and not np.isnan(final_sum):
            scaled = scaled / final_sum
        else:
            scaled = np.ones_like(scaled) / len(scaled)

        return scaled.astype(np.float32)

    def warp_logits(
        self,
        neural_logits: torch.Tensor,
        hint_logits: torch.Tensor,
        hint_weight: Optional[float] = None,
        temperature: Optional[float] = None,
    ) -> torch.Tensor:
        """Warp raw neural policy logits with additive hint logits.

        pi_hint = softmax((neural_logits + lambda * hint_logits) / T)
        """
        lam = hint_weight if hint_weight is not None else self.config.hint_weight
        temp = temperature if temperature is not None else self.config.temperature

        combined = neural_logits + lam * hint_logits
        return F.softmax(combined / max(temp, 1e-4), dim=-1)


class LemmaHintOracle:
    """Mathematical heuristic oracle producing structured action priors from propositions."""

    def __init__(
        self,
        action_space_size: int = 128,
        default_floor: float = 1e-4,
    ) -> None:
        self.action_space_size = action_space_size
        self.default_floor = default_floor
        self._registered_hints: Dict[str, Dict[int, float]] = {}

    def register_lemma_hint(
        self,
        goal_pattern: str,
        target_action: int,
        bonus_weight: float = 1.0,
    ) -> None:
        """Register an explicit heuristic lemma target for a specific theorem pattern."""
        if not (0 <= target_action < self.action_space_size):
            raise ValueError(
                f"Action index {target_action} out of bounds for space {self.action_space_size}"
            )
        if goal_pattern not in self._registered_hints:
            self._registered_hints[goal_pattern] = {}
        self._registered_hints[goal_pattern][target_action] = bonus_weight

    def compute_hint_prior(
        self,
        goal_statement: str,
        active_hypotheses: Optional[List[str]] = None,
    ) -> np.ndarray:
        """Compute heuristic prior distribution over arena dialogue actions for a goal."""
        prior = np.full(self.action_space_size, self.default_floor, dtype=np.float32)
        trimmed = goal_statement.strip()

        # Check explicit registered pattern hints
        for pattern, action_weights in self._registered_hints.items():
            if pattern in trimmed or trimmed == pattern:
                for action, weight in action_weights.items():
                    prior[action] += weight

        # Heuristic 1: Implication / Conditional reasoning
        if "->" in trimmed or "→" in trimmed:
            # Action 1: AttackHypothesis / Pi-introduction
            prior[1] += 2.0
            # Action 2: AxiomDischarge / Hypothesis resolution
            prior[2] += 1.5

        # Heuristic 2: Conjunction / Product reasoning
        if "And" in trimmed or "∧" in trimmed:
            # Action 3: Conjunction Left branch
            prior[3] += 2.0
            # Action 4: Conjunction Right branch
            prior[4] += 2.0

        # Heuristic 3: Atomic Witness / Trivial truth
        if trimmed == "True" or "rfl" in trimmed or "refl" in trimmed or "Eq" in trimmed:
            # Action 0: ProvideWitness (e.g. True.intro, rfl)
            prior[0] += 3.0

        # Heuristic 4: Universal quantification
        if "∀" in trimmed or "forall" in trimmed:
            # Action 5: InstantiateUniversal
            prior[5] += 2.0

        return (prior / prior.sum()).astype(np.float32)
