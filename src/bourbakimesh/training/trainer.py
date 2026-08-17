"""BourbakiMuZero trainer engine with K-step recurrent latent dynamics unrolling and multi-task loss."""

from __future__ import annotations
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, Optional, Tuple
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.optim import AdamW
from torch.optim.lr_scheduler import CosineAnnealingLR
from torch.utils.data import DataLoader
from bourbakimesh.models import BourbakiMuZero


def scale_gradient(tensor: torch.Tensor, scale: float) -> torch.Tensor:
    """Scale gradient during backpropagation (MuZero stability trick)."""
    return tensor * scale + tensor.detach() * (1.0 - scale)


@dataclass
class TrainingConfig:
    """Hyperparameters and configuration for BourbakiMuZero training."""

    batch_size: int = 64
    learning_rate: float = 1e-3
    weight_decay: float = 1e-4
    unroll_steps: int = 5
    epochs: int = 10
    policy_loss_coeff: float = 1.0
    value_loss_coeff: float = 0.5
    reward_loss_coeff: float = 1.0
    grad_clip_norm: float = 5.0
    device: str = "auto"
    mixed_precision: bool = True


@dataclass
class TrainStepResult:
    """Metrics and losses from a single training iteration."""

    total_loss: float
    policy_loss: float
    value_loss: float
    reward_loss: float


