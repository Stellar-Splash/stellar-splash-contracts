# Stellar Splash - Soroban Contract Threat Model & Security Invariants

## 1. Threat Landscape
Stellar Splash secures monetary tournament pools and leaderboard settlements on Soroban. The smart contract attack surface is evaluated against the following threat vectors:

### 1.1 Frontrunning & MEV Exploitation
- **Attack Vector**: An attacker monitors the mempool for settlement transactions and attempts to frontrun payouts or inject fabricated scores before the settlement window closes.
- **Mitigation**: Match settlement requires a cryptographic engine attestation hash (`keccak256(replay_payload)`) signed by the verified settlement channel account. Scores cannot be injected retroactively without matching the cryptographic attestation seed recorded at entry time.

### 1.2 Reentrancy Attacks
- **Attack Vector**: Malicious recipient contracts attempting reentrancy during prize disbursement to drain pool balances.
- **Mitigation**: Soroban's execution environment executes invocations synchronously with strictly isolated call stacks. Token transfers use standard SEP-41 token clients with Checks-Effects-Interactions patterns where tournament state is marked `Settled` prior to token transfers.

### 1.3 Flash-Loan / Pool Draining Attacks
- **Attack Vector**: Borrowing massive liquidity to manipulate prize pool calculations or create fake sponsorship deposits.
- **Mitigation**: Sponsorship tokens are transferred into the contract vault immediately and locked until final settlement. No liquidity pool interactions or flash-loanable pricing oracles are used. All prize mathematics operate strictly on basis points of actual vaulted reserves.

### 1.4 State Bloat & Storage Rent Exhaustion
- **Attack Vector**: Flooding the contract with millions of dummy tournaments or player entries to force storage exhaustion or ledger TTL expiration.
- **Mitigation**: Each entry requires a non-refundable gas fee and entry fee deposit. The contract automatically triggers `bump_persistent` extending data lifetime by $3,110,400$ ledgers (~6 months) whenever legitimate storage mutations occur.

### 1.5 Emergency Circuit Breaking
- **Invariants**:
  - `is_paused()` immediately freezes deposits and settlements.
  - Emergency refunds (`claim_refund`) remain active during pause if a tournament has timed out without settlement ($T > T_{\text{end}} + 86,400\text{s}$).
  - Admin cannot seize escrowed player funds directly; only programmatic refunds to original depositors are authorized.

