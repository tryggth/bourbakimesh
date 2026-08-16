"""BourbakiMesh Python ML and Game-Semantic Dynamics Engine."""

from .models import (
    ArenaEmbeddingConfig,
    BourbakiMuZero,
    DynamicsNetwork,
    PredictionNetwork,
    RepresentationNetwork,
)
from .latent_mcts import LatentMCTS, MCTSConfig, Node
from .self_play import GameTrajectory, ReplayBuffer, SelfPlayWorker
from .adversarial_hunt import FalseInconsistencyHunter, HuntResult

__version__ = "0.1.0"

__all__ = [
    "ArenaEmbeddingConfig",
    "BourbakiMuZero",
    "RepresentationNetwork",
    "DynamicsNetwork",
    "PredictionNetwork",
    "LatentMCTS",
    "MCTSConfig",
    "Node",
    "GameTrajectory",
    "SelfPlayWorker",
    "ReplayBuffer",
    "FalseInconsistencyHunter",
    "HuntResult",
]
