# Stellar Splash - Soroban Storage Footprint & Rent Economics

## 1. Soroban State Tiering Model
On Stellar Soroban Protocol 21/22, smart contract data storage is priced dynamically according to read/write bandwidth and rent duration. Stellar Splash implements optimized data layouts to minimize state costs for players and operators.

### 1.1 Storage Type Classification
| Data Type | Storage Class | TTL Window | Cost Factor | Justification |
|---|---|---|---|---|
| Admin & Configuration | Instance | Max Lifetime | Low | Single ledger footprint, accessed on every admin call |
| Tournament Registry | Persistent | 3,110,400 ledgers | Medium | Critical prize and state metadata; auto-bumped |
| Player Entry Records | Persistent | 3,110,400 ledgers | Variable | High frequency, pruned/archived upon tournament settlement |
| Live Match Tick Log | Temporary / Off-chain | 17,280 ledgers (~1 day) | Zero on-chain | Kept in engine memory and IPFS/Arweave cold storage |

## 2. Gas & Rent Cost Analysis
Based on Protocol 21 Soroban testnet fee matrices:
- **Contract Deployment & Initialization**: ~0.08 XLM base fee + 1,200,000 CPU instructions.
- **Player Tournament Registration**: ~0.005 XLM network fee + 150,000 CPU instructions.
- **Batch Settlement (10 Winners)**: ~0.015 XLM network fee + 480,000 CPU instructions.
- **TTL Extension (`bump_persistent`)**: 0.0001 XLM per 100,000 ledgers.

## 3. Storage Optimization Techniques
1. **Compact Enum Serialization**: Tournament status is stored as a 1-byte integer enum rather than strings or complex structs.
2. **Key Compacting**: Composite keys (e.g. `(Symbol::new(&env, "Entry"), tournament_id, player_address)`) ensure $O(1)$ lookups without iterating tables.
3. **Lazy Bumping**: Persistent entries are only bumped during active writes, avoiding unnecessary ledger updates during read-only status checks.

