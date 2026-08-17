"""Closed-loop self-play and continuous training orchestrator with progressive curriculum pacing."""

from __future__ import annotations
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional
import torch
from torch.utils.data import DataLoader
from bourbakimesh.benchmarks.bench_engine import BenchmarkReport, BenchmarkRunner
from bourbakimesh.benchmarks.tournament import ModelTournament
from bourbakimesh.bootstrap import SeedCorpusGenerator
from bourbakimesh.corpus.curriculum import CurriculumManager
from bourbakimesh.latent_mcts import MCTSConfig
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import ReplayBuffer, SelfPlayWorker
from bourbakimesh.training.dataset import ReplayDataset
from bourbakimesh.training.trainer import BourbakiTrainer, TrainStepResult, TrainingConfig


@dataclass
class LoopConfig:
    """Configuration for closed-loop self-play and continuous training."""

    iterations: int = 5
    self_play_games_per_iter: int = 20
    tableau_seeds_per_iter: int = 10
    train_epochs_per_iter: int = 3
    simulations_per_move: int = 50
    batch_size: int = 16
    unroll_steps: int = 3
    learning_rate: float = 1e-3
    checkpoint_dir: Path = field(default_factory=lambda: Path("checkpoints"))
    initial_checkpoint: Optional[str | Path] = None
    eval_bench_interval: int = 1
    buffer_capacity: int = 1000
    target_temperature: float = 0.5
    verified_boost: float = 5.0
    champion_gating: bool = True
    gating_matches: int = 6
    feature_dim: int = 32
    action_space_size: int = 16


@dataclass
class IterationMetrics:
    """Metrics collected across a single continuous training iteration."""

    iteration: int
    games_generated: int
    seeds_generated: int
    buffer_total_trajectories: int
    buffer_total_steps: int
    train_loss: float
    policy_loss: float
    value_loss: float
    reward_loss: float
    active_curriculum_tier: int = 1
    eval_cse_score: Optional[float] = None
    eval_sims_per_sec: Optional[float] = None
    promoted: bool = False
    gating_win_rate: Optional[float] = None


