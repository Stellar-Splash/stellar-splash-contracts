# Changelog - Stellar Splash Contracts

All notable changes to `stellar-splash-contracts` are documented in this file.

## [1.0.1] - 2026-10-29

### Fixed
- **Clippy Lint Analysis**: Fixed implicit saturating subtraction and manual division ceiling patterns in contract implementation.
- **WASM Size Gate**: Updated `scripts/check-wasm-size.sh` to target `stellar_splash_contracts.wasm` with size ceiling checks.
- **Formatting Standardization**: Enforced clean rustfmt across all contract source and test suites.
- **Repository Metadata**: Updated GitHub description and added topics (`hackathon`, `protocol-21`, `sbt`, `web3-gaming`).

## [1.0.0] - 2026-10-29

### Added
- **Soroban Smart Contract Suite**: Deployed at `CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`.
- **Tournament Lifecycle State Machine**: Full deterministic transition flow (`Created` -> `Active` -> `Settled` / `Cancelled`).
- **Mathematical Safety & Basis Points**: 10,000 basis points zero-deficit prize clearing engine with integer dust allocation.
- **Protocol 21/22 Storage TTL Management**: Automated persistent storage bumping (`bump_persistent`) extending lifetime to 3,110,400 ledgers.
- **Emergency Circuit Breaker**: Global `is_paused()` kill switch and automated timeout refund safeguards.
- **Soulbound Tokens (SBT)**: Non-transferable on-chain champion achievement badge minting.
- **CI/CD Quality Gates**: WASM byte size profiler, Clippy linter, Cargo Audit, and formal invariant test runners.
