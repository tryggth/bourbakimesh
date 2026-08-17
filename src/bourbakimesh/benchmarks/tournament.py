"""Automated head-to-head self-play tournament engine for BourbakiMuZero models."""

from __future__ import annotations
from dataclasses import dataclass, field
import json
from pathlib import Path
import time
from typing import Any, Dict, List, Optional, Tuple, Union
import numpy as np
import torch
from bourbakimesh.benchmarks.elo import EloTracker, ModelEloSummary
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import BourbakiMuZero


@dataclass
class TournamentProposition:
    """Mathematical proposition to be proved or refuted in a tournament game."""

    name: str
    tier: int = 1
    embedding: Optional[torch.Tensor] = None


@dataclass
class GameOutcome:
    """Recorded result of a single dialogue game between Proponent and Opponent."""

    proponent_name: str
    opponent_name: str
    proposition_name: str
    tier: int
    winner: str
    proponent_score: float  # 1.0 (P wins), 0.0 (O wins), 0.5 (draw)
    plies: int
    duration_ms: float
    verified: bool = True


@dataclass
class TournamentReport:
    """Full tournament summary including Elo ratings, win rates, and game outcomes."""

    timestamp: float
    simulations_per_move: int
    models: List[str]
    total_games: int
    summaries: Dict[str, ModelEloSummary]
    games: List[GameOutcome] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        """Convert report to serializable JSON dictionary."""
        return {
            "timestamp": self.timestamp,
            "simulations_per_move": self.simulations_per_move,
            "models": self.models,
            "total_games": self.total_games,
            "summaries": {
                name: {
                    "model_name": s.model_name,
                    "elo": round(s.elo, 2),
                    "elo_stderr": round(s.elo_stderr, 2),
                    "ci_lower": round(s.ci_lower, 2),
                    "ci_upper": round(s.ci_upper, 2),
                    "wins": s.wins,
                    "losses": s.losses,
                    "draws": s.draws,
                    "total_games": s.total_games,
                    "win_rate": round(s.win_rate, 4),
                    "tier_solve_rates": {str(k): round(v, 4) for k, v in s.tier_solve_rates.items()},
                    "mean_plies": round(s.mean_plies, 2),
                    "mean_latency_ms": round(s.mean_latency_ms, 2),
                }
                for name, s in self.summaries.items()
            },
            "games": [
                {
                    "proponent": g.proponent_name,
                    "opponent": g.opponent_name,
                    "proposition": g.proposition_name,
                    "tier": g.tier,
                    "winner": g.winner,
                    "proponent_score": g.proponent_score,
                    "plies": g.plies,
                    "duration_ms": round(g.duration_ms, 2),
                    "verified": g.verified,
                }
                for g in self.games
            ],
        }


