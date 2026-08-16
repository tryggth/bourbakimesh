"""PyTorch neural network architecture for BourbakiMuZero latent dynamics."""

from __future__ import annotations
from dataclasses import dataclass
import torch
import torch.nn as nn
import torch.nn.functional as F
from pydantic import BaseModel, Field


class ArenaEmbeddingConfig(BaseModel):
    """Hyperparameters and architecture configuration for BourbakiMuZero."""

    feature_dim: int = Field(default=32, ge=1, description="Input feature dimension")
    latent_dim: int = Field(default=128, ge=1, description="Latent state dimension d")
    action_space_size: int = Field(default=64, ge=1, description="Discrete action space |A|")
    hidden_dim: int = Field(default=256, ge=1, description="Hidden layer dimension")
    num_res_blocks: int = Field(default=4, ge=1, description="Number of residual MLP blocks")


class ResidualBlock(nn.Module):
    """Residual MLP block with LayerNorm and GELU activations."""

    def __init__(self, dim: int) -> None:
        super().__init__()
        self.fc1 = nn.Linear(dim, dim)
        self.ln1 = nn.LayerNorm(dim)
        self.fc2 = nn.Linear(dim, dim)
        self.ln2 = nn.LayerNorm(dim)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        residual = x
        out = F.gelu(self.ln1(self.fc1(x)))
        out = self.ln2(self.fc2(out))
        return F.gelu(out + residual)


def normalize_latent(latent: torch.Tensor, eps: float = 1e-6) -> torch.Tensor:
    """Normalize latent state embeddings to unit sphere."""
    norm = torch.norm(latent, p=2, dim=-1, keepdim=True)
    return latent / (norm + eps)


class RepresentationNetwork(nn.Module):
    """Representation network h_theta: obs -> s_0."""

    def __init__(self, config: ArenaEmbeddingConfig) -> None:
        super().__init__()
        self.encoder = nn.Sequential(
            nn.Linear(config.feature_dim, config.hidden_dim),
            nn.LayerNorm(config.hidden_dim),
            nn.GELU(),
            *[ResidualBlock(config.hidden_dim) for _ in range(config.num_res_blocks)],
            nn.Linear(config.hidden_dim, config.latent_dim),
        )

    def forward(self, obs: torch.Tensor) -> torch.Tensor:
        raw_latent = self.encoder(obs)
        return normalize_latent(raw_latent)


class DynamicsNetwork(nn.Module):
    """Dynamics network g_theta: (s_t, a_t) -> (s_{t+1}, r_t)."""

    def __init__(self, config: ArenaEmbeddingConfig) -> None:
        super().__init__()
        self.action_embed = nn.Embedding(config.action_space_size, config.latent_dim)
        self.fusion = nn.Linear(config.latent_dim * 2, config.hidden_dim)
        self.blocks = nn.ModuleList([ResidualBlock(config.hidden_dim) for _ in range(config.num_res_blocks)])
        self.state_head = nn.Linear(config.hidden_dim, config.latent_dim)
        self.reward_head = nn.Sequential(
            nn.Linear(config.hidden_dim, config.hidden_dim // 2),
            nn.GELU(),
            nn.Linear(config.hidden_dim // 2, 1),
            nn.Tanh(),
        )

    def forward(self, state: torch.Tensor, action: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        if action.dim() == 1:
            action = action.unsqueeze(-1)
        action_emb = self.action_embed(action.squeeze(-1))
        combined = torch.cat([state, action_emb], dim=-1)
        x = F.gelu(self.fusion(combined))
        for block in self.blocks:
            x = block(x)
        next_state = normalize_latent(self.state_head(x))
        reward = self.reward_head(x)
        return next_state, reward


class PredictionNetwork(nn.Module):
    """Prediction network f_theta: s_t -> (p_t, v_t)."""

    def __init__(self, config: ArenaEmbeddingConfig) -> None:
        super().__init__()
        self.trunk = nn.Sequential(
            nn.Linear(config.latent_dim, config.hidden_dim),
            nn.LayerNorm(config.hidden_dim),
            nn.GELU(),
            *[ResidualBlock(config.hidden_dim) for _ in range(config.num_res_blocks)],
        )
        self.policy_head = nn.Linear(config.hidden_dim, config.action_space_size)
        self.value_head = nn.Sequential(
            nn.Linear(config.hidden_dim, config.hidden_dim // 2),
            nn.GELU(),
            nn.Linear(config.hidden_dim // 2, 1),
            nn.Tanh(),
        )

    def forward(self, state: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        features = self.trunk(state)
        policy_logits = self.policy_head(features)
        value = self.value_head(features)
        return policy_logits, value


class BourbakiMuZero(nn.Module):
    """Full composite BourbakiMuZero model unifying h_theta, g_theta, and f_theta."""

    def __init__(self, config: ArenaEmbeddingConfig | None = None) -> None:
        super().__init__()
        self.config = config or ArenaEmbeddingConfig()
        self.representation = RepresentationNetwork(self.config)
        self.dynamics = DynamicsNetwork(self.config)
        self.prediction = PredictionNetwork(self.config)

    def initial_inference(self, obs: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """h_theta(obs) -> s_0, followed by f_theta(s_0) -> (policy_logits, value)."""
        latent_state = self.representation(obs)
        policy_logits, value = self.prediction(latent_state)
        return latent_state, policy_logits, value

    def recurrent_inference(
        self, latent_state: torch.Tensor, action: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        """g_theta(s_t, a_t) -> (s_{t+1}, r_t), followed by f_theta(s_{t+1}) -> (policy_logits, value)."""
        next_latent, reward = self.dynamics(latent_state, action)
        policy_logits, value = self.prediction(next_latent)
        return next_latent, reward, policy_logits, value
