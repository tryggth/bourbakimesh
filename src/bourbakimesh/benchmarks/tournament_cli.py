"""Command-line interface for head-to-head model tournaments and Bayesian Elo evaluation."""

from __future__ import annotations
import argparse
import json
import os
from pathlib import Path
import sys
import torch
from bourbakimesh.benchmarks.tournament import ModelTournament, TournamentProposition, TournamentReport
from bourbakimesh.models import BourbakiMuZero


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="BourbakiMesh Head-to-Head Model Tournament & Bayesian Elo Evaluator"
    )
    parser.add_argument(
        "--models",
        nargs="+",
        required=True,
        help="Paths to model checkpoints (e.g. checkpoints/bourbaki_v0.pt checkpoints/bourbaki_v1.pt)",
    )
    parser.add_argument(
        "--model-names",
        nargs="+",
        default=None,
        help="Custom aliases for models (default: checkpoint filenames without extension)",
    )
    parser.add_argument(
        "--simulations",
        "--sims",
        dest="simulations",
        type=int,
        default=100,
        help="MCTS simulation budget per move (default: 100)",
    )
    parser.add_argument(
        "--max-moves",
        type=int,
        default=16,
        help="Maximum dialogue plies per game (default: 16)",
    )
    parser.add_argument(
        "--manifest",
        type=str,
        default="data/curriculum/curriculum_manifest.json",
        help="Path to curriculum manifest JSON (default: data/curriculum/curriculum_manifest.json)",
    )
    parser.add_argument(
        "--device",
        type=str,
        default="cpu",
        help="Compute device ('cpu' or 'cuda')",
    )
    parser.add_argument(
        "--output",
        "-o",
        type=str,
        default="reports/tournament_v0_v1.json",
        help="Path to export tournament JSON report",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    # Configure intra-op threads on CPU
    if args.device == "cpu":
        threads = os.cpu_count() or 4
        torch.set_num_threads(threads)
        torch.set_num_interop_threads(threads)

    print("\n" + "=" * 80)
    print("⚔️ BourbakiMesh Head-to-Head Tournament & Bayesian Elo Evaluator")
    print("=" * 80)

    # 1. Load Models
    models: dict[str, BourbakiMuZero] = {}
    for idx, model_path_str in enumerate(args.models):
        path = Path(model_path_str)
        if not path.exists():
            print(f"❌ Error: Model checkpoint not found at {path}", file=sys.stderr)
            sys.exit(1)

        name = (
            args.model_names[idx]
            if args.model_names and idx < len(args.model_names)
            else path.stem
        )
        print(f"📦 Loading [{name}] from {path}...")
        model = BourbakiMuZero.load_from_checkpoint(path, map_location=args.device)
        models[name] = model

    if len(models) < 2:
        print("❌ Error: At least 2 models are required to run a tournament.", file=sys.stderr)
        sys.exit(1)

    # 2. Ingest Propositions
    manifest_path = Path(args.manifest)
    propositions = ModelTournament.load_curriculum_propositions(manifest_path)
    print(f"📚 Loaded {len(propositions)} tournament propositions across difficulty tiers.")
    print(f"⚙️ Config: {args.simulations} MCTS sims/move, max {args.max_moves} plies/game, device={args.device}\n")

    # 3. Run Tournament
    tournament = ModelTournament(
        models=models,
        propositions=propositions,
        simulations=args.simulations,
        device=args.device,
        max_moves=args.max_moves,
    )

    print("🏁 Starting paired tournament matches...")
    report = tournament.run_tournament()
    print(f"✅ Completed {report.total_games} games across {len(models)} models.\n")

    # 4. Display Formatted Markdown Summary Table
    print("### 🏆 Tournament Results & Bayesian Elo Ratings\n")
    print("| Model | Elo Rating (±95% CI) | Record (W-L-D) | Win Rate | Tier 1 Solve | Tier 2 Solve | Tier 3 Solve | Mean Plies | Mean Latency |")
    print("| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |")

    # Sort models by descending Elo rating
    sorted_summaries = sorted(
        report.summaries.values(), key=lambda s: s.elo, reverse=True
    )

    for s in sorted_summaries:
        elo_str = f"**{s.elo:.1f}** (±{1.96 * s.elo_stderr:.1f})"
        record_str = f"{s.wins}-{s.losses}-{s.draws}"
        win_rate_str = f"{s.win_rate * 100:.1f}%"
        t1_str = f"{s.tier_solve_rates.get(1, 0.0) * 100:.1f}%"
        t2_str = f"{s.tier_solve_rates.get(2, 0.0) * 100:.1f}%"
        t3_str = f"{s.tier_solve_rates.get(3, 0.0) * 100:.1f}%"
        plies_str = f"{s.mean_plies:.1f}"
        lat_str = f"{s.mean_latency_ms:.1f} ms"

        print(
            f"| `{s.model_name}` | {elo_str} | {record_str} | {win_rate_str} | "
            f"{t1_str} | {t2_str} | {t3_str} | {plies_str} | {lat_str} |"
        )

    print()

    # 5. Export JSON Report
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(report.to_dict(), f, indent=2)

    print(f"📁 Exported tournament report to: {output_path.resolve()}\n")


if __name__ == "__main__":
    main()
