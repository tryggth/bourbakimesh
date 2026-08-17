"""Command-line interface for BourbakiMesh performance profiling."""

from __future__ import annotations
import argparse
import json
import os
from dataclasses import asdict
from pathlib import Path
from typing import Any, Dict
import torch
from bourbakimesh.benchmarks.bench_engine import BenchmarkReport, BenchmarkRunner
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero


def infer_model_config(state_dict: Dict[str, Any]) -> ArenaEmbeddingConfig:
    """Infer ArenaEmbeddingConfig architectural dimensions from state_dict tensors."""
    hidden_dim = 256
    feature_dim = 32
    if "representation.token_proj.weight" in state_dict:
        w = state_dict["representation.token_proj.weight"]
        hidden_dim = w.shape[0]
        feature_dim = w.shape[1]
    elif "representation.encoder.0.weight" in state_dict:
        w = state_dict["representation.encoder.0.weight"]
        hidden_dim = w.shape[0]
        feature_dim = w.shape[1]

    action_space_size = 64
    latent_dim = 128
    if "dynamics.action_embed.weight" in state_dict:
        w = state_dict["dynamics.action_embed.weight"]
        action_space_size = w.shape[0]
        latent_dim = w.shape[1]

    num_res_blocks = 0
    while f"dynamics.blocks.{num_res_blocks}.fc1.weight" in state_dict:
        num_res_blocks += 1
    if num_res_blocks == 0:
        num_res_blocks = 2

    num_layers = 0
    while f"representation.layers.{num_layers}.ln1.weight" in state_dict:
        num_layers += 1
    use_transformer = num_layers > 0
    if num_layers == 0:
        num_layers = 4

    num_heads = 8
    if "representation.layers.0.self_attn.rel_k_embed.weight" in state_dict:
        head_dim = state_dict["representation.layers.0.self_attn.rel_k_embed.weight"].shape[1]
        num_heads = hidden_dim // max(1, head_dim)

    return ArenaEmbeddingConfig(
        feature_dim=feature_dim,
        latent_dim=latent_dim,
        action_space_size=action_space_size,
        hidden_dim=hidden_dim,
        num_res_blocks=num_res_blocks,
        use_relational_transformer=use_transformer,
        transformer_layers=num_layers,
        transformer_heads=num_heads,
    )


def print_report_table(report: BenchmarkReport) -> None:
    """Pretty-print benchmark results as Markdown tables."""
    print("\n" + "=" * 60)
    print(f"📊 BourbakiMesh Engine Benchmark Report (Device: {report.device})")
    print("=" * 60)

    print("\n### 1. Neural Dynamics Inference Latency")
    print("| Batch Size | Rep (h_θ) p50 | Dyn (g_θ) p50 | Pred (f_θ) p50 | Throughput (fps) |")
    print("|:---:|:---:|:---:|:---:|:---:|")
    for n in report.neural_benchmarks:
        print(
            f"| {n.batch_size:^10} | {n.representation_ms_p50:>10.2f} ms | "
            f"{n.dynamics_ms_p50:>10.2f} ms | {n.prediction_ms_p50:>11.2f} ms | "
            f"{n.total_throughput_fps:>14.1f} |"
        )

    print("\n### 2. Latent MCTS Search Throughput")
    print("| Simulations | Total Latency (ms) | Simulations / Sec | Root Policy Top-1 |")
    print("|:---:|:---:|:---:|:---:|")
    for m in report.mcts_benchmarks:
        print(
            f"| {m.num_simulations:^11} | {m.total_time_ms:>16.2f} ms | "
            f"{m.simulations_per_sec:>15.1f} | {m.proponent_root_value:>17.4f} |"
        )

    print(f"\n🎯 **Compute Simulation Equivalent (CSE) Score:** `{report.cse_score:.3f}x`\n")


def main() -> None:
    parser = argparse.ArgumentParser(description="BourbakiMesh Performance Benchmarking Harness")
    parser.add_argument("--quick", action="store_true", help="Run shortened smoke benchmark")
    parser.add_argument("--model-path", type=str, default=None, help="Path to trained model checkpoint (.pt)")
    parser.add_argument("--simulations", type=int, default=None, help="MCTS simulation count for search benchmark")
    parser.add_argument("--device", type=str, default="cpu", help="Compute device ('cpu', 'cuda')")
    parser.add_argument("--output", type=str, default="reports/latest_benchmark_report.json", help="Report output path")
    args = parser.parse_args()

    model = None
    if args.model_path and Path(args.model_path).exists():
        ckpt = torch.load(args.model_path, map_location=args.device)
        state_dict = ckpt.get("model_state_dict", ckpt)

        if "model_config" in ckpt and isinstance(ckpt["model_config"], dict):
            model_config = ArenaEmbeddingConfig(**ckpt["model_config"])
        else:
            model_config = infer_model_config(state_dict)

        model = BourbakiMuZero(model_config)
        model.load_state_dict(state_dict)
        model.eval()
        print(f"📦 Loaded checkpoint from: {args.model_path} (Latent: {model_config.latent_dim}, Hidden: {model_config.hidden_dim}, Actions: {model_config.action_space_size})")

    runner = BenchmarkRunner(model=model)
    sim_counts = [args.simulations] if args.simulations is not None else None
    report = runner.run_all(quick=args.quick, simulation_counts=sim_counts)

    print_report_table(report)

    output_dir = os.path.dirname(args.output)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(asdict(report), f, indent=2)

    print(f"✅ Report saved to: {args.output}")


if __name__ == "__main__":
    main()
