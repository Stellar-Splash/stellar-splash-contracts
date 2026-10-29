# Stellar Splash - Soroban Contract Upgrade & Hotfix Runbook

## 1. Upgrade Architecture
Soroban smart contracts support code updates via the `env.deployer().update_current_contract_wasm(new_wasm_hash)` primitive. In Stellar Splash, this capability is governed by a two-step multi-sig or timelocked admin procedure.

## 2. Step-by-Step Upgrade Procedure

### 2.1 Step 1: Compile & Optimize New Bytecode
```bash
cargo build --target wasm32-unknown-unknown --release
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm
```

### 2.2 Step 2: Upload New WASM to Network
```bash
stellar contract install \
  --source-account admin_key \
  --network testnet \
  --wasm target/wasm32-unknown-unknown/release/stellar_splash_contracts.optimized.wasm
```
*Output: New WASM hash (e.g. `a1b2c3d4...`)*

### 2.3 Step 3: Propose Contract Upgrade
Invoke the 2-step upgrade proposal function:
```bash
stellar contract invoke \
  --id CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ \
  --source-account admin_key \
  --network testnet \
  -- propose_upgrade \
  --new_wasm_hash <NEW_WASM_HASH>
```

### 2.4 Step 4: Verify Proposal & Invariants
Verify the proposed WASM hash through read-only inspection:
```bash
stellar contract invoke \
  --id CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ \
  --source-account admin_key \
  --network testnet \
  -- get_proposed_upgrade
```

### 2.5 Step 5: Execute Upgrade
```bash
stellar contract invoke \
  --id CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ \
  --source-account admin_key \
  --network testnet \
  -- apply_upgrade
```

## 3. Rollback & Emergency Contingency
If unexpected invariant violations occur post-upgrade:
1. Immediately trigger `client.emergency_pause()`.
2. Inspect pending tournament states.
3. If necessary, execute code rollback to previous verified WASM hash via authorized hotfix key.
4. Programmatically process tournament refunds for active matches.

