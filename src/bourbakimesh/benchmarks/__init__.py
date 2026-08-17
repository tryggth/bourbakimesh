"""Performance benchmarking, CSE evaluation, and head-to-head tournament harness."""

from .bench_engine import BenchmarkReport, BenchmarkRunner, MCTSBenchResult, NeuralBenchResult
from .elo import EloTracker, MatchRecord, ModelEloSummary
from .tournament import GameOutcome, ModelTournament, TournamentProposition, TournamentReport

__all__ = [
    "BenchmarkRunner",
    "BenchmarkReport",
    "NeuralBenchResult",
    "MCTSBenchResult",
    "EloTracker",
    "MatchRecord",
    "ModelEloSummary",
    "ModelTournament",
    "TournamentProposition",
    "GameOutcome",
    "TournamentReport",
]
