#!/usr/bin/env bash
set -euo pipefail

echo "======================================================================"
echo "🚀 BOURBAKIMESH LIVE END-TO-END PIPELINE VERIFICATION"
echo "======================================================================"

# Step 1: Export Mathlib AST & Decompile to Game Arenas
echo "[1/6] Ingesting and decompiling Mathlib curriculum propositions..."
.venv/bin/python -m bourbakimesh.corpus.pipeline \
  --export-json data/mathlib_raw.json \
  --output-dir data/curriculum \
  --validate

# Step 2: Run Multi-Target Emitter Verification
echo "[2/6] Verifying universal strategy emitters (Lean 4, Coq, Isabelle, Dedukti)..."
cargo test -p bourbaki-kernel --test multi_target_tests

# Step 3: Run P2P Daemon Loopback Task-Claim & Proof-Solving Test
echo "[3/6] Running P2P Daemon Task-Claim and Proof-Gossip loopback test..."
cargo test -p bourbaki-mesh --test daemon_integration_tests -- --nocapture

# Step 4: Run 5-Node P2P Swarm Byzantine Consensus Benchmark
echo "[4/6] Running 5-Node Swarm Byzantine Attestation Benchmark..."
cargo test -p bourbaki-mesh --test cluster_benchmarks -- --nocapture

# Step 5: Run FastAPI Telemetry Server & WebSocket Test
echo "[5/6] Testing REST & WebSocket Telemetry Server..."
.venv/bin/pytest tests/test_api_server.py

# Step 6: Full Workspace Multi-Toolchain Verification Gate
echo "[6/6] Executing full polyglot verification matrix (Rust, Lean 4, Python)..."
cargo test --workspace
.venv/bin/pytest tests/
cd lean_target && lake build
cd ..

echo "======================================================================"
echo "🎉 END-TO-END VERIFICATION COMPLETE: ALL 132+ TESTS GREEN"
echo "======================================================================"
