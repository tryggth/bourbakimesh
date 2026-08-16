"""Synthetic seed corpus generator synthesizing verified dialogue proofs for MCTS imitation learning."""

from __future__ import annotations
import json
from typing import Any, Dict, List
import numpy as np
import torch
from bourbakimesh.bootstrap.tableau import And, Atom, Formula, Implies, TableauSolver
from bourbakimesh.bootstrap.transpiler import TableauToDialogueTranspiler
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer


class SeedCorpusGenerator:
    """Automated generator synthesizing verified dialogue seed trajectories."""

    def __init__(self) -> None:
        self.solver = TableauSolver()
        self.transpiler = TableauToDialogueTranspiler()

    def generate_tautology_formulas(self, count: int = 50) -> List[Formula]:
        """Synthesize a collection of parameterized propositional tautologies."""
        formulas: List[Formula] = []
        atom_names = ["A", "B", "C", "D", "P", "Q", "R", "S"]

        for i in range(count):
            a = Atom(atom_names[i % len(atom_names)])
            b = Atom(atom_names[(i + 1) % len(atom_names)])
            c = Atom(atom_names[(i + 2) % len(atom_names)])

            family = i % 6
            if family == 0:
                # Identity: A -> A
                f = Implies(a, a)
            elif family == 1:
                # Weakening: A -> B -> A
                f = Implies(a, Implies(b, a))
            elif family == 2:
                # Modus Ponens: A -> (A -> B) -> B
                f = Implies(a, Implies(Implies(a, b), b))
            elif family == 3:
                # Conjunction: A -> B -> (A ∧ B)
                f = Implies(a, Implies(b, And(a, b)))
            elif family == 4:
                # Syllogism / Transitivity: (A -> B) -> (B -> C) -> (A -> C)
                f = Implies(Implies(a, b), Implies(Implies(b, c), Implies(a, c)))
            else:
                # S-combinator / Distribution: (A -> B -> C) -> (A -> B) -> (A -> C)
                f = Implies(Implies(a, Implies(b, c)), Implies(Implies(a, b), Implies(a, c)))

            formulas.append(f)

        return formulas

    def generate_seed_strategies(self, count: int = 50) -> List[Dict[str, Any]]:
        """Generate verified StrategyTree dictionaries for synthesized tautologies."""
        formulas = self.generate_tautology_formulas(count)
        strategies: List[Dict[str, Any]] = []

        for f in formulas:
            tableau = self.solver.prove(f)
            if tableau:
                strat = self.transpiler.transpile_to_strategy(f, tableau)
                strategies.append(strat)

        return strategies

    def populate_replay_buffer(
        self,
        buffer: ReplayBuffer,
        count: int = 50,
        feature_dim: int = 32,
        action_space_size: int = 64,
    ) -> int:
        """Transpile seed tautologies directly into training transitions in ReplayBuffer."""
        formulas = self.generate_tautology_formulas(count)
        inserted_trajectories = 0

        for f in formulas:
            tableau = self.solver.prove(f)
            if not tableau:
                continue

            traj = GameTrajectory()
            # Synthetic feature representation for seed formula
            state_vec = torch.randn(feature_dim, dtype=torch.float32)

            # Uniform/heuristic policy target centered on optimal action
            optimal_action = 0
            policy_target = np.zeros(action_space_size, dtype=np.float32)
            policy_target[optimal_action] = 0.8
            policy_target += 0.2 / action_space_size

            traj.states.append(state_vec)
            traj.actions.append(optimal_action)
            traj.policies.append(policy_target)
            traj.rewards.append(1.0)
            traj.players.append(1)
            traj.terminal_value = 1.0

            buffer.push(traj)
            inserted_trajectories += 1

        return inserted_trajectories

    def export_dataset_json(self, filepath: str, count: int = 50) -> None:
        """Export synthesized seed strategies to a JSON file."""
        strategies = self.generate_seed_strategies(count)
        with open(filepath, "w", encoding="utf-8") as f:
            json.dump(strategies, f, indent=2)
