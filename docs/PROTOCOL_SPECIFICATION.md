# Stellar Splash - Soroban Smart Contract Protocol Specification

## 1. Overview
The `TournamentVaultContract` (`CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`) serves as the non-custodial decentralized clearinghouse and escrow mechanism for the Stellar Splash multiplayer competitive gaming ecosystem. Built for Soroban on Stellar Protocol 21/22, it governs the entire tournament lifecycle, escrow custody, multi-tier prize payouts, creator royalties, emergency circuit breaking, and Soulbound Token (SBT) champion badges.

## 2. Protocol Architecture & State Machine

### 2.1 Tournament Lifecycle States
Each tournament progresses deterministically through four distinct lifecycle states:
```
           ┌─────────────┐
           │   Created   │ ◄── Admin/Creator initializes tournament parameters
           └──────┬──────┘
                  │  Player deposits entry fee (XLM/custom token)
                  ▼
           ┌─────────────┐
           │   Active    │ ◄── Match window open; players submit gameplay runs
           └──────┬──────┘
                  │  Match window concludes & verification finishes
                  ▼
           ┌─────────────┐
           │   Settled   │ ◄── Automated settlement engine commits validated leaderboard
           └──────┬──────┘
                  │  Payouts disbursed proportionally according to BPS
                  ▼
           ┌─────────────┐
           │  Archived   │ ◄── Ledger TTL maintained via persistent storage bump
           └─────────────┘
```

Alternatively, if a tournament is halted before completion, it transitions to `Cancelled`, unlocking 100% refund claims for all deposited player balances.

### 2.2 Storage Footprint & Data Keys
Soroban data storage is partitioned into **Instance**, **Persistent**, and **Temporary** storage tiers:

| Key Symbol | Storage Tier | Lifetime (Ledgers) | Description |
|---|---|---|---|
| `Admin` | Instance | Instance Lifetime | Global contract administrative authority address |
| `ContractVersion` | Instance | Instance Lifetime | Current contract revision code (`21` for Protocol 21) |
| `IsPaused` | Instance | Instance Lifetime | Emergency kill-switch boolean flag |
| `Tournament(id)` | Persistent | 3,110,400 ledgers (~6 months) | Canonical metadata, prize pool, player capacity, and state |
| `PlayerEntry(id, addr)` | Persistent | 3,110,400 ledgers | Player registration status and entry fee deposit receipt |
| `TournamentScore(id, addr)` | Persistent | 3,110,400 ledgers | Attested high score, tick sequence, and anti-cheat hash |
| `SbtBadge(id, addr)` | Persistent | Indefinite (Auto-bumped) | Soulbound non-transferable champion achievement badge |
| `Sponsorship(id, sponsor)` | Persistent | 3,110,400 ledgers | External sponsor prize pool injection records |

### 2.3 Authorization Model
All state-modifying invocations enforce strict Stellar Soroban cryptographic authorization:
- `player.require_auth()`: Verified during tournament registration and deposit.
- `admin.require_auth()`: Verified during contract upgrades, emergency pause, settlement overrides, and treasury withdrawals.
- `settlement_engine.require_auth()`: Authorized via channel account key for atomic batch payouts.

## 3. Mathematical Safety & Basis Points
All financial calculations operate on 64-bit unsigned fixed-point integers using Basis Points ($1\text{ bps} = 0.01\%$, $10,000\text{ bps} = 100\%$):
- Creator Royalty: Capped at maximum $1,500\text{ bps}$ ($15.0\%$).
- Protocol Fee: Fixed at $250\text{ bps}$ ($2.5\%$).
- Net Prize Distribution: Exactly $10,000 - (\text{Royalty} + \text{Protocol Fee})$ basis points allocated across ranked positions.
- Invariant: $\sum_{i} \text{Payout}_i + \text{Royalty} + \text{Fee} \equiv \text{TotalPool}$. No tokens can ever be trapped or minted.
