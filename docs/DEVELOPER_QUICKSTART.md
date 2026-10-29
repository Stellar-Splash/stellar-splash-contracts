# Stellar Splash Contracts - Developer Quickstart Guide

This guide walks developers through setting up the local Rust/Soroban toolchain, compiling the `TournamentVaultContract`, and running the unit and formal invariant test suites.

---

## 🛠️ Prerequisites
1. **Rust**: Stable toolchain $\ge 1.79.0$.
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
2. **Stellar CLI**:
   ```bash
   cargo install --locked stellar-cli --features opt
   ```

---

## 🧪 Building & Testing

### 1. Run All Unit & Invariant Tests
```bash
cargo test
```
Executes all 9 unit tests verifying tournament creation, deposit limits, basis points payout math, emergency circuit breaking, and TTL extensions.

### 2. Run Invariant Verifier Script
```bash
bash scripts/run-invariants.sh
```

### 3. Build WASM Bytecode
```bash
cargo build --target wasm32-unknown-unknown --release
```

### 4. Optimize Bytecode
```bash
stellar contract optimize --wasm target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm
```

### 5. Profile WASM Size
```bash
bash scripts/profile-wasm-size.sh
```
Ensures WASM bytecode stays well within the Soroban network limit ($< 64\text{ KB}$).

