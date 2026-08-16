"""Command-line interface for BourbakiMesh performance profiling."""

from __future__ import annotations
import argparse
import json
import os
from dataclasses import asdict
from bourbakimesh.benchmarks.bench_engine import BenchmarkReport, BenchmarkRunner


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
    parser.add_argument("--output", type=str, default="benchmarks/reports/latest.json", help="Report output path")
    args = parser.parse_args()

    runner = BenchmarkRunner()
    report = runner.run_all(quick=args.quick)

    print_report_table(report)

    output_dir = os.path.dirname(args.output)
    if output_dir:
        os.makedirs(output_dir, exist_ok=True)

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(asdict(report), f, indent=2)

    print(f"✅ Report saved to: {args.output}")


if __name__ == "__main__":
    main()
