#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Building BourbakiMesh WASM Proof Kernel ==="
export PATH="$HOME/.cargo/bin:$PATH"

mkdir -p "$ROOT_DIR/ui/src/wasm/kernel"

if command -v wasm-pack &> /dev/null; then
    wasm-pack build --target web "$ROOT_DIR/crates/kernel-wasm" --out-dir "$ROOT_DIR/ui/src/wasm/kernel"
    echo "✅ WASM Kernel build completed with wasm-pack."
else
    echo "⚠️ wasm-pack not found in PATH, using cargo build..."
    cargo build --target wasm32-unknown-unknown --manifest-path "$ROOT_DIR/crates/kernel-wasm/Cargo.toml" --release
fi
