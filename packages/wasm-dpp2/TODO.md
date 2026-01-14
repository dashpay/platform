# wasm-dpp2 TODO

## Type Wrappers for Flexible Input

### ProTxHash Wrapper

Create a wrapper around `ProTxHash` from dashcore with flexible input types.

`ProTxHash` is a newtype wrapper around `sha256d::Hash`:

```rust
pub struct ProTxHash(sha256d::Hash);
```

```typescript
type ProTxHashLike = ProTxHash | Uint8Array | string;
```

**Requirements:**

- Create `ProTxHashWasm` wrapper in wasm-dpp2
- Accept hex string, raw bytes (Uint8Array), or ProTxHash object
- Centralize parsing logic (currently duplicated in wasm-sdk methods)

**Affected areas in wasm-sdk:**

- `getProtocolVersionUpgradeVoteStatus` (startProTxHash parameter)
- `getProtocolVersionUpgradeVoteStatusWithProofInfo` (startProTxHash parameter)
- `getEvonodesProposedEpochBlocksByIds` (ids parameter)
- `getEvonodesProposedEpochBlocksByIdsWithProofInfo` (proTxHashes parameter)
- `EvonodeProposedBlocksRangeQuery.startAfter` field

### ~~Network Wrapper~~ ✅ DONE

~~Create a wrapper around `Network` from dashcore with flexible input types:~~

```typescript
type NetworkLike = Network | "mainnet" | "testnet" | "devnet" | "regtest";
```

**Implemented in:**

- `wasm-dpp2/src/enums/network.rs` - `NetworkWasm` with `TryFrom<JsValue>`, `try_from_options()`, `as_str()`
- `wasm-sdk/src/wallet/key_generation.rs` - Updated all functions to use `NetworkLike`
