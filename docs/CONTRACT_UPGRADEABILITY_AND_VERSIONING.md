# Soroban Contract Upgradeability & Protocol Versioning Specification

## 1. Overview
The Stellar Splash tournament vault smart contract is designed for long-term production resilience on Stellar Soroban. It leverages native Soroban WASM upgradeability while strictly protecting persistent state, prize escrows, and user reputation data against accidental loss or storage schema corruption.

This document describes the technical upgrade mechanism, protocol versioning guarantees, and state preservation architecture.

---

## 2. In-Place WASM Upgrade Mechanism
Soroban provides native in-place contract bytecode replacement through the environment deployer interface:

```rust
pub fn upgrade_contract(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
    let admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)?;
    admin.require_auth();

    env.deployer().update_current_contract_wasm(new_wasm_hash.clone());

    env.events().publish(
        (symbol_short!("upgrade"), symbol_short!("wasm")),
        new_wasm_hash,
    );
    Ok(())
}
```

### Key Security Invariants:
1. **Administrative Authentication**: Must satisfy `admin.require_auth()` with the cryptographic key stored in `DataKey::Admin`.
2. **Atomic Replacement**: Contract code is updated within the current ledger transaction without changing contract address or clearing ledger storage.
3. **Audit Trail**: An on-chain Soroban event `(upgrade, wasm)` is emitted containing the SHA-256 hash of the newly deployed bytecode.

---

## 3. Protocol Versioning & Compatibility Queries
To prevent frontends and backend engines from interacting with outdated or incompatible contracts, two canonical query methods are provided:

### `get_contract_version`
Returns the active Stellar Protocol release level:
```rust
pub fn get_contract_version(_env: Env) -> u32 {
    21 // Soroban Protocol 21
}
```

### `get_contract_metadata`
Returns immutable descriptor metadata:
```rust
pub fn get_contract_metadata(env: Env) -> ContractMetadata {
    ContractMetadata {
        name: String::from_str(&env, "Stellar Splash Tournament Vault"),
        version: String::from_str(&env, "1.0.0"),
        description: String::from_str(&env, "Deterministic eSports escrow vault and settlement protocol on Soroban"),
        protocol_version: 1,
        min_prize_amount: MIN_PRIZE_AMOUNT,
    }
}
```

---

## 4. Multi-Version Prize Agreement Evolution
Tournament prize distributions support multi-version iterations prior to finalization:
1. **Version Increments**: Creators can register successive versions (`version: u32 = 1, 2, ...`).
2. **Active Version Pointer**: `DataKey::ActiveAgreementVersion(tournament_id)` tracks the governing prize distribution schema.
3. **Immutability Locking**: Once locked via `lock_prize_agreement()`, the agreement version is permanently sealed and cannot be modified.

---

## 5. Pre-Upgrade Safety & Verification Checklist
Before submitting a WASM upgrade to Stellar Mainnet/Testnet:
- [ ] New WASM binary is compiled with `--release --target wasm32-unknown-unknown`.
- [ ] Bytecode is verified with `stellar contract optimize`.
- [ ] Invariant tests in `scripts/test-reentrancy.cjs` pass with zero failures.
- [ ] Contract benchmark instructions are within Soroban CPU/RAM resource limits.
- [ ] Storage keys in `DataKey` enum remain backward-compatible with previously written instance and persistent keys.
- [ ] `ContractUpgradeTrackerService` in the engine confirms compatibility with the new WASM hash.
