"""BourbakiMuZero training and optimization package."""

from .dataset import ReplayDataset, TrajectoryWindow
from .trainer import BourbakiTrainer, TrainStepResult, TrainingConfig
from .loop import ContinuousTrainingLoop, IterationMetrics, LoopConfig

__all__ = [
    "ReplayDataset",
    "TrajectoryWindow",
    "BourbakiTrainer",
    "TrainingConfig",
    "TrainStepResult",
    "ContinuousTrainingLoop",
    "LoopConfig",
    "IterationMetrics",
]
