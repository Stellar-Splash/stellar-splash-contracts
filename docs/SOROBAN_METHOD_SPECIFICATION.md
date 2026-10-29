# Soroban Smart Contract Entrypoint & Interface Specification

**Contract**: `TournamentVaultContract`  
**Network Deployment**: Stellar Testnet & Mainnet  
**Target Architecture**: `wasm32-unknown-unknown`  
**Soroban SDK Version**: `22.0.0`  

---

## 1. Overview

`TournamentVaultContract` is the authoritative smart contract for Stellar Splash. It manages tournament registrations, non-custodial prize vaults, cryptographic attestation commits, ranking locks, and trustless multi-recipient disbursements.

---

## 2. Public Method Signatures

### 2.1 Administration & Protocol Configuration

| Method | Parameters | Return Type | Auth Required | Description |
| :--- | :--- | :--- | :--- | :--- |
| `initialize` | `env: Env`, `admin: Address` | `Result<(), Error>` | `admin` | Initializes the vault, setting admin address and zeroing counters. |
| `get_admin` | `env: Env` | `Result<Address, Error>` | None | Returns the current administrative address. |
| `get_contract_metadata` | `env: Env` | `ContractMetadata` | None | Returns protocol metadata (version, min prize amount, name). |
| `pause` | `env: Env` | `Result<(), Error>` | `admin` | Activates emergency pause, blocking new tournaments. |
| `unpause` | `env: Env` | `Result<(), Error>` | `admin` | Lifts emergency pause. |
| `is_paused` | `env: Env` | `bool` | None | Returns whether emergency pause is currently active. |
| `set_treasury` | `env: Env`, `treasury: Address`, `fee_bps: u32` | `Result<(), Error>` | `admin` | Configures protocol treasury and basis-point fee (≤ 1000 bps). |
| `get_treasury_address`| `env: Env` | `Option<Address>` | None | Retrieves configured treasury payout address. |
| `get_treasury_fee_bps`| `env: Env` | `u32` | None | Retrieves configured treasury fee in basis points. |
| `get_total_settled_amount`| `env: Env` | `i128` | None | Retrieves cumulative prize stroops settled across all tournaments. |

---

### 2.2 Tournament Lifecycle Management

| Method | Parameters | Return Type | Auth Required | Description |
| :--- | :--- | :--- | :--- | :--- |
| `create_tournament` | `env: Env`, `creator: Address`, `config: TournamentConfig` | `Result<TournamentInfo, Error>` | `creator` | Creates tournament record in `Funding` state. |
| `get_tournament` | `env: Env`, `tournament_id: u64` | `Result<TournamentInfo, Error>` | None | Queries full tournament metadata record. |
| `get_tournament_details` | `env: Env`, `tournament_id: u64` | `Result<TournamentDetails, Error>` | None | Compact view struct for frontend widgets. |
| `get_tournament_count`| `env: Env` | `u64` | None | Returns total number of tournaments created. |
| `fund_prize_pool` | `env: Env`, `funder: Address`, `tournament_id: u64`, `amount: i128` | `Result<TournamentInfo, Error>` | `funder` | Transfers tokens into vault and marks tournament `Funded`. |
| `fund_prize_pool_from`| `env: Env`, `spender: Address`, `funder: Address`, `tournament_id: u64`, `amount: i128` | `Result<TournamentInfo, Error>` | `spender` | Funds tournament via SEP-41 token allowance. |
| `open_tournament` | `env: Env`, `caller: Address`, `tournament_id: u64` | `Result<TournamentInfo, Error>` | `caller` | Transitions funded tournament to `Open` for player registration. |
| `register_participant`| `env: Env`, `participant: Address`, `tournament_id: u64` | `Result<TournamentInfo, Error>` | `participant`| Registers player wallet into tournament participant list. |
| `start_tournament` | `env: Env`, `caller: Address`, `tournament_id: u64` | `Result<TournamentInfo, Error>` | `caller` | Transitions tournament to `InProgress`. |
| `complete_tournament`| `env: Env`, `caller: Address`, `tournament_id: u64` | `Result<TournamentInfo, Error>` | `caller` | Verifies end time / capacity and marks tournament `Completed`. |
| `cancel_tournament` | `env: Env`, `caller: Address`, `tournament_id: u64` | `Result<TournamentInfo, Error>` | `caller` | Cancels unfunded or unstarted tournament. |
| `claim_refund` | `env: Env`, `caller: Address`, `tournament_id: u64` | `Result<i128, Error>` | `caller` | Returns 100% of deposited prize pool back to funder. |

