#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Soroban WASM Binary Size Profiler"
echo "=========================================="

if [ -f "target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm" ]; then
  WASM_FILE="target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm"
elif [ -f "target/wasm32-unknown-unknown/release/stellar_splash.wasm" ]; then
  WASM_FILE="target/wasm32-unknown-unknown/release/stellar_splash.wasm"
else
  WASM_FILE="target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm"
fi

if [ ! -f "$WASM_FILE" ]; then
  echo "Building optimized WASM target..."
  cargo build --target wasm32-unknown-unknown --release
fi

if [ ! -f "$WASM_FILE" ]; then
  if [ -f "target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm" ]; then
    WASM_FILE="target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm"
  elif [ -f "target/wasm32-unknown-unknown/release/stellar_splash.wasm" ]; then
    WASM_FILE="target/wasm32-unknown-unknown/release/stellar_splash.wasm"
  else
    echo "Error: WASM target file not found at $WASM_FILE"
    exit 1
  fi
fi

FILE_SIZE_BYTES=$(wc -c < "$WASM_FILE" | tr -d ' ')
FILE_SIZE_KB=$(( FILE_SIZE_BYTES / 1024 ))

echo "WASM Binary: $WASM_FILE"
echo "Size: ${FILE_SIZE_BYTES} bytes (~${FILE_SIZE_KB} KB)"

# Soroban maximum contract limits (e.g., 64KB target, 128KB max)
MAX_ALLOWED_KB=128

if [ "$FILE_SIZE_KB" -gt "$MAX_ALLOWED_KB" ]; then
  echo "Error: WASM size (${FILE_SIZE_KB} KB) exceeds maximum allowed limit (${MAX_ALLOWED_KB} KB)"
  exit 1
fi

echo "WASM size is within acceptable threshold (< ${MAX_ALLOWED_KB} KB)."
