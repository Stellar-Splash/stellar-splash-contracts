# 🌊 Stellar Splash Contracts

Soroban smart contracts providing the financial prize vault, tournament lifecycle, verified result attestation, and locked prize agreement foundation for the Stellar Splash gaming platform on Stellar Testnet.

---

## Overview

Stellar Splash transforms skill-based gameplay into verifiable outcomes. Creators fund tournament prize pools in Soroban vaults, players compete in skill games, gameplay results are independently replayed and cryptographically attested, and locked prize agreements govern deterministic settlement.

`stellar-splash-contracts` implements:
1. **Tournament Vault**: Escrows prize pool deposits from tournament creators/sponsors.
2. **Result Verification Registry**: Authoritatively stores verified match result hashes and attestations on-chain.
3. **Tournament Ranking Registry**: Commits canonical ranking hashes for completed tournaments.
4. **Locked Prize Agreement Engine**: Enforces that prize distribution rules are explicitly registered, approved, and immutably locked before settlement can take place.

---

## Contract Architecture

```text
               +----------------------------------------------------+
               |              TournamentVaultContract               |
               +----------------------------------------------------+
                                         |
     +-------------------+---------------+-------------------+--------------------+
     |                   |                                   |                    |
     v                   v                                   v                    v
+-----------+    +----------------+                  +---------------+    +---------------+
| Lifecycle |    |  Prize Vault   |                  | Verifications |    |  Agreements   |
+-----------+    +----------------+                  +---------------+    +---------------+
| - create  |    | - fund_pool    |                  | - record_res  |    | - register_agr|
| - open    |    | - locked escrow|                  | - record_rank |    | - lock_agr    |
| - start   |    | - get_balance  |                  | - query_res   |    | - get_agr     |
+-----------+    +----------------+                  +---------------+    +---------------+
```

---

## State Model & Lifecycles

### Tournament Lifecycle
```text
Created -> Funding -> Funded -> Open -> InProgress -> Completed -> Cancelled
```

### Prize Agreement Lifecycle
```text
Draft -> PendingApproval -> ReadyToLock -> Locked -> Superseded
```

> **Immutability Invariant**: Once an agreement reaches `Locked`, its material financial terms cannot be altered. Any modification requires registering a new agreement version (e.g., `v2`) with a new cryptographic hash and obtaining fresh approvals.

---

## Public Interface

### Tournament & Vault
- `initialize(admin: Address)`: Configures contract admin address.
- `create_tournament(creator: Address, config: TournamentConfig) -> TournamentInfo`: Registers new tournament.
- `fund_prize_pool(funder: Address, tournament_id: u64, amount: i128) -> TournamentInfo`: Deposits tokens into the contract vault.
- `open_tournament(caller: Address, tournament_id: u64) -> TournamentInfo`: Opens tournament once funded.
- `register_participant(player: Address, tournament_id: u64)`: Enrolls player within capacity limits.
- `start_tournament(caller: Address, tournament_id: u64) -> TournamentInfo`: Moves tournament to `InProgress`.
- `complete_tournament(caller: Address, tournament_id: u64) -> TournamentInfo`: Concludes tournament.
- `get_vault_balance(tournament_id: u64) -> i128`: Returns total locked prize balance.

### Result Verification & Attestation
- `record_verified_result(verifier: Address, input: VerifiedResultInput) -> VerifiedResultRecord`: Authoritatively commits verified match scores and SHA-256 result digests on-chain. Rejects duplicate submissions.
- `get_verified_result(tournament_id: u64, match_id: String) -> VerifiedResultRecord`: Queries on-chain verification record.
- `record_ranking_hash(caller: Address, tournament_id: u64, ranking_version: u32, ranking_hash: BytesN<32>) -> RankingRecord`: Finalizes the canonical ranking hash for a tournament.
- `get_ranking_record(tournament_id: u64) -> RankingRecord`: Retrieves the finalized ranking hash.

### Prize Agreements
- `register_prize_agreement(creator: Address, tournament_id: u64, version: u32, agreement_hash: BytesN<32>) -> PrizeAgreementRecord`: Registers a deterministic prize rule set.
- `lock_prize_agreement(creator: Address, tournament_id: u64, version: u32) -> PrizeAgreementRecord`: Authoritatively locks the agreement terms in escrow.
- `get_prize_agreement(tournament_id: u64, version: u32) -> PrizeAgreementRecord`: Queries specific agreement version.
- `is_agreement_locked(tournament_id: u64, version: u32) -> bool`: Checks if agreement is locked.

---

## Contract Events

The contract emits structured events indexed by `stellar-splash-engine`:
- `("tourn", "created")` -> `(tournament_id, creator, prize_asset, target_prize_amount)`
- `("prize", "funded")` -> `(tournament_id, funder, deposit_amount, total_funded)`
- `("tourn", "opened")` -> `tournament_id`
- `("part", "joined")` -> `(tournament_id, player)`
- `("result", "verified")` -> `(tournament_id, match_id, player, score)`
- `("ranking", "done")` -> `(tournament_id, ranking_version, ranking_hash)`
- `("agree", "created")` -> `(tournament_id, version, agreement_hash)`
- `("agree", "locked")` -> `(tournament_id, version, agreement_hash)`

---

## Verification & Testing

Run unit tests covering lifecycles, capacity, unauthorized operations, replay attempts, and agreement immutability:

```bash
cargo test --lib
```

Static analysis with zero warnings:

```bash
cargo clippy --lib --tests -- -D warnings
```

Build the release WASM bytecode:

```bash
cargo build --target wasm32-unknown-unknown --release
```

WASM Artifact:
`target/wasm32-unknown-unknown/release/stellar_splash_contracts.wasm`
