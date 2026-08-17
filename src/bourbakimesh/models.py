"""PyTorch neural network architecture for BourbakiMuZero latent dynamics and Relational Arena Graph Transformer."""

from __future__ import annotations
import math
from typing import Optional, Tuple
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
    use_relational_transformer: bool = Field(default=True, description="Enable RelationalArenaTransformer for h_theta")
    transformer_layers: int = Field(default=4, ge=1, description="Number of relational transformer layers")
    transformer_heads: int = Field(default=8, ge=1, description="Number of attention heads")
    num_relations: int = Field(default=8, ge=1, description="Number of dialogue arena relation types")
    max_seq_len: int = Field(default=128, ge=1, description="Maximum dialogue move sequence length")


def normalize_latent(latent: torch.Tensor, eps: float = 1e-6) -> torch.Tensor:
    """Normalize latent state embeddings to unit sphere."""
    norm = torch.norm(latent, p=2, dim=-1, keepdim=True)
    return latent / (norm + eps)


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


class RelationalMultiheadAttention(nn.Module):
    """Multi-head self-attention with learnable relational edge embeddings (RGAT).

    Computes:
        alpha_ij^(h) = softmax_j ( (W_Q x_i)^T (W_K x_j + r_{e_ij}^K) / sqrt(d_k) )
        out_i^(h)   = sum_j alpha_ij^(h) (W_V x_j + r_{e_ij}^V)
    """

    def __init__(
        self,
        d_model: int,
        num_heads: int = 8,
        num_relations: int = 8,
        dropout: float = 0.0,
    ) -> None:
        super().__init__()
        assert d_model % num_heads == 0, "d_model must be divisible by num_heads"
        self.d_model = d_model
        self.num_heads = num_heads
        self.head_dim = d_model // num_heads
        self.scale = 1.0 / math.sqrt(self.head_dim)

        self.q_proj = nn.Linear(d_model, d_model)
        self.k_proj = nn.Linear(d_model, d_model)
        self.v_proj = nn.Linear(d_model, d_model)
        self.out_proj = nn.Linear(d_model, d_model)

        # Relational edge embeddings for Key and Value bias
        self.rel_k_embed = nn.Embedding(num_relations, self.head_dim)
        self.rel_v_embed = nn.Embedding(num_relations, self.head_dim)
        self.dropout = nn.Dropout(dropout) if dropout > 0 else nn.Identity()

    def forward(
        self,
        x: torch.Tensor,
        relation_matrix: Optional[torch.Tensor] = None,
        key_padding_mask: Optional[torch.Tensor] = None,
    ) -> torch.Tensor:
        """
        Args:
            x: (B, N, d_model)
            relation_matrix: (B, N, N) containing discrete relation indices 0..num_relations-1
            key_padding_mask: (B, N) bool tensor where True indicates padding
        """
        B, N, _ = x.shape

        q = self.q_proj(x).view(B, N, self.num_heads, self.head_dim).transpose(1, 2)  # (B, H, N, d_k)
        k = self.k_proj(x).view(B, N, self.num_heads, self.head_dim).transpose(1, 2)  # (B, H, N, d_k)
        v = self.v_proj(x).view(B, N, self.num_heads, self.head_dim).transpose(1, 2)  # (B, H, N, d_k)

        # Base attention scores: (B, H, N, N)
        scores = torch.matmul(q, k.transpose(-2, -1)) * self.scale

        # Incorporate relational edge bias
        if relation_matrix is not None:
            if relation_matrix.dim() == 2:
                relation_matrix = relation_matrix.unsqueeze(0).expand(B, -1, -1)
            # (B, N, N, d_k)
            r_k = self.rel_k_embed(relation_matrix)
            # (B, H, N, N) relational key score: sum_d (q_{b, h, i, d} * r_k_{b, i, j, d}) * scale
            # q: (B, H, N, 1, d_k), r_k: (B, 1, N, N, d_k)
            q_expanded = q.unsqueeze(3)
            r_k_expanded = r_k.unsqueeze(1)
            rel_scores = (q_expanded * r_k_expanded).sum(dim=-1) * self.scale
            scores = scores + rel_scores

        if key_padding_mask is not None:
            # key_padding_mask: (B, N) -> (B, 1, 1, N)
            mask = key_padding_mask.unsqueeze(1).unsqueeze(2)
            scores = scores.masked_fill(mask, float("-inf"))

        attn_weights = F.softmax(scores, dim=-1)
        attn_weights = self.dropout(attn_weights)

        # Context aggregation
        out = torch.matmul(attn_weights, v)  # (B, H, N, d_k)

        if relation_matrix is not None:
            # Add relational value bias: sum_j attn_{b, h, i, j} * r_v_{b, i, j, d}
            r_v = self.rel_v_embed(relation_matrix).unsqueeze(1)  # (B, 1, N, N, d_k)
            rel_v_out = (attn_weights.unsqueeze(-1) * r_v).sum(dim=3)  # (B, H, N, d_k)
            out = out + rel_v_out

        out = out.transpose(1, 2).contiguous().view(B, N, self.d_model)
        return self.out_proj(out)


