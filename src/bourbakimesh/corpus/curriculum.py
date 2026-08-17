"""Curriculum dataset indexer and topological difficulty manager for Mathlib theorem strategies."""

from __future__ import annotations
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Union
import numpy as np
import torch
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer


@dataclass
class TheoremEntry:
    """Indexed theorem record with topological metrics and associated trajectory."""

    name: str
    type_expr: str
    trace_depth: int
    branch_count: int
    dependency_count: int
    difficulty_score: float
    tier: int
    trajectory: Optional[GameTrajectory] = None


class CurriculumManager:
    """Manages progressive curriculum scheduling and topological complexity scoring."""

    def __init__(
        self,
        alpha: float = 1.0,
        beta: float = 1.5,
        gamma: float = 0.5,
    ) -> None:
        self.alpha = alpha
        self.beta = beta
        self.gamma = gamma
        self.theorems: List[TheoremEntry] = []

    def compute_difficulty(
        self,
        depth: int,
        branches: int,
        dependencies: int,
    ) -> float:
        """Compute topological complexity metric D(tau) = alpha*depth + beta*branches + gamma*deps."""
        return round(
            self.alpha * depth + self.beta * branches + self.gamma * dependencies,
            3,
        )

    def assign_tier(self, difficulty: float) -> int:
        """Partition difficulty score into progressive curriculum tiers."""
        if difficulty <= 4.5:
            return 1  # Tier 1: Elementary identities & single-step tautologies
        elif difficulty <= 7.5:
            return 2  # Tier 2: Multi-step propositional implication
        else:
            return 3  # Tier 3: Complex first-order logic & induction

    def add_theorem(
        self,
        name: str,
        type_expr: str,
        trace_depth: int,
        branch_count: int,
        dependency_count: int = 0,
        trajectory: Optional[GameTrajectory] = None,
    ) -> TheoremEntry:
        """Register a theorem into the curriculum index."""
        diff = self.compute_difficulty(trace_depth, branch_count, dependency_count)
        tier = self.assign_tier(diff)

        entry = TheoremEntry(
            name=name,
            type_expr=type_expr,
            trace_depth=trace_depth,
            branch_count=branch_count,
            dependency_count=dependency_count,
            difficulty_score=diff,
            tier=tier,
            trajectory=trajectory,
        )
        self.theorems.append(entry)
        return entry

    def ingest_corpus_json(self, data_or_path: Union[str, Path, dict, list]) -> int:
        """Ingest decompiled corpus JSON exported from Rust bourbaki-kernel."""
        if isinstance(data_or_path, (str, Path)):
            p = Path(data_or_path)
            if p.exists():
                with open(p, "r", encoding="utf-8") as f:
                    payload = json.load(f)
            else:
                payload = json.loads(str(data_or_path))
        else:
            payload = data_or_path

        theorems_data = payload.get("theorems", payload) if isinstance(payload, dict) else payload

        count = 0
        for item in theorems_data:
            name = item.get("name", "unknown")
            type_expr = item.get("type_expr", item.get("typeExpr", ""))
            depth = item.get("trace_depth", 1)
            branches = item.get("branch_count", 1)
            deps = item.get("dependency_count", 0)

            # Generate synthetic trajectory if none provided
            traj = self._synthesize_trajectory_for_theorem(depth, branches)

            self.add_theorem(
                name=name,
                type_expr=type_expr,
                trace_depth=depth,
                branch_count=branches,
                dependency_count=deps,
                trajectory=traj,
            )
            count += 1

        return count

    def _synthesize_trajectory_for_theorem(
        self,
        depth: int,
        branches: int,
        feature_dim: int = 32,
        action_space_size: int = 16,
    ) -> GameTrajectory:
        """Create a synthetic GameTrajectory representative of the topological tree."""
        traj = GameTrajectory()
        steps = max(2, depth * 2)

        for step in range(steps):
            obs = torch.randn(feature_dim, dtype=torch.float32)
            act = step % action_space_size
            pol = np.zeros(action_space_size, dtype=np.float32)
            pol[act] = 0.8
            pol += 0.2 / action_space_size
            player = 1 if step % 2 == 0 else -1

            traj.states.append(obs)
            traj.actions.append(act)
            traj.policies.append(pol)
            traj.rewards.append(0.1 * step)
            traj.players.append(player)

        traj.terminal_value = 1.0
        return traj

    def get_tier(self, tier: int) -> List[TheoremEntry]:
        """Return all theorem entries belonging to the specified curriculum tier."""
        return [t for t in self.theorems if t.tier == tier]

    def populate_replay_buffer(
        self,
        buffer: ReplayBuffer,
        max_tier: int = 3,
        samples_per_tier: int = 10,
    ) -> int:
        """Stream curriculum-ranked training trajectories into ReplayBuffer."""
        total_ingested = 0
        for tier in range(1, max_tier + 1):
            tier_theorems = self.get_tier(tier)
            for thm in tier_theorems[:samples_per_tier]:
                if thm.trajectory is not None:
                    buffer.push(thm.trajectory)
                    total_ingested += 1
        return total_ingested

    def get_summary(self) -> Dict[str, Any]:
        """Get curriculum partitioning summary statistics."""
        return {
            "total_theorems": len(self.theorems),
            "tier_1_count": len(self.get_tier(1)),
            "tier_2_count": len(self.get_tier(2)),
            "tier_3_count": len(self.get_tier(3)),
            "avg_difficulty": float(np.mean([t.difficulty_score for t in self.theorems]))
            if self.theorems
            else 0.0,
        }
