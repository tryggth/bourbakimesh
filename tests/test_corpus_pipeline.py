"""End-to-end integration tests for Mathlib corpus ingestion pipeline and curriculum-paced continuous training."""

import os
import shutil
import tempfile
from pathlib import Path
import pytest
from bourbakimesh.corpus import CorpusPipeline, PipelineConfig
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import ReplayBuffer
from bourbakimesh.training import ContinuousTrainingLoop, LoopConfig


@pytest.fixture
def temp_pipeline_dir():
    """Create a temporary directory for pipeline outputs."""
    tmp_dir = tempfile.mkdtemp()
    yield Path(tmp_dir)
    if os.path.exists(tmp_dir):
        shutil.rmtree(tmp_dir)


def test_corpus_pipeline_ingestion_end_to_end(temp_pipeline_dir):
    """Verify CorpusPipeline extracts theorems, indexes into CurriculumManager, and writes JSON."""
    config = PipelineConfig(
        export_dir=temp_pipeline_dir,
        output_corpus=temp_pipeline_dir / "corpus.json",
    )
    pipeline = CorpusPipeline(config)
    curriculum = pipeline.run_ingestion()

    assert (temp_pipeline_dir / "corpus.json").exists()
    summary = curriculum.get_summary()

    assert summary["total_theorems"] >= 8
    assert summary["tier_1_count"] > 0
    assert summary["tier_2_count"] > 0
    assert summary["tier_3_count"] > 0

    # Prime buffer test
    buffer = ReplayBuffer(capacity=50)
    ingested = pipeline.prime_replay_buffer(buffer, max_tier=2, samples_per_tier=3)
    assert ingested > 0
    assert len(buffer) > 0


def test_curriculum_pacing_in_training_loop(temp_pipeline_dir):
    """Verify that ContinuousTrainingLoop properly paces curriculum tiers (1 -> 2 -> 3) across iterations."""
    config = PipelineConfig(
        export_dir=temp_pipeline_dir,
        output_corpus=temp_pipeline_dir / "corpus.json",
    )
    pipeline = CorpusPipeline(config)
    curriculum = pipeline.run_ingestion()

    loop_config = LoopConfig(
        iterations=3,
        self_play_games_per_iter=2,
        tableau_seeds_per_iter=2,
        train_epochs_per_iter=1,
        simulations_per_move=5,
        batch_size=4,
        unroll_steps=2,
        checkpoint_dir=temp_pipeline_dir / "checkpoints",
        feature_dim=16,
        action_space_size=8,
    )

    model_config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(model_config)

    loop = ContinuousTrainingLoop(model=model, config=loop_config, curriculum=curriculum)
    history = loop.run()

    assert len(history) == 3
    # Check progressive tier pacing
    assert history[0].active_curriculum_tier == 1
    assert history[1].active_curriculum_tier == 2
    assert history[2].active_curriculum_tier == 3

    # Check buffer growth and loss computation
    assert history[2].buffer_total_trajectories > history[0].buffer_total_trajectories
    assert history[2].train_loss > 0.0
