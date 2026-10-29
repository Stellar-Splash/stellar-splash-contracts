#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Stellar Splash Master Contract Verification"
echo "=========================================="

echo "1. Checking code formatting..."
cargo fmt --check

echo "2. Running Clippy lints..."
cargo clippy --all-targets -- -D warnings

echo "3. Running test suite..."
cargo test

echo "4. Running formal invariants..."
bash scripts/run-invariants.sh

echo "5. Verifying WASM size limits..."
bash scripts/check-wasm-size.sh

echo "=========================================="
echo "✅ All verification checks passed cleanly!"
echo "=========================================="

