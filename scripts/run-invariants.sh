#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Soroban Contract Formal Invariant Harness"
echo "=========================================="

echo "Running invariant test suite..."
cargo test test_invariants -- --nocapture

echo "✅ All mathematical and state invariants held."
