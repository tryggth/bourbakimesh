"""Training orchestration CLI for BourbakiMuZero."""

from __future__ import annotations
import argparse
from pathlib import Path
import torch
from torch.utils.data import DataLoader
from bourbakimesh.bootstrap import SeedCorpusGenerator
from bourbakimesh.models import ArenaEmbeddingConfig, BourbakiMuZero
from bourbakimesh.self_play import ReplayBuffer
from bourbakimesh.training.dataset import ReplayDataset
from bourbakimesh.training.trainer import BourbakiTrainer, TrainingConfig


def main() -> None:
    parser = argparse.ArgumentParser(description="BourbakiMuZero Training Pipeline")
    parser.add_argument("--epochs", type=int, default=5, help="Number of training epochs")
    parser.add_argument("--batch-size", type=int, default=16, help="Batch size per step")
    parser.add_argument("--unroll-steps", type=int, default=3, help="K-step recurrent unrolling")
    parser.add_argument("--lr", type=float, default=1e-3, help="Learning rate")
    parser.add_argument("--checkpoint-dir", type=str, default="checkpoints", help="Directory to save checkpoints")
    parser.add_argument("--seed-samples", type=int, default=64, help="Number of bootstrap seed problems to generate")
    args = parser.parse_args()

    print("\n" + "=" * 60)
    print("🚀 Initializing BourbakiMuZero Training Pipeline")
    print("=" * 60)

    # 1. Model Configuration
    model_config = ArenaEmbeddingConfig(
        feature_dim=32,
        latent_dim=64,
        action_space_size=16,
        hidden_dim=128,
        num_res_blocks=2,
    )
    model = BourbakiMuZero(model_config)

    # 2. Synthesize Seed Data
    print(f"📦 Synthesizing {args.seed_samples} bootstrap seed trajectories via Semantic Tableau...")
    generator = SeedCorpusGenerator()
    buffer = ReplayBuffer(capacity=max(100, args.seed_samples * 2))
    inserted = generator.populate_replay_buffer(
        buffer,
        count=args.seed_samples,
        feature_dim=model_config.feature_dim,
        action_space_size=model_config.action_space_size,
    )
    print(f"✅ Ingested {inserted} verified dialogue trajectories into ReplayBuffer.")

    # 3. Create Dataset & DataLoader
    dataset = ReplayDataset.from_replay_buffer(
        buffer,
        unroll_steps=args.unroll_steps,
        feature_dim=model_config.feature_dim,
        action_space_size=model_config.action_space_size,
    )
    dataloader = DataLoader(
        dataset,
        batch_size=args.batch_size,
        shuffle=True,
        collate_fn=ReplayDataset.collate_fn,
        drop_last=False,
    )

    # 4. Trainer Initialization
    train_config = TrainingConfig(
        batch_size=args.batch_size,
        learning_rate=args.lr,
        unroll_steps=args.unroll_steps,
        epochs=args.epochs,
    )
    trainer = BourbakiTrainer(model, train_config)
    print(f"⚙️ Training device: {trainer.device} | Mixed precision: {trainer.scaler.is_enabled()}")

    # 5. Training Loop
    print("\nStarting training epochs:")
    print("| Epoch | Total Loss | Policy Loss | Value Loss | Reward Loss |")
    print("|:---:|:---:|:---:|:---:|:---:|")

    for epoch in range(1, args.epochs + 1):
        res = trainer.train_epoch(dataloader)
        print(
            f"| {epoch:^5} | {res.total_loss:>10.4f} | {res.policy_loss:>11.4f} | "
            f"{res.value_loss:>10.4f} | {res.reward_loss:>11.4f} |"
        )

    # 6. Save Final Checkpoint
    checkpoint_dir = Path(args.checkpoint_dir)
    checkpoint_dir.mkdir(parents=True, exist_ok=True)
    final_path = checkpoint_dir / "model_final.pt"
    trainer.save_checkpoint(final_path)
    print(f"\n💾 Saved trained model checkpoint to: {final_path}")


if __name__ == "__main__":
    main()
