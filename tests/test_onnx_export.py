import sys
from pathlib import Path
import numpy as np
import pytest
import torch

# Ensure project root is on sys.path
sys.path.insert(0, str(Path(__file__).parent.parent))

from bourbakimesh.models.arch import BourbakiMuZero, ArenaEmbeddingConfig
from scripts.export_onnx import (
    RepresentationONNXWrapper,
    DynamicsONNXWrapper,
    PredictionONNXWrapper,
    export_models,
)


@pytest.fixture
def temp_export_dir(tmp_path: Path) -> Path:
    export_dir = tmp_path / "onnx_models"
    export_dir.mkdir(parents=True, exist_ok=True)
    return export_dir


@pytest.fixture
def sample_checkpoint(tmp_path: Path) -> Path:
    ckpt_path = tmp_path / "sample_model.pt"
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=16,
        hidden_dim=64,
        num_res_blocks=2,
        use_relational_transformer=True,
        transformer_layers=2,
        transformer_heads=4,
    )
    model = BourbakiMuZero(config)
    torch.save({"model_state_dict": model.state_dict(), "config": config.model_dump()}, ckpt_path)
    return ckpt_path


def test_onnx_export_and_inference_parity(sample_checkpoint: Path, temp_export_dir: Path) -> None:
    try:
        import onnxruntime as ort
    except ImportError:
        pytest.skip("onnxruntime not installed")

    export_models(str(sample_checkpoint), str(temp_export_dir), opset_version=18)

    rep_path = temp_export_dir / "representation.onnx"
    dyn_path = temp_export_dir / "dynamics.onnx"
    pred_path = temp_export_dir / "prediction.onnx"

    assert rep_path.exists() and rep_path.stat().st_size > 0
    assert dyn_path.exists() and dyn_path.stat().st_size > 0
    assert pred_path.exists() and pred_path.stat().st_size > 0

    model = BourbakiMuZero.load_from_checkpoint(str(sample_checkpoint), map_location="cpu")
    model.eval()

    B = 1
    N = 3
    feature_dim = model.config.feature_dim
    latent_dim = model.config.latent_dim

    # Test Representation Parity
    dummy_obs = torch.randn(B, N, feature_dim, dtype=torch.float32)
    dummy_rel_mat = torch.zeros(B, N, N, dtype=torch.long)
    dummy_pol = torch.ones(B, N, dtype=torch.long)

    sess_rep = ort.InferenceSession(str(rep_path), providers=["CPUExecutionProvider"])
    ort_latent = sess_rep.run(
        None,
        {
            "obs": dummy_obs.numpy(),
            "relation_matrix": dummy_rel_mat.numpy(),
            "polarities": dummy_pol.numpy(),
        },
    )[0]

    with torch.no_grad():
        pt_latent = model.representation(dummy_obs, relation_matrix=dummy_rel_mat, polarities=dummy_pol).numpy()

    np.testing.assert_allclose(ort_latent, pt_latent, rtol=1e-3, atol=1e-4)

    # Test Dynamics Parity
    dummy_state = torch.randn(B, latent_dim, dtype=torch.float32)
    dummy_action = torch.tensor([[1]], dtype=torch.long)

    sess_dyn = ort.InferenceSession(str(dyn_path), providers=["CPUExecutionProvider"])
    ort_next_state, ort_reward = sess_dyn.run(
        None,
        {
            "state": dummy_state.numpy(),
            "action": dummy_action.squeeze(-1).numpy(),
        },
    )

    with torch.no_grad():
        pt_next_state, pt_reward = model.dynamics(dummy_state, dummy_action)
        pt_next_state = pt_next_state.numpy()
        pt_reward = pt_reward.numpy()

    np.testing.assert_allclose(ort_next_state, pt_next_state, rtol=1e-3, atol=1e-4)
    np.testing.assert_allclose(ort_reward, pt_reward, rtol=1e-3, atol=1e-4)

    # Test Prediction Parity
    sess_pred = ort.InferenceSession(str(pred_path), providers=["CPUExecutionProvider"])
    ort_policy, ort_value = sess_pred.run(None, {"state": dummy_state.numpy()})

    with torch.no_grad():
        pt_policy, pt_value = model.prediction(dummy_state)
        pt_policy = pt_policy.numpy()
        pt_value = pt_value.numpy()

    np.testing.assert_allclose(ort_policy, pt_policy, rtol=1e-3, atol=1e-4)
    np.testing.assert_allclose(ort_value, pt_value, rtol=1e-3, atol=1e-4)
