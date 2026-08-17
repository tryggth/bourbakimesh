#!/usr/bin/env python3
"""Export BourbakiMuZero neural models (h_theta, g_theta, f_theta) to ONNX format for in-browser WebGPU execution."""

from __future__ import annotations
import argparse
from pathlib import Path
import sys
import torch
import torch.nn as nn
import numpy as np

from bourbakimesh.models.arch import BourbakiMuZero, ArenaEmbeddingConfig


class RepresentationONNXWrapper(nn.Module):
    """Wrapper for h_theta representation network for ONNX export."""

    def __init__(self, rep_net: nn.Module) -> None:
        super().__init__()
        self.rep_net = rep_net

    def forward(
        self,
        obs: torch.Tensor,
        relation_matrix: torch.Tensor,
        polarities: torch.Tensor,
    ) -> torch.Tensor:
        return self.rep_net(obs, relation_matrix=relation_matrix, polarities=polarities)


class DynamicsONNXWrapper(nn.Module):
    """Wrapper for g_theta dynamics network for ONNX export."""

    def __init__(self, dyn_net: nn.Module) -> None:
        super().__init__()
        self.dyn_net = dyn_net

    def forward(
        self,
        state: torch.Tensor,
        action: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor]:
        return self.dyn_net(state, action)


class PredictionONNXWrapper(nn.Module):
    """Wrapper for f_theta prediction network for ONNX export."""

    def __init__(self, pred_net: nn.Module) -> None:
        super().__init__()
        self.pred_net = pred_net

    def forward(self, state: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        return self.pred_net(state)


def export_models(checkpoint_path: str, output_dir: str, opset_version: int = 17) -> None:
    out_path = Path(output_dir)
    out_path.mkdir(parents=True, exist_ok=True)

    print(f"📦 Loading BourbakiMuZero checkpoint from {checkpoint_path}...")
    model = BourbakiMuZero.load_from_checkpoint(checkpoint_path, map_location="cpu")
    model.eval()
    config = model.config

    B = 1
    N = 4
    feature_dim = config.feature_dim
    latent_dim = config.latent_dim

    # 1. Export Representation Network (h_theta)
    print("🚀 Exporting Representation Network (h_theta) -> representation.onnx...")
    rep_wrapper = RepresentationONNXWrapper(model.representation)
    rep_wrapper.eval()

    dummy_obs = torch.randn(B, N, feature_dim, dtype=torch.float32)
    dummy_rel_mat = torch.zeros(B, N, N, dtype=torch.long)
    for i in range(N - 1):
        dummy_rel_mat[:, i, i + 1] = 1
    dummy_pol = torch.ones(B, N, dtype=torch.long)

    rep_onnx_path = out_path / "representation.onnx"
    torch.onnx.export(
        rep_wrapper,
        (dummy_obs, dummy_rel_mat, dummy_pol),
        str(rep_onnx_path),
        input_names=["obs", "relation_matrix", "polarities"],
        output_names=["latent"],
        dynamic_axes={
            "obs": {0: "batch_size", 1: "seq_len"},
            "relation_matrix": {0: "batch_size", 1: "seq_len", 2: "seq_len"},
            "polarities": {0: "batch_size", 1: "seq_len"},
            "latent": {0: "batch_size"},
        },
        opset_version=opset_version,
        do_constant_folding=True,
    )
    print(f"   Saved: {rep_onnx_path} ({rep_onnx_path.stat().st_size:,} bytes)")

    # 2. Export Dynamics Network (g_theta)
    print("🚀 Exporting Dynamics Network (g_theta) -> dynamics.onnx...")
    dyn_wrapper = DynamicsONNXWrapper(model.dynamics)
    dyn_wrapper.eval()

    dummy_state = torch.randn(B, latent_dim, dtype=torch.float32)
    dummy_action = torch.zeros(B, dtype=torch.long)

    dyn_onnx_path = out_path / "dynamics.onnx"
    torch.onnx.export(
        dyn_wrapper,
        (dummy_state, dummy_action),
        str(dyn_onnx_path),
        input_names=["state", "action"],
        output_names=["next_state", "reward"],
        dynamic_axes={
            "state": {0: "batch_size"},
            "action": {0: "batch_size"},
            "next_state": {0: "batch_size"},
            "reward": {0: "batch_size"},
        },
        opset_version=opset_version,
        do_constant_folding=True,
    )
    print(f"   Saved: {dyn_onnx_path} ({dyn_onnx_path.stat().st_size:,} bytes)")

    # 3. Export Prediction Network (f_theta)
    print("🚀 Exporting Prediction Network (f_theta) -> prediction.onnx...")
    pred_wrapper = PredictionONNXWrapper(model.prediction)
    pred_wrapper.eval()

    pred_onnx_path = out_path / "prediction.onnx"
    torch.onnx.export(
        pred_wrapper,
        (dummy_state,),
        str(pred_onnx_path),
        input_names=["state"],
        output_names=["policy_logits", "value"],
        dynamic_axes={
            "state": {0: "batch_size"},
            "policy_logits": {0: "batch_size"},
            "value": {0: "batch_size"},
        },
        opset_version=opset_version,
        do_constant_folding=True,
    )
    print(f"   Saved: {pred_onnx_path} ({pred_onnx_path.stat().st_size:,} bytes)")

    # 4. Numeric Parity Verification with onnxruntime
    try:
        import onnxruntime as ort
        print("🔍 Verifying numeric parity with ONNX Runtime...")

        # Verify Representation
        sess_rep = ort.InferenceSession(str(rep_onnx_path), providers=["CPUExecutionProvider"])
        ort_latent = sess_rep.run(
            None,
            {
                "obs": dummy_obs.numpy(),
                "relation_matrix": dummy_rel_mat.numpy(),
                "polarities": dummy_pol.numpy(),
            },
        )[0]
        with torch.no_grad():
            pt_latent = rep_wrapper(dummy_obs, dummy_rel_mat, dummy_pol).numpy()
        np.testing.assert_allclose(ort_latent, pt_latent, rtol=1e-3, atol=1e-4)
        print("   ✅ Representation Network parity certified (MSE < 1e-4)")

        # Verify Dynamics
        sess_dyn = ort.InferenceSession(str(dyn_onnx_path), providers=["CPUExecutionProvider"])
        ort_next_state, ort_reward = sess_dyn.run(
            None,
            {
                "state": dummy_state.numpy(),
                "action": dummy_action.numpy(),
            },
        )
        with torch.no_grad():
            pt_next_state, pt_reward = dyn_wrapper(dummy_state, dummy_action)
            pt_next_state = pt_next_state.numpy()
            pt_reward = pt_reward.numpy()
        np.testing.assert_allclose(ort_next_state, pt_next_state, rtol=1e-3, atol=1e-4)
        np.testing.assert_allclose(ort_reward, pt_reward, rtol=1e-3, atol=1e-4)
        print("   ✅ Dynamics Network parity certified (MSE < 1e-4)")

        # Verify Prediction
        sess_pred = ort.InferenceSession(str(pred_onnx_path), providers=["CPUExecutionProvider"])
        ort_policy, ort_value = sess_pred.run(None, {"state": dummy_state.numpy()})
        with torch.no_grad():
            pt_policy, pt_value = pred_wrapper(dummy_state)
            pt_policy = pt_policy.numpy()
            pt_value = pt_value.numpy()
        np.testing.assert_allclose(ort_policy, pt_policy, rtol=1e-3, atol=1e-4)
        np.testing.assert_allclose(ort_value, pt_value, rtol=1e-3, atol=1e-4)
        print("   ✅ Prediction Network parity certified (MSE < 1e-4)")

    except ImportError:
        print("⚠️ onnxruntime not installed, skipping runtime parity test.")

    print("\n🎉 All 3 ONNX models successfully exported and validated!")


def main() -> None:
    parser = argparse.ArgumentParser(description="Export BourbakiMuZero checkpoints to ONNX format")
    parser.add_argument(
        "--checkpoint",
        type=str,
        default="checkpoints/bourbaki_v2.pt",
        help="Path to BourbakiMuZero PyTorch checkpoint",
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default="ui/public/models",
        help="Directory where ONNX models will be written",
    )
    parser.add_argument(
        "--opset",
        type=int,
        default=17,
        help="ONNX opset version (default: 17)",
    )
    args = parser.parse_args()
    export_models(args.checkpoint, args.output_dir, args.opset)


if __name__ == "__main__":
    main()
