# Stellar Splash Contracts - Error Codes & Troubleshooting Catalog

When interacting with the `TournamentVaultContract` (`CACHWJEZY6JN36VFACWRQ7FP4T5EPRRRNBVLOSFHXTJHETZY36JD7QYJ`), Soroban reverts return specific error codes or panic symbols.

---

## 🛑 Error Codes Reference

| Error Code | Name | Cause | Resolution |
|---|---|---|---|
| `1` | `NotInitialized` | Contract invoked prior to admin initialization. | Execute `initialize(&admin)` once upon deployment. |
| `2` | `AlreadyInitialized` | Attempted re-initialization of admin authority. | Invariant violation; admin key is permanent. |
| `3` | `ContractPaused` | Circuit breaker tripped by administrator. | Wait for admin recovery or inspect system status. |
| `4` | `Unauthorized` | Caller lacks required cryptographic signature. | Ensure `require_auth()` is signed by matching key. |
| `5` | `TournamentNotFound` | Tournament ID does not exist in registry. | Query valid tournament ID from indexer. |
| `6` | `TournamentNotActive` | Registration or score submitted on closed tournament. | Check match window and lifecycle state. |
| `7` | `TournamentFull` | Participant count exceeds `max_players`. | Register for an upcoming tournament pool. |
| `8` | `AlreadyRegistered` | Player address already registered for this tournament. | Cannot register multiple entries with identical key. |
| `9` | `InvalidAmount` | Entry fee deposit or payout amount is zero/negative. | Provide positive 64-bit integer amount. |
| `10` | `BasisPointsOverflow`| Total payout basis points does not equal 10,000. | Reconcile prize split to sum exactly to 10,000 bps. |
| `11` | `SettlementWindowExpired` | Payout attempted past authorized settlement window. | Submit dispute arbitration or trigger refund. |
