# Stellar Splash - Fair Play Cryptographic Attestation Specification

## 1. Abstract
In real-time multiplayer arcade gaming, client-side score fabrication is the predominant vulnerability. Stellar Splash solves this without high-latency on-chain game loops by implementing an off-chain deterministic physics simulation coupled with on-chain cryptographic replay attestations.

## 2. Attestation Architecture

```
 Player Client                  Backend Engine                     Soroban Vault
   (Browser)                  (Deterministic Node)                   (On-Chain)
       │                                │                                │
       │ 1. Request Match Seed          │                                │
       │───────────────────────────────>│                                │
       │ 2. Return Signed Match Seed    │                                │
       │<───────────────────────────────│                                │
       │                                │                                │
       │ 3. Execute 60Hz Physics Ticks  │                                │
       │    Record Input Delta Log      │                                │
       │                                │                                │
       │ 4. Submit Replay & Hash        │                                │
       │───────────────────────────────>│                                │
       │                                │ 5. Headless Re-Simulation      │
       │                                │    Verify Physics Invariants   │
       │                                │    Compute Score & SHA-256     │
       │                                │                                │
       │                                │ 6. Channel Account Settlement  │
       │                                │───────────────────────────────>│
       │                                │    Batch Payouts + Merkle Root │
       │                                │                                │
```

## 3. Cryptographic Verification Primitives

### 3.1 Match Seed Derivation
At the start of a match, the engine signs a seed composed of:
$$\text{MatchSeed} = \text{SHA256}(\text{TournamentID} \parallel \text{PlayerAddress} \parallel \text{LedgerSequence} \parallel \text{ServerEntropy})$$

This seed seeds the Pseudo-Random Number Generator (PRNG - XorShift128+) in both the client canvas and the headless engine validator. Obstacle placements, powerup timings, and hazard velocities are completely deterministic.

### 3.2 Input Vector Encoding
The client sends an array of delta-encoded input frames:
```json
{
  "tick": 1420,
  "action": "JUMP",
  "angle": 1.482,
  "velocity": 12.5,
  "hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}
```

### 3.3 Movement Entropy Validation
The engine analyzes the Shannon entropy of directional changes ($H \ge 2.1$). Inhuman linear paths, instantaneous teleportation, or scripted bot sequences fail entropy gates and trigger dispute logging.

### 3.4 Merkle Root Anchor on Soroban
For tournaments with over 50 participants, payouts are verified via a Merkle Tree:
- **Leaf**: $\text{Leaf}_i = \text{SHA256}(\text{Rank}_i \parallel \text{PlayerAddress}_i \parallel \text{Amount}_i \parallel \text{Score}_i)$
- **Contract State**: The contract stores the Merkle root $\mathcal{R}_{\text{tournament}}$ and allows winners to claim prizes trustlessly by presenting their Merkle proof path.
