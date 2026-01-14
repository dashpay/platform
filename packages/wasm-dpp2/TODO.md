# wasm-dpp2 TODO

## Type Wrappers for Flexible Input

### ~~ProTxHash Wrapper~~ ✅ DONE

~~Create a wrapper around `ProTxHash` from dashcore with flexible input types.~~

```typescript
type ProTxHashLike = ProTxHash | Uint8Array | string;
```

**Implemented in:**

- `wasm-dpp2/src/enums/pro_tx_hash.rs` - `ProTxHashWasm` with `TryFrom<JsValue>`, `from_hex()`, `from_bytes()`, `try_from_options()`, `try_from_options_optional()`
- `wasm-sdk/src/queries/protocol.rs` - Updated `getProtocolVersionUpgradeVoteStatus`, `getProtocolVersionUpgradeVoteStatusWithProofInfo`
- `wasm-sdk/src/queries/epoch.rs` - Updated `getEvonodesProposedEpochBlocksByIds`, `getEvonodesProposedEpochBlocksByIdsWithProofInfo`, `parse_evonode_range_query`

### ~~Network Wrapper~~ ✅ DONE

~~Create a wrapper around `Network` from dashcore with flexible input types:~~

```typescript
type NetworkLike = Network | "mainnet" | "testnet" | "devnet" | "regtest";
```

**Implemented in:**

- `wasm-dpp2/src/enums/network.rs` - `NetworkWasm` with `TryFrom<JsValue>`, `try_from_options()`, `as_str()`
- `wasm-sdk/src/wallet/key_generation.rs` - Updated all functions to use `NetworkLike`
