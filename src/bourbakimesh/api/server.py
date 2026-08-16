"""FastAPI server for BourbakiMesh ML dynamics and worker orchestration."""

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field
from bourbakimesh import __version__
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.dynamics.arena_game import ArenaState, DialogueMove, DialoguePolarity, MoveKind
import torch

app = FastAPI(
    title="BourbakiMesh ML Orchestrator",
    description="FastAPI service for Latent MCTS self-play and dialogue arena dynamics",
    version=__version__,
)

_global_model = BourbakiMuZero(
    ArenaEmbeddingConfig(
        feature_dim=32,
        latent_dim=128,
        action_space_size=64,
        hidden_dim=256,
        num_res_blocks=4,
    )
)


class HealthResponse(BaseModel):
    status: str
    version: str
    torch_version: str
    cuda_available: bool


class SearchRequest(BaseModel):
    num_simulations: int = Field(default=100, ge=1)
    action_space_size: int = Field(default=64, ge=1)


class SearchResponse(BaseModel):
    action_distribution: list[float]


@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Health check endpoint exposing runtime metadata."""
    return HealthResponse(
        status="healthy",
        version=__version__,
        torch_version=torch.__version__,
        cuda_available=torch.cuda.is_available(),
    )


@app.post("/mcts/search", response_model=SearchResponse)
async def run_mcts_search(request: SearchRequest):
    """Execute a latent MCTS search over dialogue actions."""
    config = MCTSConfig(num_simulations=request.num_simulations)
    mcts = LatentMCTS(_global_model, config)
    dummy_root = torch.zeros((1, 32), dtype=torch.float32)
    dist = mcts.search(dummy_root, current_player=1)
    return SearchResponse(action_distribution=dist.tolist())
