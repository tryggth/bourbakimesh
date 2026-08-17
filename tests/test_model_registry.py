"""Tests for Multi-Tier Model Registry, SHA-256 Checksums, and ModelPuller CLI."""

from pathlib import Path
import sys
import tempfile
import pytest
import torch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bourbakimesh.models import BourbakiMuZero, ArenaEmbeddingConfig
from bourbakimesh.models.pull import (
    ModelPuller,
    compute_file_sha256,
    load_checksum_manifest,
    pull_checkpoint,
)
from scripts.sync_huggingface import generate_model_card


def test_checksum_manifest_loading():
    manifest = load_checksum_manifest("checkpoints/CHECKSUMS.txt")
    assert "bourbaki_v0.pt" in manifest
    assert "bourbaki_v1.pt" in manifest
    assert "bourbaki_v2.pt" in manifest
    assert len(manifest["bourbaki_v2.pt"]) == 64


def test_model_puller_local_disk_cache(tmp_path):
    # 1. Create a dummy model checkpoint
    model = BourbakiMuZero(ArenaEmbeddingConfig(hidden_dim=32, latent_dim=16, feature_dim=8))
    ckpt_path = tmp_path / "test_model_v1.pt"
    torch.save({"model_state_dict": model.state_dict()}, ckpt_path)

    # 2. Compute SHA-256 and write manifest
    digest = compute_file_sha256(ckpt_path)
    manifest_path = tmp_path / "CHECKSUMS.txt"
    manifest_path.write_text(f"{digest}  test_model_v1.pt\n")

    # 3. Pull via ModelPuller
    puller = ModelPuller(output_dir=tmp_path, manifest_path=manifest_path)
    pulled_path = puller.pull("test_model_v1.pt")

    assert pulled_path.exists()
    assert pulled_path == ckpt_path
    assert puller.verify_integrity(pulled_path)


def test_model_puller_corrupt_file_detection(tmp_path):
    manifest_path = tmp_path / "CHECKSUMS.txt"
    manifest_path.write_text("0000000000000000000000000000000000000000000000000000000000000000  corrupt.pt\n")

    corrupt_file = tmp_path / "corrupt.pt"
    corrupt_file.write_bytes(b"Corrupted weights payload")

    puller = ModelPuller(output_dir=tmp_path, manifest_path=manifest_path)
    assert not puller.verify_integrity(corrupt_file)


def test_huggingface_card_generation(tmp_path):
    report_path = Path("reports/tournament_v0_v1_v2.json")
    card = generate_model_card(tmp_path, report_path)
    assert "BourbakiMuZero" in card
    assert "bourbaki_v0.pt" in card
    assert "bourbaki_v1.pt" in card
    assert "bourbaki_v2.pt" in card
    assert "Zero-Trust Lean 4 Kernel Soundness" in card
