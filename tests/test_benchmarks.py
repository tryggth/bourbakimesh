"""PyTest test suite for neural latency, MCTS throughput, and CSE benchmarking harness."""

import os
import tempfile
import pytest
from bourbakimesh.benchmarks import BenchmarkRunner
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero


def test_benchmark_runner_quick():
    """Verify BenchmarkRunner executes quickly and outputs valid throughput metrics."""
    config = ArenaEmbeddingConfig(
        feature_dim=16,
        latent_dim=32,
        action_space_size=8,
        hidden_dim=32,
        num_res_blocks=1,
    )
    model = BourbakiMuZero(config)
    runner = BenchmarkRunner(model=model)

    report = runner.run_all(quick=True)

    assert len(report.neural_benchmarks) > 0
    assert all(n.total_throughput_fps > 0 for n in report.neural_benchmarks)
    assert len(report.mcts_benchmarks) > 0
    assert all(m.simulations_per_sec > 0 for m in report.mcts_benchmarks)
    assert report.cse_score > 0.0


def test_benchmark_cli_invocation():
    """Verify CLI entrypoint generates structured JSON output file."""
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        from bourbakimesh.benchmarks.cli import print_report_table
        runner = BenchmarkRunner()
        report = runner.run_all(quick=True)
        print_report_table(report)
        assert len(report.neural_benchmarks) > 0
    finally:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)
