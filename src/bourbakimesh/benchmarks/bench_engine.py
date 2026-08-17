"""Automated neural latency, MCTS throughput, and Compute Simulation Equivalent (CSE) benchmark engine."""

from __future__ import annotations
import time
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional
import numpy as np
import torch
from bourbakimesh.latent_mcts import LatentMCTS, MCTSConfig
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero


@dataclass
class NeuralBenchResult:
    batch_size: int
    representation_ms_p50: float
    dynamics_ms_p50: float
    prediction_ms_p50: float
    total_throughput_fps: float


@dataclass
class MCTSBenchResult:
    num_simulations: int
    total_time_ms: float
    simulations_per_sec: float
    proponent_root_value: float


@dataclass
class BenchmarkReport:
    timestamp: float
    device: str
    neural_benchmarks: List[NeuralBenchResult] = field(default_factory=list)
    mcts_benchmarks: List[MCTSBenchResult] = field(default_factory=list)
    cse_score: float = 1.0


class BenchmarkRunner:
    """Comprehensive performance benchmark runner for BourbakiMuZero and Latent MCTS."""

    def __init__(self, model: Optional[BourbakiMuZero] = None) -> None:
        self.device = "cuda" if torch.cuda.is_available() else "cpu"
        if model is not None:
            self.model = model.to(self.device)
        else:
            config = ArenaEmbeddingConfig(
                feature_dim=32,
                latent_dim=64,
                action_space_size=16,
                hidden_dim=128,
                num_res_blocks=2,
            )
            self.model = BourbakiMuZero(config).to(self.device)

    def bench_neural_inference(
        self,
        batch_sizes: Optional[List[int]] = None,
        iterations: int = 30,
    ) -> List[NeuralBenchResult]:
        """Benchmark forward pass latency for h_theta, g_theta, and f_theta."""
        if batch_sizes is None:
            batch_sizes = [1, 16, 64]

        results: List[NeuralBenchResult] = []
        self.model.eval()

        with torch.no_grad():
            for bsz in batch_sizes:
                obs = torch.randn(bsz, self.model.config.feature_dim, device=self.device)
                action = torch.zeros(bsz, dtype=torch.long, device=self.device)

                # 1. Warm-up
                for _ in range(5):
                    s0 = self.model.representation(obs)
                    s1, r = self.model.dynamics(s0, action)
                    p, v = self.model.prediction(s1)

                # 2. Measure Representation (h_theta)
                rep_times = []
                for _ in range(iterations):
                    t0 = time.perf_counter()
                    s0 = self.model.representation(obs)
                    t1 = time.perf_counter()
                    rep_times.append((t1 - t0) * 1000.0)

                # 3. Measure Dynamics (g_theta)
                dyn_times = []
                for _ in range(iterations):
                    t0 = time.perf_counter()
                    s1, _ = self.model.dynamics(s0, action)
                    t1 = time.perf_counter()
                    dyn_times.append((t1 - t0) * 1000.0)

                # 4. Measure Prediction (f_theta)
                pred_times = []
                for _ in range(iterations):
                    t0 = time.perf_counter()
                    _, _ = self.model.prediction(s1)
                    t1 = time.perf_counter()
                    pred_times.append((t1 - t0) * 1000.0)

                p50_rep = float(np.median(rep_times))
                p50_dyn = float(np.median(dyn_times))
                p50_pred = float(np.median(pred_times))
                total_step_ms = p50_dyn + p50_pred
                fps = (bsz / (total_step_ms / 1000.0)) if total_step_ms > 0 else 0.0

                results.append(
                    NeuralBenchResult(
                        batch_size=bsz,
                        representation_ms_p50=round(p50_rep, 3),
                        dynamics_ms_p50=round(p50_dyn, 3),
                        prediction_ms_p50=round(p50_pred, 3),
                        total_throughput_fps=round(fps, 1),
                    )
                )

        return results

    def bench_mcts_throughput(
        self,
        simulation_counts: Optional[List[int]] = None,
        runs: int = 3,
    ) -> List[MCTSBenchResult]:
        """Benchmark MCTS search throughput (simulations/sec)."""
        if simulation_counts is None:
            simulation_counts = [50, 100, 250]

        results: List[MCTSBenchResult] = []
        obs = torch.randn(1, self.model.config.feature_dim, device=self.device)

        for sims in simulation_counts:
            config = MCTSConfig(num_simulations=sims, exploration_fraction=0.0)
            mcts = LatentMCTS(self.model, config)

            run_times = []
            values = []

            for _ in range(runs):
                t0 = time.perf_counter()
                policy = mcts.search(obs, current_player=1, num_simulations=sims, is_latent=False)
                t1 = time.perf_counter()
                run_times.append(t1 - t0)
                values.append(float(policy[0]))

            avg_time_ms = float(np.mean(run_times)) * 1000.0
            sims_per_sec = (sims / (avg_time_ms / 1000.0)) if avg_time_ms > 0 else 0.0

            results.append(
                MCTSBenchResult(
                    num_simulations=sims,
                    total_time_ms=round(avg_time_ms, 2),
                    simulations_per_sec=round(sims_per_sec, 1),
                    proponent_root_value=round(float(np.mean(values)), 4),
                )
            )

        return results

    def calculate_cse(self, mcts_results: List[MCTSBenchResult]) -> float:
        """Compute Simulation Equivalent (CSE) metric calibrated against baseline search."""
        if not mcts_results:
            return 1.0
        avg_throughput = np.mean([r.simulations_per_sec for r in mcts_results])
        return round(float(avg_throughput / 500.0), 3)

    def run_all(
        self,
        quick: bool = False,
        simulation_counts: Optional[List[int]] = None,
        batch_sizes: Optional[List[int]] = None,
    ) -> BenchmarkReport:
        """Run complete benchmark suite and return structured report."""
        bsz = batch_sizes or ([1, 16] if quick else [1, 16, 64])
        sims = simulation_counts or ([50, 100] if quick else [50, 100, 250])

        neural_res = self.bench_neural_inference(batch_sizes=bsz, iterations=15 if quick else 30)
        mcts_res = self.bench_mcts_throughput(simulation_counts=sims, runs=2 if quick else 3)
        cse = self.calculate_cse(mcts_res)

        return BenchmarkReport(
            timestamp=time.time(),
            device=self.device,
            neural_benchmarks=neural_res,
            mcts_benchmarks=mcts_res,
            cse_score=cse,
        )
