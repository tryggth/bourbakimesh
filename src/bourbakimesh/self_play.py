"""Self-play worker and replay buffer for BourbakiMuZero."""

from __future__ import annotations
from dataclasses import dataclass, field
import random
from typing import Dict, List, Optional
import numpy as np
import torch
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import BourbakiMuZero


@dataclass
class GameTrajectory:
    """Recorded trajectory from a self-play dialogue game."""

    states: List[torch.Tensor] = field(default_factory=list)
    actions: List[int] = field(default_factory=list)
    policies: List[np.ndarray] = field(default_factory=list)
    rewards: List[float] = field(default_factory=list)
    players: List[int] = field(default_factory=list)
    terminal_value: float = 0.0

    def __len__(self) -> usize:
        return len(self.actions)


class SelfPlayWorker:
    """Self-play orchestrator generating game-semantic dialogue trajectories."""

    def __init__(
        self,
        model: BourbakiMuZero,
        mcts_config: Optional[MCTSConfig] = None,
    ) -> None:
        self.model = model
        self.config = mcts_config or MCTSConfig()
        self.mcts = LatentMCTS(model, self.config)

    def play_game(
        self,
        initial_obs: Optional[torch.Tensor] = None,
        max_moves: int = 20,
        num_simulations: int = 50,
    ) -> GameTrajectory:
        """Execute a full self-play dialogue game between Proponent (+1) and Opponent (-1)."""
        feature_dim = self.model.config.feature_dim
        if initial_obs is None:
            current_obs = torch.randn(1, feature_dim, dtype=torch.float32)
        else:
            current_obs = initial_obs if initial_obs.dim() == 2 else initial_obs.unsqueeze(0)

        trajectory = GameTrajectory()
        current_player = 1  # Proponent opens at step 0
        terminal = False
        step = 0

        while not terminal and step < max_moves:
            policy = self.mcts.search(
                current_obs,
                current_player=current_player,
                num_simulations=num_simulations,
                is_latent=False,
            )

            # Sample action from visit distribution
            action = int(np.random.choice(len(policy), p=policy))

            trajectory.states.append(current_obs.squeeze(0).clone())
            trajectory.actions.append(action)
            trajectory.policies.append(policy)
            trajectory.players.append(current_player)

            # Step dynamics
            with torch.no_grad():
                latent = self.model.representation(current_obs)
                next_latent, reward_tensor = self.model.dynamics(
                    latent, torch.tensor([action], dtype=torch.long)
                )
                reward = float(reward_tensor[0].item())
                trajectory.rewards.append(reward)

            # Terminal heuristic for synthetic play
            if abs(reward) > 0.8 or step == max_moves - 1:
                terminal = True
                # Positive reward favors current player
                terminal_value = 1.0 if current_player == 1 else -1.0
                trajectory.terminal_value = terminal_value
            else:
                # Update observation for next step
                current_obs = torch.randn(1, feature_dim, dtype=torch.float32)
                current_player = -current_player
                step += 1

        return trajectory


class ReplayBuffer:
    """Experience replay buffer storing self-play trajectories for policy/value optimization."""

    def __init__(self, capacity: int = 10000) -> None:
        self.capacity = capacity
        self.trajectories: List[GameTrajectory] = []

    def push(self, trajectory: GameTrajectory) -> None:
        """Add a trajectory to the buffer."""
        if len(self.trajectories) >= self.capacity:
            self.trajectories.pop(0)
        self.trajectories.append(trajectory)

    def __len__(self) -> int:
        return len(self.trajectories)

    def total_steps(self) -> int:
        """Total number of transitions across all stored trajectories."""
        return sum(len(t) for t in self.trajectories)

    def sample_batch(self, batch_size: int) -> Dict[str, torch.Tensor]:
        """Sample a batch of training transitions (s, a, target_policy, target_value)."""
        if not self.trajectories:
            raise ValueError("Cannot sample from an empty replay buffer")

        states = []
        actions = []
        target_policies = []
        target_values = []

        all_samples = []
        for traj in self.trajectories:
            for idx in range(len(traj)):
                all_samples.append((traj, idx))

        sampled = random.sample(all_samples, min(batch_size, len(all_samples)))

        for traj, idx in sampled:
            states.append(traj.states[idx])
            actions.append(traj.actions[idx])
            target_policies.append(traj.policies[idx])
            # Value perspective relative to the acting player at that step
            player = traj.players[idx]
            value_target = traj.terminal_value if player == 1 else -traj.terminal_value
            target_values.append(value_target)

        return {
            "states": torch.stack(states),
            "actions": torch.tensor(actions, dtype=torch.long),
            "target_policies": torch.tensor(np.array(target_policies), dtype=torch.float32),
            "target_values": torch.tensor(target_values, dtype=torch.float32).unsqueeze(-1),
        }
