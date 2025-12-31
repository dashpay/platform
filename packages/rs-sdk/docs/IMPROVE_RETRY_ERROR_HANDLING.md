# Improve Retry Error Handling

## Problem

When all DAPI addresses become banned due to retryable errors (like Proof verification errors), the SDK returns a generic "no available addresses to use" error instead of the actual underlying error that caused all addresses to be banned.

Example: `getDataContractHistory()` fails with proof verification error, but after all addresses are banned, the user sees:
```
Dapi client error: no available addresses to use
```

Instead of the meaningful:
```
Proof verification error: <actual error details>
```

## Root Cause

The SDK retry loop in `packages/rs-sdk/src/sync.rs` uses the `backon` crate for retry logic. The `.when()` and `.notify()` callbacks only receive references to errors, not ownership. This makes it impossible to store the last meaningful error for later retrieval without cloning.

The `ExecutionError<E>` type doesn't require `E: Clone`, so we cannot store errors during the retry loop.

## Proposed Solution

Add `Clone` bound to the error type `E` in the retry function, allowing the last meaningful error to be stored and returned when all addresses become unavailable.

### Changes Required

1. **Add `Clone` derive to SDK Error** (`packages/rs-sdk/src/error.rs`):
   - `Error` enum needs `#[derive(Clone)]`

2. **Add `Clone` to nested error types** - these types are embedded in SDK `Error`:
   - `drive::error::Error` (packages/rs-drive)
   - `drive::error::proof::ProofError` (packages/rs-drive)
   - `drive_proof_verifier::Error` (packages/rs-drive-proof-verifier)
   - `ProtocolError` (packages/rs-dpp)
   - `DapiClientError` (packages/rs-dapi-client)
   - `dpp::dashcore::Error` (external crate - may need wrapper)
   - `dashcore_rpc::Error` (external crate - may need wrapper)
   - `ContextProviderError` (packages/rs-context-provider)
   - `StaleNodeError` (packages/rs-sdk - already simple enum)
   - `StateTransitionBroadcastError` (packages/rs-sdk)
   - `ConsensusError` (packages/rs-dpp)

3. **Update retry function signature** (`packages/rs-sdk/src/sync.rs`):
   ```rust
   pub async fn retry<Fut, FutureFactoryFn, R, E>(
       address_list: &AddressList,
       settings: RequestSettings,
       future_factory_fn: FutureFactoryFn,
   ) -> ExecutionResult<R, E>
   where
       Fut: Future<Output = ExecutionResult<R, E>>,
       FutureFactoryFn: FnMut(RequestSettings) -> Fut,
       E: CanRetry + Display + Debug + Clone,  // Add Clone bound
   ```

4. **Store last error in retry loop**:
   ```rust
   let last_error: Arc<Mutex<Option<ExecutionError<E>>>> = Arc::new(Mutex::new(None));

   // In .when() callback:
   .when(|e| {
       if e.can_retry() {
           *last_error.lock().unwrap() = Some(e.clone());
           // ... existing retry logic
       }
   })
   ```

5. **Return stored error when NoAvailableAddresses**:
   After retry loop completes with `NoAvailableAddresses`, check if we have a stored error and return that instead.

## Affected Tests

- `packages/wasm-sdk/tests/functional/contracts.spec.mjs` - `getDataContractHistory()` test is skipped pending this fix

## Notes

- External crates (`dashcore`, `dashcore_rpc`) may not implement `Clone` on their error types. In this case, wrapper types or `Arc<E>` may be needed.
- This is a breaking change if any downstream code relies on the error type not implementing `Clone`.
- Consider whether `Clone` should be required at the `DapiRequestExecutor` trait level or just in specific retry implementations.
