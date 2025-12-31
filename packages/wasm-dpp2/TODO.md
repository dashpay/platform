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

### Network Wrapper

Create a wrapper around `Network` from dashcore with flexible input types:

```typescript
type NetworkLike = Network | string;
```

**Requirements:**

- Create `NetworkWasm` wrapper in wasm-dpp2
- Accept string ("mainnet", "testnet", "devnet", "regtest") or Network object
- Centralize parsing logic
- Use everywhere we pass network parameter

**Affected areas:**

- SDK builder methods
- Wallet key derivation
- Address generation/validation

## Error Handling Improvements

### Add Clone to Error Types for Better Retry Error Reporting

**Problem:** When all DAPI addresses become banned due to retryable errors (like Proof verification errors), the SDK returns a generic "no available addresses to use" error instead of the actual underlying error.

**Solution:** Add `Clone` derive to error types across crates to enable storing the last meaningful error in the retry loop.

**Detailed plan:** See `packages/rs-sdk/docs/IMPROVE_RETRY_ERROR_HANDLING.md`

**Affected crates:**

- `rs-sdk` (Error enum)
- `rs-dapi-client` (DapiClientError)
- `rs-drive` (drive::error::Error, ProofError)
- `rs-drive-proof-verifier` (Error)
- `rs-dpp` (ProtocolError, ConsensusError)
- `rs-context-provider` (ContextProviderError)

**Blocked test:** `packages/wasm-sdk/tests/functional/contracts.spec.mjs` - `getDataContractHistory()` is skipped pending this fix
