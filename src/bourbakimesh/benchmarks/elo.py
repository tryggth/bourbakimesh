"""Bayesian Elo rating tracker and Bradley-Terry logistic estimator."""

from __future__ import annotations
from dataclasses import dataclass, field
import math
from typing import Any, Dict, List, Optional, Tuple
import numpy as np


@dataclass
class MatchRecord:
    """Individual match result between two models."""

    player_a: str
    player_b: str
    score_a: float  # 1.0 (win A), 0.5 (draw), 0.0 (win B)
    tier: int = 1
    num_plies: int = 0
    duration_ms: float = 0.0


@dataclass
class ModelEloSummary:
    """Summary of Elo rating, confidence intervals, and performance stats for a model."""

    model_name: str
    elo: float
    elo_stderr: float
    ci_lower: float
    ci_upper: float
    wins: int
    losses: int
    draws: int
    total_games: int
    win_rate: float
    tier_solve_rates: Dict[int, float] = field(default_factory=dict)
    mean_plies: float = 0.0
    mean_latency_ms: float = 0.0


class EloTracker:
    """Estimates posterior Elo ratings and confidence intervals using a regularized Bradley-Terry model."""

    def __init__(self, base_rating: float = 1500.0, prior_std: float = 200.0) -> None:
        self.base_rating = base_rating
        self.prior_std = prior_std
        self.matches: List[MatchRecord] = []
        self.models: set[str] = set()

    def add_match(
        self,
        player_a: str,
        player_b: str,
        score_a: float,
        tier: int = 1,
        num_plies: int = 0,
        duration_ms: float = 0.0,
    ) -> None:
        """Record a single match between two players."""
        self.models.add(player_a)
        self.models.add(player_b)
        self.matches.append(
            MatchRecord(
                player_a=player_a,
                player_b=player_b,
                score_a=score_a,
                tier=tier,
                num_plies=num_plies,
                duration_ms=duration_ms,
            )
        )

    @staticmethod
    def expected_score(rating_a: float, rating_b: float) -> float:
        """Calculate Bradley-Terry logistic win probability for Player A vs Player B."""
        diff = rating_b - rating_a
        return 1.0 / (1.0 + math.pow(10.0, diff / 400.0))

    def compute_ratings(self, max_iter: int = 100, tol: float = 1e-4) -> Dict[str, Tuple[float, float]]:
        """Compute Maximum A Posteriori (MAP) Elo ratings and standard errors using Newton-Raphson."""
        model_list = sorted(list(self.models))
        n = len(model_list)
        if n == 0:
            return {}
        if n == 1:
            return {model_list[0]: (self.base_rating, 0.0)}

        model_idx = {name: i for i, name in enumerate(model_list)}
        ratings = np.full(n, self.base_rating, dtype=np.float64)
        beta = math.log(10.0) / 400.0
        prior_var = self.prior_std**2

        for _ in range(max_iter):
            grad = np.zeros(n, dtype=np.float64)
            hessian_diag = np.zeros(n, dtype=np.float64)

            # Prior contribution
            grad -= (ratings - self.base_rating) / prior_var
            hessian_diag -= 1.0 / prior_var

            # Match likelihood contributions
            for match in self.matches:
                i = model_idx[match.player_a]
                j = model_idx[match.player_b]
                p_ij = self.expected_score(ratings[i], ratings[j])
                s_i = match.score_a

                # Gradient
                grad[i] += beta * (s_i - p_ij)
                grad[j] -= beta * (s_i - p_ij)

                # Hessian diagonal
                w = beta * beta * p_ij * (1.0 - p_ij)
                hessian_diag[i] -= w
                hessian_diag[j] -= w

            step = -grad / np.clip(hessian_diag, -1e10, -1e-6)
            ratings += step

            if np.max(np.abs(step)) < tol:
                break

        # Standard error is sqrt(-1 / H_ii)
        stderrs = np.sqrt(np.maximum(0.0, -1.0 / np.clip(hessian_diag, -1e10, -1e-6)))

        results: Dict[str, Tuple[float, float]] = {}
        for name, idx in model_idx.items():
            results[name] = (float(ratings[idx]), float(stderrs[idx]))

        return results

    def get_summary(self) -> Dict[str, ModelEloSummary]:
        """Compute full statistical summary for all models."""
        ratings = self.compute_ratings()
        summaries: Dict[str, ModelEloSummary] = {}

        for model_name in sorted(list(self.models)):
            elo, stderr = ratings.get(model_name, (self.base_rating, 0.0))
            wins = 0
            losses = 0
            draws = 0
            total_games = 0
            total_plies = 0
            total_time_ms = 0.0

            # Tier stats: {tier: (solved, total)}
            tier_stats: Dict[int, List[int]] = {}

            for m in self.matches:
                if m.player_a == model_name:
                    total_games += 1
                    total_plies += m.num_plies
                    total_time_ms += m.duration_ms
                    if m.tier not in tier_stats:
                        tier_stats[m.tier] = [0, 0]
                    tier_stats[m.tier][1] += 1

                    if m.score_a == 1.0:
                        wins += 1
                        tier_stats[m.tier][0] += 1
                    elif m.score_a == 0.5:
                        draws += 1
                    else:
                        losses += 1

                elif m.player_b == model_name:
                    total_games += 1
                    total_plies += m.num_plies
                    total_time_ms += m.duration_ms
                    score_b = 1.0 - m.score_a
                    if m.tier not in tier_stats:
                        tier_stats[m.tier] = [0, 0]
                    tier_stats[m.tier][1] += 1

                    if score_b == 1.0:
                        wins += 1
                        tier_stats[m.tier][0] += 1
                    elif score_b == 0.5:
                        draws += 1
                    else:
                        losses += 1

            win_rate = (wins + 0.5 * draws) / max(1, total_games)
            tier_solve_rates = {
                t: (counts[0] / max(1, counts[1])) for t, counts in tier_stats.items()
            }
            mean_plies = total_plies / max(1, total_games)
            mean_latency_ms = total_time_ms / max(1, total_games)

            summaries[model_name] = ModelEloSummary(
                model_name=model_name,
                elo=elo,
                elo_stderr=stderr,
                ci_lower=elo - 1.96 * stderr,
                ci_upper=elo + 1.96 * stderr,
                wins=wins,
                losses=losses,
                draws=draws,
                total_games=total_games,
                win_rate=win_rate,
                tier_solve_rates=tier_solve_rates,
                mean_plies=mean_plies,
                mean_latency_ms=mean_latency_ms,
            )

        return summaries