class RelationalTransformerLayer(nn.Module):
    """Pre-LayerNorm Relational Transformer Layer with SwiGLU / GeLU feedforward block."""

    def __init__(
        self,
        d_model: int,
        num_heads: int = 8,
        num_relations: int = 8,
        dim_feedforward: Optional[int] = None,
        dropout: float = 0.0,
    ) -> None:
        super().__init__()
        d_ff = dim_feedforward or (d_model * 4)
        self.ln1 = nn.LayerNorm(d_model)
        self.self_attn = RelationalMultiheadAttention(
            d_model=d_model,
            num_heads=num_heads,
            num_relations=num_relations,
            dropout=dropout,
        )
        self.dropout1 = nn.Dropout(dropout) if dropout > 0 else nn.Identity()

        self.ln2 = nn.LayerNorm(d_model)
        self.ffn = nn.Sequential(
            nn.Linear(d_model, d_ff),
            nn.GELU(),
            nn.Dropout(dropout) if dropout > 0 else nn.Identity(),
            nn.Linear(d_ff, d_model),
        )
        self.dropout2 = nn.Dropout(dropout) if dropout > 0 else nn.Identity()

    def forward(
        self,
        x: torch.Tensor,
        relation_matrix: Optional[torch.Tensor] = None,
        key_padding_mask: Optional[torch.Tensor] = None,
    ) -> torch.Tensor:
        # Pre-LN Self-Attention
        norm_x = self.ln1(x)
        attn_out = self.self_attn(
            norm_x,
            relation_matrix=relation_matrix,
            key_padding_mask=key_padding_mask,
        )
        x = x + self.dropout1(attn_out)

        # Pre-LN Feedforward
        x = x + self.dropout2(self.ffn(self.ln2(x)))
        return x


class RelationalArenaTransformer(nn.Module):
    """20M–50M parameter Relational Arena Graph Transformer representation network (h_theta).

    Encodes dialogue moves, polarities, justification pointers, and active view scopes
    into a continuous latent arena embedding s_0 in R^{latent_dim}.
    """

    def __init__(self, config: ArenaEmbeddingConfig) -> None:
        super().__init__()
        self.config = config
        d_model = config.hidden_dim
        num_layers = config.transformer_layers
        num_heads = config.transformer_heads
        num_relations = config.num_relations
        max_seq_len = config.max_seq_len

        self.token_proj = nn.Linear(config.feature_dim, d_model)
        self.pos_embed = nn.Embedding(max_seq_len, d_model)
        self.polarity_embed = nn.Embedding(3, d_model)  # -1 (O), 0 (Neutral), +1 (P)

        self.layers = nn.ModuleList([
            RelationalTransformerLayer(
                d_model=d_model,
                num_heads=num_heads,
                num_relations=num_relations,
            )
            for _ in range(num_layers)
        ])

        self.final_ln = nn.LayerNorm(d_model)
        self.pool_head = nn.Sequential(
            nn.Linear(d_model, d_model),
            nn.GELU(),
            nn.Linear(d_model, config.latent_dim),
        )

    def forward(
        self,
        obs: torch.Tensor,
        relation_matrix: Optional[torch.Tensor] = None,
        polarities: Optional[torch.Tensor] = None,
    ) -> torch.Tensor:
        """
        Args:
            obs: (B, feature_dim) or (B, N, feature_dim)
            relation_matrix: Optional (B, N, N) relation tensor
            polarities: Optional (B, N) polarities (-1, 0, +1)
        """
        # Handle 2D flat observations: (B, feature_dim) -> (B, 1, feature_dim)
        if obs.dim() == 2:
            obs = obs.unsqueeze(1)

        B, N, _ = obs.shape
        device = obs.device

        x = self.token_proj(obs)

        # Positional Embeddings
        positions = torch.arange(N, device=device).unsqueeze(0).expand(B, N)
        positions = torch.clamp(positions, max=self.config.max_seq_len - 1)
        x = x + self.pos_embed(positions)

        # Polarity Embeddings
        if polarities is not None:
            # Map [-1, 0, 1] to [0, 1, 2]
            pol_indices = (polarities + 1).long().clamp(0, 2)
            x = x + self.polarity_embed(pol_indices)

        # Default sequential relation matrix if none provided
        if relation_matrix is None and N > 1:
            rel_mat = torch.zeros((B, N, N), dtype=torch.long, device=device)
            # Relation 0: Sequential adjacency
            for i in range(N - 1):
                rel_mat[:, i, i + 1] = 1
            relation_matrix = rel_mat

        # Pass through Relational Transformer Layers
        for layer in self.layers:
            x = layer(x, relation_matrix=relation_matrix)

        x = self.final_ln(x)

        # Permutation-invariant attentive/mean pooling across sequence
        pooled = x.mean(dim=1)  # (B, d_model)
        raw_latent = self.pool_head(pooled)
        return normalize_latent(raw_latent)


