# 📜 Soroban Smart Contract Governance, Upgradeability & Dispute Specification

---

## 1. Overview & Security Philosophy

The **Stellar Splash Tournament Vault Contract** (`TournamentVaultContract`) deployed on Stellar Protocol 21+ (`CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`) manages escrowed prize pools, multi-recipient payouts, and on-chain verified rankings. To combine security guarantees with operational adaptability, the contract implements a deterministic governance framework:

1. **Explicit Admin Role Separation**: Only the verified administrative address (`get_admin`) can authorize contract upgrades, whitelist prize assets, or configure treasury parameters.
2. **Deterministic Self-Upgrade Entrypoint**: Built on Soroban's native `env.deployer().update_current_contract_wasm(new_wasm_hash)` without proxy indirection or storage collision risks.
3. **Emergency Circuit Breaker**: Instant pause and unpause switches protecting participant balances during detected network anomalies or protocol migrations.
4. **Time-Locked Dispute Window**: Mandates a delay before settlement execution, allowing contenders or automated anti-cheat watchdogs to challenge suspicious match rankings.

---

## 2. Contract Upgrade Lifecycle

```text
[ Developer / Auditor ]
          │
          ▼
1. Compile Rust to WASM (wasm32-unknown-unknown)
          │
          ▼
2. Calculate SHA-256 Hash of new WASM Bytecode:
   stellar contract install --wasm target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm
   ==> Yields new_wasm_hash (BytesN<32>)
          │
          ▼
3. Multi-Sig / Admin executes `upgrade(admin, new_wasm_hash)`
          │
          ├─► Enforces admin.require_auth()
          ├─► Emits event: (symbol_short!("contract"), symbol_short!("upgrade"))
          ├─► Calls env.deployer().update_current_contract_wasm(new_wasm_hash)
          └─► Extends instance storage TTL
          │
          ▼
[ New Contract Code Active Immediately - All Storage Persisted ]
```

### Storage Preservation Guarantees

In Soroban, contract upgrades via `update_current_contract_wasm` **replace only the executable WASM bytecode** while preserving all persistent and instance storage entries:
- Existing tournament registries (`DataKey::Tournament(u64)`)
- Locked prize agreements (`DataKey::PrizeAgreement(u64, u32)`)
- Historical settlements and volume counters (`DataKey::TotalVolume`, `DataKey::TotalSettledAmount`)
- Vault token allowances and balances

---

## 3. Dispute Resolution & Fair Play Safeguards

### 3.1 Dispute Window Architecture

When a tournament concludes, organizers or the protocol open a dispute window (`open_dispute_window`) with a duration $\ge 300\text{ seconds}$ (5 minutes):

$$\text{dispute\_deadline} = \text{ledger\_timestamp} + \text{duration\_seconds}$$

During this window:
1. Contenders can review full match replay inputs and cryptographic hashes on the frontend Dispute Modal.
2. If fraud or bot activity is identified, players or engine watchdogs invoke `submit_dispute(challenger, tournament_id, reason_code)`.
3. The tournament is flagged with `TournamentDisputedFlag(tournament_id) = true`.
4. Settlement execution via `execute_settlement` is **strictly blocked** for all non-administrative callers.

### 3.2 Dispute Reason Codes

| Reason Code | Name | Description |
| :--- | :--- | :--- |
| `101` | `FRAUD_AUTOCLICKER` | Statistical entropy anomaly detected (CPS > 18 or zero interval variance) |
| `102` | `FRAME_DESYNC` | Replay event log desynchronizes from deterministic PRNG state |
| `103` | `TAMPERED_SCORE` | Score in verified result does not match reconstructed replay physics |
| `104` | `UNAUTHORIZED_ENTRY` | Player failed entry fee or KYC eligibility requirements |

### 3.3 Dispute Resolution

Only the administrative multisig can resolve an active dispute via `resolve_dispute(admin, tournament_id)` after reviewing on-chain replay evidence or refunding contested contenders.

---

## 4. Emergency Circuit Breaker (Pause / Unpause)

In the event of an upstream Soroban network upgrade or unexpected ledger condition:
1. `pause(admin)` locks all tournament creation, participant registration, prize pool funding, and settlement execution.
2. Read-only view methods (`get_tournament`, `get_total_volume`, `get_vault_balance`) remain fully accessible to inspect state.
3. `unpause(admin)` restores normal operations once the network state is verified.

---

## 5. Storage Rent & TTL Management Policy

Every persistent storage write utilizes protocol-mandated TTL extensions:
- **Threshold**: 17,280 ledgers (~1 day)
- **Lifetime Bump**: 518,400 ledgers (~30 days)

```rust
env.storage().persistent().extend_ttl(&key, STORAGE_BUMP_THRESHOLD, STORAGE_LIFETIME_BUMP);
```

This ensures zero archived state errors across long-running seasonal leagues and historical payout ledgers.
