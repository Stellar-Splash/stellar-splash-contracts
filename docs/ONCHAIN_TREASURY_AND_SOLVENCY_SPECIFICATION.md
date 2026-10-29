# On-Chain Treasury, Solvency & Capacity Specification

## 1. Executive Summary
The Stellar Splash Soroban smart contract (`TournamentVaultContract`) functions as a non-custodial, deterministic escrow vault on the Stellar network. Its primary financial responsibilities are:
1. Guaranteeing prize pool solvency for all open tournaments.
2. Collecting protocol treasury fees within capped mathematical constraints.
3. Enforcing participant capacity limits to eliminate registration race conditions.
4. Blocking payout execution whenever an active dispute challenge is pending.

This document details the on-chain data types, query methods, fee mathematics, and administrative safeguards implemented in `src/lib.rs`.

---

## 2. On-Chain Data Structures & Types

### 2.1. Tournament Capacity Information
```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TournamentCapacityInfo {
    pub current_participants: u32,
    pub max_participants: u32,
    pub is_full: bool,
}
```
- **Entrypoint**: `get_tournament_capacity_status(env: Env, tournament_id: u64) -> Result<TournamentCapacityInfo, Error>`
- **Logic**: Returns current count, maximum limit, and a boolean flag (`current_participants >= max_participants`). Prevents over-subscription and enables instant UI capacity rendering without scanning participant arrays.

### 2.2. Protocol Treasury Configuration
- **Storage Keys**:
  - `DataKey::TreasuryAddress`: Destination account for protocol fee distributions.
  - `DataKey::TreasuryFeeBps`: Fee rate in basis points (1 bps = 0.01%).
- **Safety Cap**:
  ```rust
  pub const MAX_TREASURY_FEE_BPS: u32 = 1_000; // 10.00% maximum fee
  ```
  Attempting to configure a treasury fee greater than `MAX_TREASURY_FEE_BPS` returns `Error::InvalidTreasuryFeeBps`.

---

## 3. Treasury Fee Calculation Logic
The contract exposes a dedicated query method to calculate the exact protocol fee for any tournament:

```rust
pub fn get_tournament_treasury_fee(env: Env, tournament_id: u64) -> Result<i128, Error>
```

### Formula:
$$\text{Treasury Fee} = \frac{\text{Current Funded Amount} \times \text{Treasury Fee Bps}}{10,000}$$

- Returns `0` if no treasury fee is set or if funded amount is zero.
- Uses integer arithmetic to avoid rounding discrepancies.
- Verified in `test_dispute_active_capacity_and_treasury_queries`.

---

## 4. Dispute Activity & Settlement Holds

### 4.1. `is_dispute_active`
```rust
pub fn is_dispute_active(env: Env, tournament_id: u64) -> bool
```
- Checks whether a formal dispute challenge exists for `tournament_id` and has status `DisputeStatus::Active`.
- While active, settlement execution (`execute_settlement`) is strictly rejected with `Error::TournamentDisputed`.
- Only an administrator can clear an active dispute by invoking `resolve_dispute(env, admin, tournament_id)`.

---

## 5. Contract Deployer & Admin Discovery
To facilitate multi-indexer synchronization and verifiable telemetry across web and mobile clients:
- `get_admin(env: Env) -> Result<Address, Error>`: Returns root admin address.
- `get_contract_deployer(env: Env) -> Result<Address, Error>`: Canonical discovery method exposing the deployer/admin authority.
- Both methods bump instance storage TTL (`STORAGE_BUMP_THRESHOLD`, `STORAGE_LIFETIME_BUMP`) to prevent state archiving.
