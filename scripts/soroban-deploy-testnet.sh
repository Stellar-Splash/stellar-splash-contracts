#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "Stellar Splash Soroban Testnet Deployer"
echo "=========================================="

NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"
NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
ADMIN_ALIAS="stellar-splash-admin"

echo "1. Building optimized contract WASM..."
cargo build --target wasm32-unknown-unknown --release

WASM_PATH="target/wasm32-unknown-unknown/release/stellar_splash.wasm"

echo "2. Optimizing WASM binary..."
if command -v stellar &> /dev/null; then
  stellar contract optimize --wasm "$WASM_PATH"
fi

echo "3. Deploying to Stellar $NETWORK..."
if command -v stellar &> /dev/null; then
  CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM_PATH" \
    --source "$ADMIN_ALIAS" \
    --network "$NETWORK")

  echo "✅ Contract deployed successfully!"
  echo "Contract ID: $CONTRACT_ID"
else
  echo "Stellar CLI not detected; build completed for manual deployment."
fi

