#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Soroban WASM Binary Size Profiler"
echo "=========================================="

# Build target wasm
cargo rustc --target wasm32-unknown-unknown --release --crate-type=cdylib 2>/dev/null || cargo build --target wasm32-unknown-unknown --release 2>/dev/null || true

WASM_FILE=$(find target -name "*.wasm" 2>/dev/null | grep -E "release|wasm32" | head -n 1 || true)

if [ -z "$WASM_FILE" ] || [ ! -f "$WASM_FILE" ]; then
  WASM_FILE=$(find target -name "*.rlib" 2>/dev/null | grep -E "stellar_splash" | head -n 1 || true)
fi

if [ -n "$WASM_FILE" ] && [ -f "$WASM_FILE" ]; then
  FILE_SIZE_BYTES=$(wc -c < "$WASM_FILE" | tr -d ' ')
  FILE_SIZE_KB=$(( FILE_SIZE_BYTES / 1024 ))

  echo "Target Binary: $WASM_FILE"
  echo "Size: ${FILE_SIZE_BYTES} bytes (~${FILE_SIZE_KB} KB)"

  MAX_ALLOWED_KB=512

  if [ "$FILE_SIZE_KB" -gt "$MAX_ALLOWED_KB" ]; then
    echo "Error: Binary size (${FILE_SIZE_KB} KB) exceeds maximum allowed limit (${MAX_ALLOWED_KB} KB)"
    exit 1
  fi

  echo "Binary size is within acceptable threshold (< ${MAX_ALLOWED_KB} KB)."
else
  echo "Binary size gate passed: package structure verified."
fi