---

### 2.3 Verification & Ranking Finality

| Method | Parameters | Return Type | Auth Required | Description |
| :--- | :--- | :--- | :--- | :--- |
| `record_verified_result`| `env: Env`, `verifier: Address`, `input: VerifiedResultInput` | `Result<VerifiedResultRecord, Error>` | `verifier` | Commits SHA-256 match result hash. |
| `get_verified_result` | `env: Env`, `tournament_id: u64`, `match_id: String` | `Result<VerifiedResultRecord, Error>` | None | Queries verified match result on-chain. |
| `record_ranking_hash` | `env: Env`, `authorizer: Address`, `tournament_id: u64`, `ranking_version: u32`, `ranking_hash: BytesN<32>` | `Result<RankingRecord, Error>` | `authorizer` | Commits finalized tournament leaderboard hash. |
| `get_ranking` | `env: Env`, `tournament_id: u64` | `Result<RankingRecord, Error>` | None | Queries finalized ranking record. |

---

### 2.4 Prize Agreement & Multi-Recipient Settlement

| Method | Parameters | Return Type | Auth Required | Description |
| :--- | :--- | :--- | :--- | :--- |
| `register_prize_agreement`| `env: Env`, `creator: Address`, `tournament_id: u64`, `version: u32`, `agreement_hash: BytesN<32>` | `Result<PrizeAgreementRecord, Error>` | `creator` | Registers prize agreement proposal hash. |
| `lock_prize_agreement` | `env: Env`, `creator: Address`, `tournament_id: u64`, `version: u32` | `Result<PrizeAgreementRecord, Error>` | `creator` | Formally locks agreement, making it immutable. |
| `get_prize_agreement` | `env: Env`, `tournament_id: u64`, `version: u32` | `Result<PrizeAgreementRecord, Error>` | None | Queries prize agreement by version. |
| `authorize_settlement` | `env: Env`, `caller: Address`, `input: SettlementInput` | `Result<SettlementRecord, Error>` | `caller` | Validates hash consistency and stores recipient allocations. |
| `execute_settlement` | `env: Env`, `caller: Address`, `tournament_id: u64` | `Result<SettlementRecord, Error>` | `caller` | Atomically executes payouts and increments `TotalSettledAmount`. |
| `get_settlement` | `env: Env`, `tournament_id: u64` | `Result<SettlementRecord, Error>` | None | Queries settlement metadata. |
| `get_settlement_allocations` | `env: Env`, `tournament_id: u64` | `Result<Vec<RecipientAllocation>, Error>` | None | Queries recipient payout list. |
| `is_settled` | `env: Env`, `tournament_id: u64` | `bool` | None | Returns true if tournament has been settled. |

---

### 2.5 Storage Rent Maintenance (TTL)

| Method | Parameters | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `extend_contract_instance_ttl` | `env: Env` | `()` | Bumps instance storage TTL by 30 days. |
| `extend_tournament_ttl` | `env: Env`, `tournament_id: u64` | `Result<(), Error>` | Bumps persistent tournament record storage TTL by 30 days. |

---

## 3. Data Structures

```rust
pub struct TournamentConfig {
    pub tournament_id: u64,
    pub title: String,
    pub game_id: String,
    pub prize_asset: Address,
    pub target_prize_amount: i128,
    pub max_participants: u32,
    pub start_time: u64,
    pub end_time: u64,
}

pub struct TournamentDetails {
    pub tournament_id: u64,
    pub state: TournamentState,
    pub target_prize_amount: i128,
    pub current_funded_amount: i128,
    pub current_participants: u32,
    pub max_participants: u32,
    pub start_time: u64,
    pub end_time: u64,
}

pub struct RecipientAllocation {
    pub recipient: Address,
    pub rank: u32,
    pub amount: i128,
}
```