class BourbakiTrainer:
    """Recurrent multi-task trainer for BourbakiMuZero model."""

    def __init__(
        self,
        model: BourbakiMuZero,
        config: Optional[TrainingConfig] = None,
    ) -> None:
        self.config = config or TrainingConfig()

        if self.config.device == "auto":
            self.device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
        else:
            self.device = torch.device(self.config.device)

        self.model = model.to(self.device)
        self.optimizer = AdamW(
            self.model.parameters(),
            lr=self.config.learning_rate,
            weight_decay=self.config.weight_decay,
        )
        self.scheduler = CosineAnnealingLR(
            self.optimizer,
            T_max=self.config.epochs,
            eta_min=1e-6,
        )
        enabled_amp = self.config.mixed_precision and self.device.type == "cuda"
        self.scaler = torch.amp.GradScaler("cuda", enabled=enabled_amp)
        self.global_step = 0

    def compute_loss(
        self,
        batch: Dict[str, torch.Tensor],
    ) -> Tuple[torch.Tensor, TrainStepResult]:
        """Unroll model over K steps and compute composite multi-task loss."""
        obs_0 = batch["obs_0"].to(self.device)
        actions = batch["actions"].to(self.device)                    # (B, K)
        target_policies = batch["target_policies"].to(self.device)    # (B, K+1, A)
        target_values = batch["target_values"].to(self.device)        # (B, K+1, 1)
        target_rewards = batch["target_rewards"].to(self.device)      # (B, K, 1)
        masks = batch["masks"].to(self.device)                        # (B, K+1)

        unroll_steps = actions.shape[1]

        total_policy_loss = torch.tensor(0.0, device=self.device)
        total_value_loss = torch.tensor(0.0, device=self.device)
        total_reward_loss = torch.tensor(0.0, device=self.device)

        # 1. Step 0: Initial Representation & Prediction
        s_k = self.model.representation(obs_0)
        p_0, v_0 = self.model.prediction(s_k)

        # Policy cross-entropy: - \sum target * log_softmax(logits)
        log_p_0 = F.log_softmax(p_0, dim=-1)
        step_0_pol_loss = -(target_policies[:, 0] * log_p_0).sum(dim=-1)
        total_policy_loss = total_policy_loss + (step_0_pol_loss * masks[:, 0]).mean()

        step_0_val_loss = F.mse_loss(v_0, target_values[:, 0], reduction="none").squeeze(-1)
        total_value_loss = total_value_loss + (step_0_val_loss * masks[:, 0]).mean()

        # 2. Recurrent Unrolling (k = 0 ... K-1)
        for k in range(unroll_steps):
            act_k = actions[:, k]
            s_next, r_k = self.model.dynamics(s_k, act_k)

            # Scale gradient for recurrent stability
            s_k = scale_gradient(s_next, 0.5)

            p_k, v_k = self.model.prediction(s_k)

            mask_k1 = masks[:, k + 1]

            # Recurrent Policy Loss
            log_p_k = F.log_softmax(p_k, dim=-1)
            pol_loss_k = -(target_policies[:, k + 1] * log_p_k).sum(dim=-1)
            total_policy_loss = total_policy_loss + (pol_loss_k * mask_k1).mean()

            # Recurrent Value Loss
            val_loss_k = F.mse_loss(v_k, target_values[:, k + 1], reduction="none").squeeze(-1)
            total_value_loss = total_value_loss + (val_loss_k * mask_k1).mean()

            # Intermediate Reward Loss
            rew_loss_k = F.mse_loss(r_k, target_rewards[:, k], reduction="none").squeeze(-1)
            total_reward_loss = total_reward_loss + (rew_loss_k * mask_k1).mean()

        # Normalization across unroll steps
        total_policy_loss = total_policy_loss / (unroll_steps + 1)
        total_value_loss = total_value_loss / (unroll_steps + 1)
        if unroll_steps > 0:
            total_reward_loss = total_reward_loss / unroll_steps

        total_loss = (
            self.config.policy_loss_coeff * total_policy_loss
            + self.config.value_loss_coeff * total_value_loss
            + self.config.reward_loss_coeff * total_reward_loss
        )

        metrics = TrainStepResult(
            total_loss=float(total_loss.item()),
            policy_loss=float(total_policy_loss.item()),
            value_loss=float(total_value_loss.item()),
            reward_loss=float(total_reward_loss.item()),
        )

        return total_loss, metrics

    def train_step(self, batch: Dict[str, torch.Tensor]) -> TrainStepResult:
        """Perform one training optimization step."""
        self.model.train()
        self.optimizer.zero_grad()

        device_type = self.device.type
        with torch.amp.autocast(device_type=device_type, enabled=(self.config.mixed_precision and device_type == "cuda")):
            loss, metrics = self.compute_loss(batch)

        if self.scaler.is_enabled():
            self.scaler.scale(loss).backward()
            self.scaler.unscale_(self.optimizer)
            torch.nn.utils.clip_grad_norm_(
                self.model.parameters(), self.config.grad_clip_norm
            )
            self.scaler.step(self.optimizer)
            self.scaler.update()
        else:
            loss.backward()
            torch.nn.utils.clip_grad_norm_(
                self.model.parameters(), self.config.grad_clip_norm
            )
            self.optimizer.step()

        self.global_step += 1
        return metrics

    def train_epoch(self, dataloader: DataLoader) -> TrainStepResult:
        """Train over an entire epoch of batches."""
        epoch_losses: Dict[str, list[float]] = {
            "total": [],
            "policy": [],
            "value": [],
            "reward": [],
        }

        for batch in dataloader:
            metrics = self.train_step(batch)
            epoch_losses["total"].append(metrics.total_loss)
            epoch_losses["policy"].append(metrics.policy_loss)
            epoch_losses["value"].append(metrics.value_loss)
            epoch_losses["reward"].append(metrics.reward_loss)

        self.scheduler.step()

        return TrainStepResult(
            total_loss=float(np.mean(epoch_losses["total"])),
            policy_loss=float(np.mean(epoch_losses["policy"])),
            value_loss=float(np.mean(epoch_losses["value"])),
            reward_loss=float(np.mean(epoch_losses["reward"])),
        )

    def save_checkpoint(
        self,
        path: Path | str,
        extra_meta: Optional[Dict[str, Any]] = None,
    ) -> None:
        """Save model weights, optimizer, and training metadata to disk."""
        target_path = Path(path)
        target_path.parent.mkdir(parents=True, exist_ok=True)

        model_cfg = (
            self.model.config.model_dump()
            if hasattr(self.model.config, "model_dump")
            else self.model.config.__dict__
        )
        state = {
            "global_step": self.global_step,
            "model_state_dict": self.model.state_dict(),
            "state_dict": self.model.state_dict(),
            "model_config": model_cfg,
            "model_kwargs": model_cfg,
            "optimizer_state_dict": self.optimizer.state_dict(),
            "scheduler_state_dict": self.scheduler.state_dict(),
            "config": self.config.__dict__,
            "extra_meta": extra_meta or {},
        }
        torch.save(state, target_path)

    def load_checkpoint(self, path: Path | str) -> Dict[str, Any]:
        """Load checkpoint state into trainer and model."""
        checkpoint = torch.load(path, map_location=self.device)
        self.model.load_state_dict(checkpoint["model_state_dict"])
        self.optimizer.load_state_dict(checkpoint["optimizer_state_dict"])
        if "scheduler_state_dict" in checkpoint:
            self.scheduler.load_state_dict(checkpoint["scheduler_state_dict"])
        self.global_step = checkpoint.get("global_step", 0)
        return checkpoint
