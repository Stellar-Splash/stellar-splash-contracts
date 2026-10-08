# Contributing to Stellar Splash Contracts

Thank you for contributing to **Stellar Splash Contracts**! These Soroban smart contracts manage the financial escrow vaults, tournament lifecycle states, result verification registries, and immutable prize agreement governance on Stellar.

---

## Code of Conduct

All contributors must adhere to our [Code of Conduct](CODE_OF_CONDUCT.md). Please treat all members of the community with dignity and respect.

---

## Development Setup

### Prerequisites
- **Rust**: `>= 1.79.0`
- **Cargo**: `>= 1.79.0`
- **wasm32-unknown-unknown target**:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Stellar CLI**: `>= 28.0.0`

### Getting Started
1. **Fork and Clone** the repository:
   ```bash
   git clone https://github.com/Stellar-Splash/stellar-splash-contracts.git
   cd stellar-splash-contracts
   ```
2. **Run Tests**:
   ```bash
   cargo test
   ```
3. **Build Release WASM**:
   ```bash
   stellar contract build
   ```

---

## Smart Contract Invariants

When contributing changes to smart contract logic, you must strictly uphold the following core invariants:
1. **10,000 Basis Points Rule**: Prize agreement percentages must sum to exactly 10,000 basis points ($100.00\%$).
2. **Agreement Immutability**: Once an agreement reaches `Locked`, its parameters cannot be modified in place. Any update requires superseding via a new incremented version.
3. **Double Settlement Prevention**: Settlements cannot be executed more than once per tournament.
4. **Vault Balance Integrity**: Authorized settlement sums cannot exceed the active escrow vault balance.

---

## Pull Request Guidelines

1. **Create a Feature Branch**:
   ```bash
   git checkout -b feature/contract-improvement
   ```
2. **Commit Style**:
   - Follow Conventional Commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`.
3. **Submit PR**:
   - Describe contract changes and their impact on gas/footprint.
   - Include test coverage for all new public contract functions and edge cases.
