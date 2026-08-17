"""Mathlib theorem corpus decompilation and curriculum ingestion package."""

from .curriculum import CurriculumManager, TheoremEntry
from .pipeline import CorpusPipeline, PipelineConfig

__all__ = [
    "CurriculumManager",
    "TheoremEntry",
    "CorpusPipeline",
    "PipelineConfig",
]