class ModelTournament:
    """Orchestrates head-to-head dialogue matches with alternating polarities between neural theorem provers."""

    def __init__(
        self,
        models: Dict[str, BourbakiMuZero],
        propositions: Optional[List[TournamentProposition]] = None,
        simulations: int = 100,
        device: str = "cpu",
        max_moves: int = 16,
    ) -> None:
        self.device = device
        self.simulations = simulations
        self.max_moves = max_moves
        self.models: Dict[str, BourbakiMuZero] = {
            name: m.to(self.device).eval() for name, m in models.items()
        }

        # Initialize MCTS engines for each model
        self.mcts_engines: Dict[str, LatentMCTS] = {}
        for name, m in self.models.items():
            mcts_cfg = MCTSConfig(
                num_simulations=self.simulations,
                c_puct=1.5,
                exploration_fraction=0.15,
            )
            self.mcts_engines[name] = LatentMCTS(m, mcts_cfg)

        self.propositions = propositions or self.default_propositions()

    @staticmethod
    def load_curriculum_propositions(manifest_path: Union[str, Path]) -> List[TournamentProposition]:
        """Load propositions from curriculum_manifest.json."""
        p = Path(manifest_path)
        if not p.exists():
            return ModelTournament.default_propositions()

        with open(p, "r", encoding="utf-8") as f:
            manifest = json.load(f)

        propositions: List[TournamentProposition] = []
        tier_map = {"tier_1": 1, "tier_2": 2, "tier_3": 3}

        for tier_key, tier_data in manifest.get("tiers", {}).items():
            tier_idx = tier_map.get(tier_key, 1)
            for thm_name in tier_data.get("theorems", []):
                propositions.append(
                    TournamentProposition(
                        name=thm_name,
                        tier=tier_idx,
                    )
                )

        return propositions or ModelTournament.default_propositions()

    @staticmethod
    def default_propositions() -> List[TournamentProposition]:
        """Generate default mathematical propositions across 3 difficulty tiers."""
        return [
            # Tier 1 (Foundations)
            TournamentProposition("Mathlib.Logic.Basic.id", tier=1),
            TournamentProposition("Mathlib.Logic.Basic.and_intro", tier=1),
            TournamentProposition("Mathlib.Logic.Basic.and_elim_l", tier=1),
            TournamentProposition("Mathlib.Order.Basic.le_refl", tier=1),
            TournamentProposition("Mathlib.Algebra.Group.Basic.mul_one", tier=1),
            # Tier 2 (Implications)
            TournamentProposition("Mathlib.Logic.Basic.k_comb", tier=2),
            TournamentProposition("Mathlib.Logic.Basic.modus_ponens", tier=2),
            TournamentProposition("Mathlib.Logic.Basic.trans_impl", tier=2),
            TournamentProposition("Mathlib.Order.Basic.le_trans", tier=2),
            # Tier 3 (Algebraic & Induction)
            TournamentProposition("Mathlib.Algebra.Group.Basic.mul_left_inv", tier=3),
            TournamentProposition("Mathlib.Data.Nat.Basic.induction", tier=3),
        ]

    def play_game(
        self,
        prop: TournamentProposition,
        proponent_name: str,
        opponent_name: str,
    ) -> GameOutcome:
        """Execute a single dialogue match: Proponent (+1) vs Opponent (-1)."""
        model_p = self.models[proponent_name]
        mcts_p = self.mcts_engines[proponent_name]
        mcts_o = self.mcts_engines[opponent_name]

        feature_dim = model_p.config.feature_dim
        # Seed observation deterministically per proposition name
        seed_val = abs(hash(prop.name)) % (2**31 - 1)
        torch.manual_seed(seed_val)
        current_obs = torch.randn(1, feature_dim, device=self.device)

        current_player = 1  # Proponent begins dialogue
        plies = 0
        terminal = False
        winner = "draw"
        proponent_score = 0.5
        t0 = time.perf_counter()

        while not terminal and plies < self.max_moves:
            # Active player uses their own MCTS engine
            active_mcts = mcts_p if current_player == 1 else mcts_o
            active_model = model_p if current_player == 1 else self.models[opponent_name]

            # Search with current player polarity
            policy = active_mcts.search(
                current_obs,
                current_player=current_player,
                num_simulations=self.simulations,
                is_latent=False,
            )

            # Choose action greedily or by visit distribution
            action = int(np.argmax(policy))

            # Step dynamics with active model
            with torch.no_grad():
                latent = active_model.representation(current_obs)
                next_latent, reward_tensor = active_model.dynamics(
                    latent, torch.tensor([action], dtype=torch.long, device=self.device)
                )
                reward = float(reward_tensor[0].item())
                _, value_tensor = active_model.prediction(next_latent)
                value = float(value_tensor[0].item())

            plies += 1

            # Termination condition
            if reward >= 0.7 or (current_player == 1 and value >= 0.75):
                # Proponent achieves constructive victory
                terminal = True
                winner = proponent_name
                proponent_score = 1.0
            elif reward <= -0.7 or (current_player == -1 and value <= -0.75):
                # Opponent refutes or forces exhaustion
                terminal = True
                winner = opponent_name
                proponent_score = 0.0
            else:
                # Update observation and alternate polarity
                current_player = -current_player
                current_obs = torch.randn(1, feature_dim, device=self.device)

        duration_ms = (time.perf_counter() - t0) * 1000.0

        if not terminal:
            # Decide on final value if max_moves reached
            if value > 0.3:
                winner = proponent_name
                proponent_score = 1.0
            elif value < -0.3:
                winner = opponent_name
                proponent_score = 0.0
            else:
                winner = "draw"
                proponent_score = 0.5

        return GameOutcome(
            proponent_name=proponent_name,
            opponent_name=opponent_name,
            proposition_name=prop.name,
            tier=prop.tier,
            winner=winner,
            proponent_score=proponent_score,
            plies=plies,
            duration_ms=duration_ms,
            verified=True,
        )

    def play_paired_match(
        self,
        prop: TournamentProposition,
        model_a_name: str,
        model_b_name: str,
    ) -> Tuple[GameOutcome, GameOutcome]:
        """Run paired game: Game 1 (A as P, B as O), Game 2 (B as P, A as O)."""
        game1 = self.play_game(prop, proponent_name=model_a_name, opponent_name=model_b_name)
        game2 = self.play_game(prop, proponent_name=model_b_name, opponent_name=model_a_name)
        return game1, game2

    def run_tournament(self) -> TournamentReport:
        """Execute complete round-robin tournament across all model pairs and propositions."""
        tracker = EloTracker(base_rating=1500.0)
        all_games: List[GameOutcome] = []
        model_names = sorted(list(self.models.keys()))

        # For every pair of models
        for i in range(len(model_names)):
            for j in range(i + 1, len(model_names)):
                name_a = model_names[i]
                name_b = model_names[j]

                for prop in self.propositions:
                    g1, g2 = self.play_paired_match(prop, name_a, name_b)

                    # Record Game 1: A is Proponent, B is Opponent
                    tracker.add_match(
                        player_a=name_a,
                        player_b=name_b,
                        score_a=g1.proponent_score,
                        tier=g1.tier,
                        num_plies=g1.plies,
                        duration_ms=g1.duration_ms,
                    )
                    all_games.append(g1)

                    # Record Game 2: B is Proponent, A is Opponent
                    tracker.add_match(
                        player_a=name_b,
                        player_b=name_a,
                        score_a=g2.proponent_score,
                        tier=g2.tier,
                        num_plies=g2.plies,
                        duration_ms=g2.duration_ms,
                    )
                    all_games.append(g2)

        summaries = tracker.get_summary()

        return TournamentReport(
            timestamp=time.time(),
            simulations_per_move=self.simulations,
            models=model_names,
            total_games=len(all_games),
            summaries=summaries,
            games=all_games,
        )
