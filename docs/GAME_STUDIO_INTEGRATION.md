# Stellar Splash - Third-Party Game Studio Integration Guide

## 1. Overview
The `TournamentVaultContract` (`CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`) is designed as a composable, modular primitive. Any independent game studio building on Stellar can integrate their web, desktop, or mobile games with Stellar Splash to provide instant, trustless tournament prize pools without writing custom smart contracts.

---

## 2. Integration Architecture

```
 External Game Client       Stellar Splash Engine         Soroban Contract Vault
 (Unity, Godot, React)      (Verification Service)            (Testnet / Mainnet)
          │                            │                               │
          │ 1. Create Tournament       │                               │
          │───────────────────────────>│                               │
          │                            │ 2. Initialize Vault On-Chain  │
          │                            │──────────────────────────────>│
          │ 3. Players Join & Deposit  │                               │
          │───────────────────────────────────────────────────────────>│
          │ 4. Game Matches Played     │                               │
          │    Submit Replay Hash      │                               │
          │───────────────────────────>│                               │
          │                            │ 5. Attest & Batch Payout      │
          │                            │──────────────────────────────>│
```

---

## 3. Composable Interfaces

### 3.1 Custom Entry Tokens (SEP-41)
Games can specify any Stellar asset (XLM, USDC, EURC, or custom studio utility token) as the tournament denomination:
```rust
client.create_tournament(&admin, &tournament_id, &token_address, &entry_fee, &max_players);
```

### 3.2 Studio Creator Royalties
Studios can earn up to 15% ($1,500\text{ bps}$) of every tournament pool:
```rust
client.set_creator_royalty_bps(&admin, &tournament_id, &1000); // 10.0%
```
Upon tournament settlement, the creator royalty is atomically deposited into the studio's treasury wallet.
