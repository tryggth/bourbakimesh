"""Dataset and trajectory window sampler for BourbakiMuZero K-step recurrent training."""

from __future__ import annotations
from dataclasses import dataclass
from typing import Any, Dict, List, Tuple
import numpy as np
import torch
from torch.utils.data import Dataset
from bourbakimesh.self_play import GameTrajectory, ReplayBuffer


@dataclass
class TrajectoryWindow:
    """A sampled K-step recurrent trajectory window for MuZero unrolled training."""

    obs_0: torch.Tensor             # (feature_dim,)
    actions: torch.Tensor           # (unroll_steps,) Long
    target_policies: torch.Tensor   # (unroll_steps + 1, action_space_size) Float
    target_values: torch.Tensor     # (unroll_steps + 1, 1) Float
    target_rewards: torch.Tensor    # (unroll_steps, 1) Float
    mask: torch.Tensor              # (unroll_steps + 1,) Float (1.0 = valid, 0.0 = padded)


class ReplayDataset(Dataset):
    """PyTorch Dataset sampling fixed K-step unroll windows from replay trajectories."""

    def __init__(
        self,
        trajectories: List[GameTrajectory],
        unroll_steps: int = 5,
        feature_dim: int = 32,
        action_space_size: int = 16,
    ) -> None:
        self.trajectories = [t for t in trajectories if len(t.actions) > 0]
        self.unroll_steps = unroll_steps
        self.feature_dim = feature_dim
        self.action_space_size = action_space_size

        # Index all valid start positions across all non-empty trajectories
        self.samples: List[Tuple[int, int]] = []
        for traj_idx, traj in enumerate(self.trajectories):
            for start_pos in range(len(traj.actions)):
                self.samples.append((traj_idx, start_pos))

    @classmethod
    def from_replay_buffer(
        cls,
        buffer: ReplayBuffer,
        unroll_steps: int = 5,
        feature_dim: int = 32,
        action_space_size: int = 16,
    ) -> ReplayDataset:
        """Construct dataset directly from an existing ReplayBuffer."""
        return cls(
            trajectories=buffer.trajectories,
            unroll_steps=unroll_steps,
            feature_dim=feature_dim,
            action_space_size=action_space_size,
        )

    def __len__(self) -> int:
        return len(self.samples)

    def __getitem__(self, idx: int) -> TrajectoryWindow:
        traj_idx, start_pos = self.samples[idx]
        traj = self.trajectories[traj_idx]

        # 1. Extract initial observation o_0
        obs_0 = traj.states[start_pos]
        if isinstance(obs_0, np.ndarray):
            obs_0 = torch.from_numpy(obs_0).float()
        elif obs_0.dim() > 1:
            obs_0 = obs_0.squeeze(0).float()
        else:
            obs_0 = obs_0.float()

        # 2. Extract action sequence and targets over K unroll steps
        actions = torch.zeros(self.unroll_steps, dtype=torch.long)
        target_policies = torch.zeros(
            (self.unroll_steps + 1, self.action_space_size), dtype=torch.float32
        )
        target_values = torch.zeros((self.unroll_steps + 1, 1), dtype=torch.float32)
        target_rewards = torch.zeros((self.unroll_steps, 1), dtype=torch.float32)
        mask = torch.zeros(self.unroll_steps + 1, dtype=torch.float32)

        traj_len = len(traj.actions)

        for k in range(self.unroll_steps + 1):
            curr_pos = start_pos + k

            if curr_pos < traj_len:
                # Step exists in trajectory
                pol = traj.policies[curr_pos]
                if isinstance(pol, np.ndarray):
                    target_policies[k] = torch.from_numpy(pol).float()
                else:
                    target_policies[k] = torch.tensor(pol, dtype=torch.float32)

                player = traj.players[curr_pos] if curr_pos < len(traj.players) else 1
                val = traj.terminal_value * player
                target_values[k, 0] = float(val)
                mask[k] = 1.0

                if k < self.unroll_steps:
                    actions[k] = int(traj.actions[curr_pos])
                    rew = traj.rewards[curr_pos] if curr_pos < len(traj.rewards) else 0.0
                    target_rewards[k, 0] = float(rew)
            else:
                # Padding beyond terminal state
                target_policies[k].fill_(1.0 / self.action_space_size)
                target_values[k, 0] = 0.0
                mask[k] = 0.0

                if k < self.unroll_steps:
                    actions[k] = 0
                    target_rewards[k, 0] = 0.0

        return TrajectoryWindow(
            obs_0=obs_0,
            actions=actions,
            target_policies=target_policies,
            target_values=target_values,
            target_rewards=target_rewards,
            mask=mask,
        )

    @staticmethod
    def collate_fn(batch: List[TrajectoryWindow]) -> Dict[str, torch.Tensor]:
        """Collate a batch of TrajectoryWindow objects into stacked tensors."""
        obs_0 = torch.stack([w.obs_0 for w in batch], dim=0)
        actions = torch.stack([w.actions for w in batch], dim=0)
        target_policies = torch.stack([w.target_policies for w in batch], dim=0)
        target_values = torch.stack([w.target_values for w in batch], dim=0)
        target_rewards = torch.stack([w.target_rewards for w in batch], dim=0)
        masks = torch.stack([w.mask for w in batch], dim=0)

        return {
            "obs_0": obs_0,
            "actions": actions,
            "target_policies": target_policies,
            "target_values": target_values,
            "target_rewards": target_rewards,
            "masks": masks,
        }
