"""PyTest integration test suite for Mathlib corpus decompilation and curriculum indexer."""

import json
import pytest
import torch
from torch.utils.data import DataLoader
from bourbakimesh.corpus import CurriculumManager, TheoremEntry
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import ReplayBuffer
from bourbakimesh.training import BourbakiTrainer, ReplayDataset, TrainingConfig


@pytest.fixture
def sample_corpus_payload():
    """Create a sample decompiled corpus payload matching Rust CorpusDataset JSON format."""
    return {
        "theorems": [
            {
                "name": "id_prop",
                "type_expr": "A -> A",
                "trace_depth": 1,
                "branch_count": 1,
                "dependency_count": 0,
            },
            {
                "name": "k_comb",
                "type_expr": "A -> B -> A",
                "trace_depth": 2,
                "branch_count": 1,
                "dependency_count": 0,
            },
            {
                "name": "modus_ponens",
                "type_expr": "A -> (A -> B) -> B",
                "trace_depth": 3,
                "branch_count": 2,
                "dependency_count": 1,
            },
            {
                "name": "and_assoc",
                "type_expr": "(A /\\ B) /\\ C -> A /\\ (B /\\ C)",
                "trace_depth": 5,
                "branch_count": 3,
                "dependency_count": 2,
            },
            {
                "name": "nat_induction_thm",
                "type_expr": "P 0 -> (forall n, P n -> P (n+1)) -> forall n, P n",
                "trace_depth": 8,
                "branch_count": 5,
                "dependency_count": 4,
            },
        ],
        "total_nodes": 45,
    }


def test_curriculum_difficulty_computation_and_tiering():
    """Verify topological complexity score computation and tier classification."""
    manager = CurriculumManager(alpha=1.0, beta=1.5, gamma=0.5)

    # Elementary theorem (Tier 1)
    diff_1 = manager.compute_difficulty(depth=1, branches=1, dependencies=0)
    assert diff_1 == 2.5
    assert manager.assign_tier(diff_1) == 1

    # Multi-step implication (Tier 2)
    diff_2 = manager.compute_difficulty(depth=3, branches=2, dependencies=1)
    assert diff_2 == 6.5
    assert manager.assign_tier(diff_2) == 2

    # High-depth induction (Tier 3)
    diff_3 = manager.compute_difficulty(depth=8, branches=5, dependencies=4)
    assert diff_3 == 17.5
    assert manager.assign_tier(diff_3) == 3


def test_curriculum_manager_ingestion_and_summary(sample_corpus_payload):
    """Verify JSON ingestion and structured curriculum summary partitioning."""
    manager = CurriculumManager()
    count = manager.ingest_corpus_json(sample_corpus_payload)

    assert count == 5
    summary = manager.get_summary()

    assert summary["total_theorems"] == 5
    assert summary["tier_1_count"] == 2  # id_prop, k_comb
    assert summary["tier_2_count"] == 1  # modus_ponens
    assert summary["tier_3_count"] == 2  # and_assoc (diff = 5 + 4.5 + 1 = 10.5), nat_induction
    assert summary["avg_difficulty"] > 0.0


def test_curriculum_replay_buffer_streaming_and_training(sample_corpus_payload):
    """Verify streaming curriculum trajectories into ReplayBuffer and optimizing BourbakiMuZero."""
    manager = CurriculumManager()
    manager.ingest_corpus_json(sample_corpus_payload)

    buffer = ReplayBuffer(capacity=50)
    ingested = manager.populate_replay_buffer(buffer, max_tier=3, samples_per_tier=5)

    assert ingested == 5
    assert len(buffer) == 5

    # Train a step of BourbakiTrainer using curriculum data
    config = ArenaEmbeddingConfig(
        feature_dim=32,
        latent_dim=32,
        action_space_size=16,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(config)
    trainer = BourbakiTrainer(
        model,
        TrainingConfig(batch_size=4, unroll_steps=2, learning_rate=1e-3),
    )

    dataset = ReplayDataset.from_replay_buffer(
        buffer, unroll_steps=2, feature_dim=32, action_space_size=16
    )
    dataloader = DataLoader(
        dataset, batch_size=4, shuffle=True, collate_fn=ReplayDataset.collate_fn
    )

    batch = next(iter(dataloader))
    metrics = trainer.train_step(batch)

    assert metrics.total_loss > 0.0
    assert not torch.isnan(torch.tensor(metrics.total_loss))
