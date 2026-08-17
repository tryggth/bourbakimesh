#!/usr/bin/env bash
set -euo pipefail

# Calibrate CPU threading
export OMP_NUM_THREADS=$(nproc)
export MKL_NUM_THREADS=$(nproc)
export OPENBLAS_NUM_THREADS=$(nproc)

echo "========================================================"
echo " BourbakiMesh Overnight Training Run"
echo " Start Time: $(date)"
echo " Host CPU Cores: $(nproc)"
echo "========================================================"

mkdir -p checkpoints reports logs

# 1. Execute Continuous Training Loop
.venv/bin/python -m bourbakimesh.training.cli \
  --iterations 60 \
  --games-per-iter 8 \
  --tableau-seeds 10 \
  --epochs-per-iter 3 \
  --batch-size 32 \
  --simulations-per-move 80 \
  --curriculum-dir data/curriculum \
  --checkpoint-dir checkpoints \
  --device cpu \
  2>&1 | tee -a logs/train_overnight.log

# 2. Promote Top Checkpoint to bourbaki_v0.pt
echo "[$(date)] Promoting best checkpoint to bourbaki_v0.pt..."
if [ -f checkpoints/best_model.pt ]; then
  cp checkpoints/best_model.pt checkpoints/bourbaki_v0.pt
elif [ -f checkpoints/checkpoint_iter_1.pt ]; then
  cp checkpoints/checkpoint_iter_1.pt checkpoints/bourbaki_v0.pt
fi

# 3. Execute Post-Training Benchmark Evaluation
echo "[$(date)] Running benchmark evaluation suite on CPU..."
.venv/bin/python -m bourbakimesh.benchmarks.cli \
  --model-path checkpoints/bourbaki_v0.pt \
  --simulations 100 \
  --device cpu \
  --output reports/v0_benchmark_report.json \
  2>&1 | tee -a logs/train_overnight.log

echo "========================================================"
echo " Training Complete: $(date)"
echo " Checkpoint Ready: checkpoints/bourbaki_v0.pt"
echo " Benchmark Report: reports/v0_benchmark_report.json"
echo "========================================================"
