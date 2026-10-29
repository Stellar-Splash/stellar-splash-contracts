# Dispute Window Extension & Settlement Metrics Specification

## 1. Overview
Trustless eSports escrow requires an unalterable balance between swift prize disbursements and rigorous dispute investigations. The Stellar Splash contract enforces a time-locked dispute window immediately following tournament conclusion, while giving platform administrators the power to extend deadlines during forensic anti-cheat reviews.

This specification documents the on-chain dispute window mechanics, administrative extension entrypoints, and settlement query methods.

---

## 2. Dispute Window Governance & Constraints

### 2.1 Minimum Duration Enforced (`MIN_DISPUTE_WINDOW_SECONDS = 300`)
To prevent malicious tournament creators from opening and immediately executing settlements before participants can submit evidence, the contract strictly rejects any window duration less than 300 seconds (5 minutes):

```rust
if duration_seconds < MIN_DISPUTE_WINDOW_SECONDS {
    return Err(Error::TournamentDurationTooShort);
}
```

### 2.2 Time Calculation
The dispute window expiration timestamp is calculated using the canonical ledger timestamp:
$$\text{EndTimestamp} = \text{LedgerTimestamp} + \text{DurationSeconds}$$

---

## 3. Administrative Extension Entrypoint (`extend_dispute_window`)
When a complex dispute is filed (such as suspected bot collusion or packet replay), administrators can lengthen the dispute window to preserve escrow freeze while reviewing telemetry:

```rust
pub fn extend_dispute_window(
    env: Env,
    admin: Address,
    tournament_id: u64,
    extension_seconds: u64,
) -> Result<u64, Error>
```

### Invariants:
1. **Administrative Authorization**: Must satisfy `admin.require_auth()` matching `DataKey::Admin`.
2. **Safe Addition**: Uses checked arithmetic (`checked_add`) to prevent integer overflow.
3. **TTL Bumping**: Extends persistent storage lifetime with `STORAGE_LIFETIME_BUMP`.
4. **Event Emission**: Emits `(symbol_short!("dispute"), symbol_short!("extend")), (tournament_id, new_end)`.

---

## 4. Query Method Specifications

### `get_tournament_dispute_window(env: Env, tournament_id: u64) -> Option<DisputeWindowInfo>`
Returns structured information indicating deadline and whether the challenge period has elapsed:
```rust
#[contracttype]
pub struct DisputeWindowInfo {
    pub end_timestamp: u64,
    pub is_elapsed: bool,
}
```

### `get_settled_amount(env: Env, tournament_id: u64) -> i128`
Queries the exact disbursed prize pool for a given tournament. Returns `0` if the tournament is not yet settled or unfinalized.

### `get_tournament_settlement_status(env: Env, tournament_id: u64) -> Option<SettlementStatus>`
Returns the `SettlementStatus` enum variant (`Draft`, `Authorized`, `Executing`, `Settled`, `Failed`) or `None` if no settlement record has been created.

### `get_total_settled_amount(env: Env) -> i128`
Returns cumulative volume of prizes successfully disbursed across all settled tournaments.

### `get_total_volume(env: Env) -> i128`
Returns cumulative prize pool volume ever deposited into the contract vault.
