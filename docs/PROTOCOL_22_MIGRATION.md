# Stellar Splash Contracts - Soroban Protocol 21/22 Migration Guide

## 1. Context & Objectives
Stellar Protocol 21/22 introduces enhanced state archiving, optimized cryptographic host functions, and predictable gas metering. The `TournamentVaultContract` (`CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`) is engineered to transition natively across protocol versions.

---

## 2. Key Architecture Features

### 2.1 State Rent & Storage Invariants
- Persistent entries automatically call `env.storage().persistent().bump(&key, low_threshold, high_threshold)` where `high_threshold = 3,110,400` ledgers (~6 months).
- Instance storage keeps global configuration alive for the lifetime of the contract address.

### 2.2 Host Cryptographic Verification
- Replay attestation hashes and Merkle root verification use native Soroban `env.crypto().keccak256(&payload)` and `sha256(&payload)` primitives, minimizing CPU instruction gas consumption.

### 2.3 Backward & Forward Compatibility
- `get_contract_version()` returns `21` on Protocol 21 testnet, and cleanly satisfies Protocol 22 testnet and future mainnet environments without bytecode rebuilds.
