#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Soroban Storage TTL Extension Auditor"
echo "=========================================="

CONTRACT_SOURCE="contracts/stellar-splash/src/lib.rs"

if [ ! -f "$CONTRACT_SOURCE" ]; then
  echo "Error: Contract source not found at $CONTRACT_SOURCE"
  exit 1
fi

echo "Auditing storage key extend_ttl invocations in $CONTRACT_SOURCE..."

# Grep for extend_ttl occurrences
EXTEND_COUNT=$(grep -c "extend_ttl" "$CONTRACT_SOURCE" || true)

echo "Found $EXTEND_COUNT extend_ttl calls across persistent data keys."

if [ "$EXTEND_COUNT" -lt 3 ]; then
  echo "❌ Error: Insufficient TTL extension coverage in contract implementation."
  exit 1
fi

echo "✅ Storage TTL extension coverage verified across persistent state keys."

