# Stellar Splash Contracts - Security Audit Scope & Attack Matrix

## 1. Scope of Audit
This audit scope defines the boundaries, trust assumptions, and security invariants of the `TournamentVaultContract` (`CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`).

---

## 2. Tested Security Invariants

### 2.1 Access Control Invariants
- Only `admin` can trigger emergency pause, propose WASM code upgrades, or claim protocol fees.
- Only registered players can submit match runs or claim refunds.
- Player entry fee deposits require explicit cryptographic signature authorization (`player.require_auth()`).

### 2.2 Reentrancy & Cross-Contract Calls
- All internal state mutations (`TournamentState::Settled`, balance deductions) precede external token transfer callbacks.
- Checks-Effects-Interactions (CEI) design pattern strictly maintained.

### 2.3 Economic Invariants
- $\sum \text{Payouts} + \text{Royalty} + \text{Fee} \equiv \text{VaultBalance}$.
- No negative numbers or integer overflow permitted.
- Refund claims return exactly the original deposit amount ($100\%$).