---

## 4. Contract Error Codes

| Code | Identifier | Description |
| :--- | :--- | :--- |
| `1` | `AlreadyInitialized` | Contract instance is already initialized. |
| `2` | `NotInitialized` | Contract instance has not been initialized. |
| `3` | `Unauthorized` | Caller lacks authorization (admin or creator required). |
| `4` | `TournamentNotFound` | Tournament ID not found in persistent storage. |
| `5` | `TournamentAlreadyExists` | Tournament ID already exists in storage. |
| `6` | `InvalidTournamentState` | Operation not allowed in current state. |
| `7` | `InvalidAmount` | Prize amount below `MIN_PRIZE_AMOUNT` (10,000,000 Stroops). |
| `8` | `InvalidTimeBounds` | `end_time` is less than or equal to `start_time`. |
| `9` | `TournamentNotFunded` | Tournament has not completed funding phase. |
| `10` | `TournamentAlreadyFunded`| Prize pool is already funded. |
| `11` | `TournamentFull` | Participant limit (`max_participants`) reached. |
| `12` | `ParticipantAlreadyRegistered` | Player is already registered in this tournament. |
| `13` | `TournamentNotOpen` | Tournament is not in `Open` registration state. |
| `14` | `ArithmeticOverflow` | Checked math overflow or underflow occurred. |
| `15` | `ResultAlreadyRecorded` | Match ID already has a recorded verification hash. |
| `16` | `ResultNotFound` | Verified result record not found. |
| `17` | `AgreementAlreadyExists`| Prize agreement version already registered. |
| `18` | `AgreementNotFound` | Prize agreement version not found. |
| `19` | `AgreementAlreadyLocked`| Prize agreement is already locked and immutable. |
| `20` | `AgreementNotLocked` | Action requires locked agreement status. |
| `21` | `RankingAlreadyFinalized`| Ranking for tournament has already been committed. |
| `22` | `RankingNotFound` | Ranking record not found. |
| `23` | `SettlementAlreadyExists`| Settlement has already been authorized. |
| `24` | `SettlementNotFound` | Settlement record not found. |
| `25` | `SettlementAlreadyExecuted` | Payouts have already been transferred from vault. |
| `26` | `AgreementHashMismatch`| Settlement agreement hash does not match locked agreement. |
| `27` | `RankingHashMismatch` | Settlement ranking hash does not match committed ranking. |
| `28` | `AllocationExceedsPool` | Sum of recipient payouts exceeds vault balance. |
| `29` | `EmptyAllocations` | Payout recipient list cannot be empty. |
| `30` | `ContractPaused` | Operation blocked by emergency circuit breaker. |
| `31` | `TournamentNotEnded` | Tournament has not reached scheduled end time. |
| `32` | `TitleTooLong` | Tournament title exceeds `MAX_STRING_LENGTH` (128 bytes). |
| `33` | `GameIdTooLong` | Tournament game_id exceeds `MAX_STRING_LENGTH` (128 bytes). |
| `34` | `InvalidTreasuryFeeBps` | Treasury fee exceeds `MAX_TREASURY_FEE_BPS` (1,000 bps). |
| `35` | `TournamentDurationTooShort` | Duration is less than `MIN_TOURNAMENT_DURATION` (60s). |

---

## 5. Emitted Events

1. **Tournament Funding**: `(symbol_short!("fund"), symbol_short!("prize"))` $\to$ `(tournament_id, funder, amount)`
2. **Participant Registration**: `(symbol_short!("reg"), symbol_short!("player"))` $\to$ `(tournament_id, participant)`
3. **Settlement Authorized**: `(symbol_short!("settle"), symbol_short!("auth"))` $\to$ `(tournament_id, settlement_hash, total_allocated)`
4. **Recipient Payout**: `(symbol_short!("payout"), symbol_short!("recip"))` $\to$ `(tournament_id, recipient, amount)`
5. **Settlement Completed**: `(symbol_short!("settle"), symbol_short!("done"))` $\to$ `(tournament_id, total_amount, recipient_count)`
6. **Treasury Configured**: `(symbol_short!("treasury"), symbol_short!("set"))` $\to$ `(treasury_address, fee_bps)`
