# Stellar Community Fund (SCF) & Wave Judging Rubric Alignment

This document directly maps **Stellar Splash** architecture, features, and implementations to the official evaluation pillars of the **Stellar Community Fund (SCF)** and developer grants.

---

## 1. Executive Summary & Value Proposition

**Stellar Splash** is an open-source, trustless competitive gaming and prize settlement platform powered by **Soroban Smart Contracts** on Stellar Testnet. It solves the endemic problems of traditional online tournaments—organizer fraud, opaque scoring, delayed payouts, and custodial risk—by enforcing **deterministic, mathematically verified prize allocation** directly on-ledger.

* **Target Deployed Soroban Contract**: `CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`
* **Stellar Network**: Testnet (Passphrase: `Test SDF Network ; September 2015`)
* **StellarExpert Explorer**: [View Deployed Contract](https://stellar.expert/explorer/testnet/contract/CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ)

---

## 2. Alignment Matrix with SCF Criteria

| SCF Evaluation Pillar | Stellar Splash Feature & Architecture | Codebase Reference |
| :--- | :--- | :--- |
| **Ecosystem Utility** | Direct utility for native XLM and SAC USDC tokens via SEP-41 token interfaces and delegated allowance funding (`fund_prize_pool_from`). | `contracts/src/lib.rs:fund_prize_pool_from` |
| **Technical Excellence** | Production-ready Rust Soroban contract with strict 10,000 basis point invariant validation, deterministic integer dust allocation to Rank 1, and TTL extension guards. | `contracts/docs/FINANCIAL_GOVERNANCE.md`, `contracts/src/lib.rs:1118-1130` |
| **Security & Auditing** | Admin-governed circuit breaker (`pause` / `unpause`), floor minimum prize enforcement (`MIN_PRIZE_AMOUNT = 10,000,000 Stroops`), and premature completion time guards (`TournamentNotEnded`). | `contracts/src/lib.rs:is_paused`, `lib.rs:complete_tournament` |
| **Verifiable Gaming** | Deterministic replay engine replaying user inputs and generating 32-byte cryptographic digests checked on Soroban prior to payout authorization. | `engine/src/services/replay.service.ts`, `engine/src/services/canonical.service.ts` |
| **Fairness & Anti-Cheat** | Seeded PRNG trajectory generation (Mulberry32) preventing client target injection; physical velocity bounds (< 35ms / 28 CPS) preventing bot auto-clickers. | `engine/src/services/prng.service.ts`, `engine/src/services/replay.service.ts:91-115` |
| **State Persistence** | Atomic temporary-file-and-rename disk storage with graceful process shutdown hooks (`SIGINT`/`SIGTERM`) ensuring zero record corruption. | `engine/src/services/persistent-store.service.ts`, `engine/src/services/shutdown.service.ts` |
| **User Experience** | Zero-asset Web Audio API procedural sound synthesizer, reactive toast container, clipboard crash diagnostics with React Error Boundary, and responsive touch gestures. | `frontend/src/services/audio-synth.ts`, `frontend/src/components/ErrorBoundary.tsx` |
| **Transparency & UI** | Instant StellarExpert deep-linking for all public keys and contract IDs, real-time live accuracy meter, multiplier glow animations, and circuit breaker banners. | `frontend/src/components/StellarExpertLink.tsx`, `frontend/src/components/CircuitBreakerBanner.tsx` |

---

## 3. Mathematical Governance & Invariants

To guarantee that no prize funds are lost, locked, or overdrawn, Stellar Splash enforces strict mathematical guarantees:

1. **10,000 Basis Points Summation**:
   $$\sum_{i=1}^{N} \text{bps}_i = 10{,}000$$
   Every prize agreement must sum to exactly 10,000 basis points (100.00%). Allocations exceeding or falling short of 10,000 bps are strictly rejected before locking.

2. **Deterministic Integer Dust Floor Allocation**:
   All fractional currency units resulting from integer division ($\text{amount} \times \text{bps} / 10{,}000$) are deterministically attributed to the 1st place winner. This ensures zero dust remains stranded in the Soroban contract vault.

3. **Total Allocation Invariant**:
   $$\sum_{i=1}^{N} \text{alloc}_i \le \text{vault\_funded\_balance}$$
   Settlement transactions cannot execute unless total allocations are mathematically bounded by the current verified vault balance.

---

## 4. Soroban Contract Verification Walkthrough

Judges and evaluators can verify contract functionality directly on Stellar Testnet:

1. **Instance Initialization & Health**:
   ```bash
   soroban contract invoke \
     --id CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ \
     --source <SOURCE_ACCOUNT> \
     --network testnet \
     -- get_admin
   ```

2. **Immutable Config Query**:
   ```bash
   soroban contract invoke \
     --id CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ \
     --source <SOURCE_ACCOUNT> \
     --network testnet \
     -- get_tournament_config \
     --tournament_id 101
   ```

3. **Paginated Participant Query**:
   ```bash
   soroban contract invoke \
     --id CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ \
     --source <SOURCE_ACCOUNT> \
     --network testnet \
     -- get_participants_page \
     --tournament_id 101 \
     --start_index 0 \
     --limit 10
   ```

---

## 5. Test Suite Verification

Every component in the Stellar Splash architecture contains exhaustive unit and integration test coverage:

* **Smart Contracts**: `cargo test` in `stellar-splash-contracts` executes 12 test suites verifying token allowances, circuit breakers, time guards, zero-amount rejections, settlement execution, and storage TTLs.
* **Engine**: `npm test` in `stellar-splash-engine` executes 13 test suites (73+ tests) verifying PRNG determinism, sliding window rate limits, replay verifications, and graceful shutdown.
* **Frontend**: `npm test` in `stellar-splash-frontend` executes 16 test suites (50+ tests) verifying component rendering, Web Audio synthesis, ErrorBoundary fallbacks, and 10,000 bps dust math.
