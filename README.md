# 🌊 Stellar Splash Contracts

Soroban smart contracts providing the financial prize vault and tournament lifecycle foundation for the Stellar Splash gaming platform on Stellar Testnet.

---

## Overview

Stellar Splash is a skill-based gaming platform where creators fund tournament prize pools on Stellar, players compete in fast-paced games, gameplay results are deterministically recorded, and verified outcomes become programmable Stellar settlements.

`stellar-splash-contracts` implements the foundational **Tournament Vault** on Soroban. The vault securely locks prize pool funds deposited by tournament creators/sponsors, tracks tournament state transitions, verifies participant eligibility, and guarantees that funds cannot be arbitrarily drained before settlement.

---

## Contract Architecture

```text
               +--------------------------------------+
               |       TournamentVaultContract        |
               +--------------------------------------+
                                  |
         +------------------------+------------------------+
         |                                                 |
         v                                                 v
+------------------+                              +------------------+
| Lifecycle Engine |                              |   Prize Vault    |
+------------------+                              +------------------+
| - create         |                              | - fund_prize_pool|
| - open           |                              | - get_balance    |
| - register_player|                              | - locked custody |
| - start / complete                              +------------------+
+------------------+
```

### Core Responsibilities
1. **Tournament Registry**: Records tournament configurations including title, game ID, prize asset, target prize amount, capacity, and timestamps.
2. **Locked Prize Vault**: Enforces that prize pool deposits are held in contract escrow. Funds are locked upon deposit and cannot be unilaterally withdrawn by creators.
3. **Explicit State Machine**: Manages state transitions strictly:
   ```text
   CREATED -> FUNDING -> FUNDED -> OPEN -> IN_PROGRESS -> COMPLETED
   ```
4. **Capacity & Participant Validation**: Prevents oversubscription and duplicate participant registrations.
5. **Event Emission**: Publishes structured Soroban events consumed by the backend indexing engine (`stellar-splash-engine`).

---

## State Model

| State | Description |
| :--- | :--- |
| `Created` | Tournament record initialized. |
| `Funding` | Tournament is accepting prize pool deposits from creator/sponsor. |
| `Funded` | Target prize pool has reached 100% of required funds in vault. |
| `Open` | Tournament is open for eligible player registration. |
| `InProgress` | Matches are underway. |
| `Completed` | Tournament concluded, awaiting final verification and settlement. |
| `Cancelled` | Tournament cancelled prior to launch. |

---

## Public Interface

### Initialization & Administration
- `initialize(admin: Address)`: Configures contract admin address (idempotent, single-use).
- `get_admin() -> Address`: Returns the administrative authority.

### Tournament Management
- `create_tournament(creator: Address, config: TournamentConfig) -> TournamentInfo`:
  Registers a new tournament. Requires `creator.require_auth()`.
- `fund_prize_pool(funder: Address, tournament_id: u64, amount: i128) -> TournamentInfo`:
  Transfers `amount` of `prize_asset` from funder to the contract vault. Transitions tournament to `Funded` once target is reached.
- `open_tournament(caller: Address, tournament_id: u64) -> TournamentInfo`:
  Opens registration. Requires creator authorization and `Funded` state.
- `register_participant(player: Address, tournament_id: u64)`:
  Registers a player into an open tournament. Rejects duplicate entries or attempts exceeding capacity.
- `start_tournament(caller: Address, tournament_id: u64) -> TournamentInfo`:
  Transitions tournament to `InProgress`.
- `complete_tournament(caller: Address, tournament_id: u64) -> TournamentInfo`:
  Transitions tournament to `Completed`.
- `get_tournament(tournament_id: u64) -> TournamentInfo`:
  Queries tournament record.
- `is_participant(tournament_id: u64, player: Address) -> bool`:
  Checks registration status of a given wallet.
- `get_vault_balance(tournament_id: u64) -> i128`:
  Returns total locked prize funds for the tournament.

---

## Contract Events

The contract emits structured events indexed by the `stellar-splash-engine`:

- `("tourn", "created")` -> `(tournament_id, creator, prize_asset, target_prize_amount)`
- `("prize", "funded")` -> `(tournament_id, funder, deposit_amount, total_funded)`
- `("tourn", "opened")` -> `tournament_id`
- `("part", "joined")` -> `(tournament_id, player)`
- `("tourn", "started")` -> `tournament_id`
- `("tourn", "done")` -> `tournament_id`

---

## Invariants & Security

- **Strict Authorization**: Every state change requires corresponding cryptographic signature (`require_auth`).
- **No Unfunded Opens**: Tournaments cannot be opened for players until 100% of the prize pool is funded and confirmed in the vault.
- **Custody Boundary**: The creator does not have arbitrary withdrawal privileges after funding.
- **Arithmetic Safety**: All balances use checked integer arithmetic (`checked_add`, `checked_sub`) to prevent overflows.
- **Double-Registration Prevention**: Player registration sets a unique storage key per `(tournament_id, player)` preventing replay or double-entry.

---

## Testing

Run unit tests covering the full lifecycle, edge cases, capacity constraints, and invalid parameters:

```bash
cargo test --lib
```

Run static analysis with zero warnings:

```bash
cargo clippy --lib --tests -- -D warnings
```

---

## Building WASM

Build the optimized WebAssembly binary target:

```bash
cargo build --target wasm32-unknown-unknown --release
```

Compiled artifact location:
`target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm`

---

## Testnet Deployment

To deploy to Stellar Testnet using the Stellar CLI:

```bash
# 1. Generate identity if needed
stellar keys generate alice --network testnet

# 2. Fund identity via Friendbot
stellar keys fund alice --network testnet

# 3. Deploy contract WASM
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm \
  --source alice \
  --network testnet
```

### Reference Addresses (Stellar Testnet)
- **Network Passphrase**: `Test SDF Network ; September 2015`
- **Horizon URL**: `https://horizon-testnet.stellar.org`
- **RPC URL**: `https://soroban-testnet.stellar.org`
- **Native Asset**: `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` (Native XLM Wrapper) / Native Lumens