class ContinuousTrainingLoop:
    """Orchestrates closed-loop self-play generation, training optimization, and curriculum-paced evaluation."""

    def __init__(
        self,
        model: Optional[BourbakiMuZero] = None,
        config: Optional[LoopConfig] = None,
        curriculum: Optional[CurriculumManager] = None,
    ) -> None:
        self.config = config or LoopConfig()
        self.checkpoint_dir = Path(self.config.checkpoint_dir)
        self.checkpoint_dir.mkdir(parents=True, exist_ok=True)
        device = "cuda" if torch.cuda.is_available() else "cpu"

        if model is None:
            if self.config.initial_checkpoint and Path(self.config.initial_checkpoint).exists():
                self.model = BourbakiMuZero.load_from_checkpoint(
                    self.config.initial_checkpoint,
                    map_location=device,
                )
                self.config.feature_dim = self.model.config.feature_dim
                self.config.action_space_size = self.model.config.action_space_size
            else:
                model_config = ArenaEmbeddingConfig(
                    feature_dim=self.config.feature_dim,
                    latent_dim=64,
                    action_space_size=self.config.action_space_size,
                    hidden_dim=128,
                    num_res_blocks=2,
                )
                self.model = BourbakiMuZero(model_config)
        else:
            self.model = model

        self.buffer = ReplayBuffer(
            capacity=self.config.buffer_capacity,
            verified_boost=self.config.verified_boost,
        )
        self.generator = SeedCorpusGenerator()
        self.curriculum = curriculum

        mcts_config = MCTSConfig(
            num_simulations=self.config.simulations_per_move,
            exploration_fraction=0.25,
        )
        self.worker = SelfPlayWorker(
            self.model,
            mcts_config,
            target_temperature=self.config.target_temperature,
        )

        trainer_config = TrainingConfig(
            batch_size=self.config.batch_size,
            learning_rate=self.config.learning_rate,
            unroll_steps=self.config.unroll_steps,
            epochs=self.config.train_epochs_per_iter,
        )
        self.trainer = BourbakiTrainer(self.model, trainer_config)

        self.best_loss = float("inf")
        self.history: List[IterationMetrics] = []

    def get_active_curriculum_tier(self, iter_idx: int) -> int:
        """Determine unlocked curriculum difficulty tier based on training loop progression."""
        progress = iter_idx / max(1, self.config.iterations)
        if progress <= 0.34:
            return 1
        elif progress <= 0.67:
            return 2
        else:
            return 3

    def run_iteration(self, iter_idx: int) -> IterationMetrics:
        """Execute one complete cycle: Seed Injection -> Self-Play -> Train -> Eval -> Promote."""
        active_tier = self.get_active_curriculum_tier(iter_idx)

        # 1. Synthesize Seed Data from Semantic Tableau
        seeds_added = 0
        if self.config.tableau_seeds_per_iter > 0:
            seeds_added = self.generator.populate_replay_buffer(
                self.buffer,
                count=self.config.tableau_seeds_per_iter,
                feature_dim=self.config.feature_dim,
                action_space_size=self.config.action_space_size,
            )

        # 2. Inject Curriculum Demonstrations up to active tier
        if self.curriculum is not None:
            curriculum_added = self.curriculum.populate_replay_buffer(
                self.buffer,
                max_tier=active_tier,
                samples_per_tier=max(2, self.config.tableau_seeds_per_iter // 2),
            )
            seeds_added += curriculum_added

        # 3. Generate Self-Play Trajectories with Current Model
        games_added = 0
        for _ in range(self.config.self_play_games_per_iter):
            traj = self.worker.play_game(
                max_moves=15,
                num_simulations=self.config.simulations_per_move,
            )
            if len(traj.actions) > 0:
                self.buffer.push(traj)
                games_added += 1

        # 4. Create Dataset from Updated Experience Buffer
        dataset = ReplayDataset.from_replay_buffer(
            self.buffer,
            unroll_steps=self.config.unroll_steps,
            feature_dim=self.config.feature_dim,
            action_space_size=self.config.action_space_size,
        )

        # Handle edge-case of empty dataset gracefully
        if len(dataset) == 0:
            return IterationMetrics(
                iteration=iter_idx,
                games_generated=games_added,
                seeds_generated=seeds_added,
                buffer_total_trajectories=len(self.buffer),
                buffer_total_steps=self.buffer.total_steps(),
                train_loss=0.0,
                policy_loss=0.0,
                value_loss=0.0,
                reward_loss=0.0,
                active_curriculum_tier=active_tier,
                promoted=False,
            )

        dataloader = DataLoader(
            dataset,
            batch_size=min(self.config.batch_size, len(dataset)),
            shuffle=True,
            collate_fn=ReplayDataset.collate_fn,
            drop_last=False,
        )

        # 5. Neural Optimization
        epoch_res = TrainStepResult(0.0, 0.0, 0.0, 0.0)
        for _ in range(self.config.train_epochs_per_iter):
            epoch_res = self.trainer.train_epoch(dataloader)

        # 6. Benchmark Evaluation
        eval_cse: Optional[float] = None
        eval_sims: Optional[float] = None

        if iter_idx % self.config.eval_bench_interval == 0:
            bench_runner = BenchmarkRunner(self.model)
            report = bench_runner.run_all(quick=True)
            eval_cse = report.cse_score
            if report.mcts_benchmarks:
                eval_sims = report.mcts_benchmarks[0].simulations_per_sec

        # 7. Checkpoint Management & Model Promotion
        promoted = False
        gating_win_rate: Optional[float] = None
        iter_ckpt_path = self.checkpoint_dir / f"checkpoint_iter_{iter_idx}.pt"
        self.trainer.save_checkpoint(iter_ckpt_path, extra_meta={"iteration": iter_idx})

        best_ckpt_path = self.checkpoint_dir / "best_model.pt"

        if iter_idx == 1 and not best_ckpt_path.exists():
            self.best_loss = epoch_res.total_loss
            promoted = True
            self.trainer.save_checkpoint(
                best_ckpt_path,
                extra_meta={"iteration": iter_idx, "best_loss": self.best_loss},
            )
        elif self.config.champion_gating and best_ckpt_path.exists():
            try:
                device_str = str(self.trainer.device)
                incumbent = BourbakiMuZero.load_from_checkpoint(
                    best_ckpt_path, map_location=device_str
                )
                props = ModelTournament.default_propositions()[: max(1, self.config.gating_matches // 2)]
                tourney = ModelTournament(
                    {"candidate": self.model, "incumbent": incumbent},
                    propositions=props,
                    simulations=min(50, self.config.simulations_per_move),
                    device=device_str,
                )
                report = tourney.run_tournament()
                cand_summary = report.summaries.get("candidate")
                if cand_summary:
                    gating_win_rate = cand_summary.win_rate
                    if cand_summary.win_rate > 0.5:
                        self.best_loss = epoch_res.total_loss
                        promoted = True
                        self.trainer.save_checkpoint(
                            best_ckpt_path,
                            extra_meta={
                                "iteration": iter_idx,
                                "best_loss": self.best_loss,
                                "gating_win_rate": gating_win_rate,
                            },
                        )
            except Exception:
                if epoch_res.total_loss < self.best_loss:
                    self.best_loss = epoch_res.total_loss
                    promoted = True
                    self.trainer.save_checkpoint(
                        best_ckpt_path,
                        extra_meta={"iteration": iter_idx, "best_loss": self.best_loss},
                    )
        else:
            if epoch_res.total_loss < self.best_loss:
                self.best_loss = epoch_res.total_loss
                promoted = True
                self.trainer.save_checkpoint(
                    best_ckpt_path,
                    extra_meta={"iteration": iter_idx, "best_loss": self.best_loss},
                )

        metrics = IterationMetrics(
            iteration=iter_idx,
            games_generated=games_added,
            seeds_generated=seeds_added,
            buffer_total_trajectories=len(self.buffer),
            buffer_total_steps=self.buffer.total_steps(),
            train_loss=epoch_res.total_loss,
            policy_loss=epoch_res.policy_loss,
            value_loss=epoch_res.value_loss,
            reward_loss=epoch_res.reward_loss,
            active_curriculum_tier=active_tier,
            eval_cse_score=eval_cse,
            eval_sims_per_sec=eval_sims,
            promoted=promoted,
            gating_win_rate=gating_win_rate,
        )

        self.history.append(metrics)
        return metrics

    def run(self) -> List[IterationMetrics]:
        """Execute full training loop over all configured iterations."""
        for iter_idx in range(1, self.config.iterations + 1):
            self.run_iteration(iter_idx)
        return self.history
