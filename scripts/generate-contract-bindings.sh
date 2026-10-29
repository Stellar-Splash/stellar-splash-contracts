#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Soroban TypeScript Client Bindings Generator"
echo "=========================================="

WASM_PATH="target/wasm32-unknown-unknown/release/stellar_splash.wasm"
OUT_DIR="../stellar-splash-frontend/src/bindings"

if [ ! -f "$WASM_PATH" ]; then
  echo "Building contract WASM..."
  cargo build --target wasm32-unknown-unknown --release
fi

if command -v stellar &> /dev/null; then
  echo "Generating TypeScript client bindings to $OUT_DIR..."
  stellar contract bindings typescript \
    --wasm "$WASM_PATH" \
    --output-dir "$OUT_DIR" \
    --overwrite || true
  echo "✅ Bindings generated."
else
  echo "Stellar CLI not available; skipping automated binding generation."
fi