class MLPRepresentationNetwork(nn.Module):
    """Fallback MLP representation network."""

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
        if obs.dim() == 3:
            obs = obs.mean(dim=1)
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

    def forward(self, state: torch.Tensor, action: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
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

    def forward(self, state: torch.Tensor) -> Tuple[torch.Tensor, torch.Tensor]:
        features = self.trunk(state)
        policy_logits = self.policy_head(features)
        value = self.value_head(features)
        return policy_logits, value


class BourbakiMuZero(nn.Module):
    """Full composite BourbakiMuZero model unifying h_theta, g_theta, and f_theta."""

    def __init__(self, config: Optional[ArenaEmbeddingConfig] = None) -> None:
        super().__init__()
        self.config = config or ArenaEmbeddingConfig()

        if self.config.use_relational_transformer:
            self.representation = RelationalArenaTransformer(self.config)
        else:
            self.representation = MLPRepresentationNetwork(self.config)

        self.dynamics = DynamicsNetwork(self.config)
        self.prediction = PredictionNetwork(self.config)

    def initial_inference(
        self,
        obs: torch.Tensor,
        relation_matrix: Optional[torch.Tensor] = None,
        polarities: Optional[torch.Tensor] = None,
    ) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """h_theta(obs) -> s_0, followed by f_theta(s_0) -> (policy_logits, value)."""
        if isinstance(self.representation, RelationalArenaTransformer):
            latent_state = self.representation(
                obs, relation_matrix=relation_matrix, polarities=polarities
            )
        else:
            latent_state = self.representation(obs)

        policy_logits, value = self.prediction(latent_state)
        return latent_state, policy_logits, value

    def recurrent_inference(
        self, latent_state: torch.Tensor, action: torch.Tensor
    ) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        """g_theta(s_t, a_t) -> (s_{t+1}, r_t), followed by f_theta(s_{t+1}) -> (policy_logits, value)."""
        next_latent, reward = self.dynamics(latent_state, action)
        policy_logits, value = self.prediction(next_latent)
        return next_latent, reward, policy_logits, value

    @classmethod
    def load_from_checkpoint(
        cls,
        path: str | Path,
        map_location: str = "cpu",
    ) -> BourbakiMuZero:
        """Load BourbakiMuZero model from checkpoint with automatic architecture inference."""
        ckpt = torch.load(path, map_location=map_location)

        # 1. Extract state dict
        if isinstance(ckpt, dict):
            state_dict = ckpt.get("model_state_dict", ckpt.get("state_dict", ckpt))
        else:
            state_dict = ckpt

        # 2. Extract or infer config
        config = None
        if isinstance(ckpt, dict):
            for cfg_key in ["model_config", "model_kwargs", "config"]:
                if cfg_key in ckpt and isinstance(ckpt[cfg_key], dict):
                    try:
                        cfg_dict = ckpt[cfg_key]
                        # Filter to only valid fields of ArenaEmbeddingConfig
                        valid_fields = set(ArenaEmbeddingConfig.model_fields.keys())
                        filtered_cfg = {k: v for k, v in cfg_dict.items() if k in valid_fields}
                        candidate = ArenaEmbeddingConfig(**filtered_cfg)

                        # Verify candidate against tensor dimensions
                        if "representation.token_proj.weight" in state_dict:
                            w = state_dict["representation.token_proj.weight"]
                            if w.shape[0] == candidate.hidden_dim and w.shape[1] == candidate.feature_dim:
                                config = candidate
                                break
                    except Exception:
                        pass

        if config is None:
            # Auto-infer config from state_dict shapes
            hidden_dim = 256
            feature_dim = 32
            if "representation.token_proj.weight" in state_dict:
                w = state_dict["representation.token_proj.weight"]
                hidden_dim = w.shape[0]
                feature_dim = w.shape[1]
            elif "representation.encoder.0.weight" in state_dict:
                w = state_dict["representation.encoder.0.weight"]
                hidden_dim = w.shape[0]
                feature_dim = w.shape[1]

            action_space_size = 64
            latent_dim = 128
            if "dynamics.action_embed.weight" in state_dict:
                w = state_dict["dynamics.action_embed.weight"]
                action_space_size = w.shape[0]
                latent_dim = w.shape[1]

            num_res_blocks = 0
            while f"dynamics.blocks.{num_res_blocks}.fc1.weight" in state_dict:
                num_res_blocks += 1
            if num_res_blocks == 0:
                num_res_blocks = 2

            num_layers = 0
            while f"representation.layers.{num_layers}.ln1.weight" in state_dict:
                num_layers += 1
            use_transformer = num_layers > 0
            if num_layers == 0:
                num_layers = 4

            num_heads = 8
            if "representation.layers.0.self_attn.rel_k_embed.weight" in state_dict:
                head_dim = state_dict["representation.layers.0.self_attn.rel_k_embed.weight"].shape[1]
                num_heads = hidden_dim // max(1, head_dim)

            config = ArenaEmbeddingConfig(
                feature_dim=feature_dim,
                latent_dim=latent_dim,
                action_space_size=action_space_size,
                hidden_dim=hidden_dim,
                num_res_blocks=num_res_blocks,
                use_relational_transformer=use_transformer,
                transformer_layers=num_layers,
                transformer_heads=num_heads,
            )

        model = cls(config)
        model.load_state_dict(state_dict, strict=True)
        model.to(map_location)
        model.eval()
        return model
