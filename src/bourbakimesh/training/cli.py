"""Continuous training loop command-line interface."""

from __future__ import annotations
import argparse
from pathlib import Path
from bourbakimesh.training.loop import ContinuousTrainingLoop, LoopConfig


def main() -> None:
    parser = argparse.ArgumentParser(description="BourbakiMuZero Continuous Self-Play & Training Loop")
    parser.add_argument("--iterations", type=int, default=3, help="Number of loop iterations")
    parser.add_argument("--games-per-iter", type=int, default=10, help="Self-play games generated per iteration")
    parser.add_argument("--tableau-seeds", type=int, default=10, help="Tableau bootstrap seeds injected per iteration")
    parser.add_argument("--epochs-per-iter", type=int, default=2, help="Training epochs per iteration")
    parser.add_argument("--sims-per-move", type=int, default=50, help="MCTS simulation budget per move")
    parser.add_argument("--batch-size", type=int, default=16, help="Batch size")
    parser.add_argument("--unroll-steps", type=int, default=3, help="Recurrent K-step unrolling depth")
    parser.add_argument("--lr", type=float, default=1e-3, help="Learning rate")
    parser.add_argument("--checkpoint-dir", type=str, default="checkpoints", help="Output directory for checkpoints")
    args = parser.parse_args()

    print("\n" + "=" * 70)
    print("🔁 BourbakiMuZero Closed-Loop Self-Play & Continuous Training")
    print("=" * 70)

    config = LoopConfig(
        iterations=args.iterations,
        self_play_games_per_iter=args.games_per_iter,
        tableau_seeds_per_iter=args.tableau_seeds,
        train_epochs_per_iter=args.epochs_per_iter,
        simulations_per_move=args.sims_per_move,
        batch_size=args.batch_size,
        unroll_steps=args.unroll_steps,
        learning_rate=args.lr,
        checkpoint_dir=Path(args.checkpoint_dir),
    )

    loop = ContinuousTrainingLoop(config=config)

    print(f"⚙️ Configured {config.iterations} iterations ({config.self_play_games_per_iter} games + {config.tableau_seeds_per_iter} seeds per iter)")
    print(f"📂 Checkpoint target: {config.checkpoint_dir.resolve()}\n")

    print("| Iter | Games | Seeds | Total Steps | Train Loss | Policy Loss | Value Loss | CSE Score | Promoted |")
    print("|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|")

    for iter_idx in range(1, config.iterations + 1):
        m = loop.run_iteration(iter_idx)
        cse_str = f"{m.eval_cse_score:.2f}x" if m.eval_cse_score is not None else "N/A"
        prom_str = "✅ Yes" if m.promoted else "➖ No"

        print(
            f"| {m.iteration:^4} | {m.games_generated:^5} | {m.seeds_generated:^5} | "
            f"{m.buffer_total_steps:>11} | {m.train_loss:>10.4f} | {m.policy_loss:>11.4f} | "
            f"{m.value_loss:>10.4f} | {cse_str:^9} | {prom_str:^8} |"
        )

    print("\n🎉 Continuous training completed successfully.")


if __name__ == "__main__":
    main()
