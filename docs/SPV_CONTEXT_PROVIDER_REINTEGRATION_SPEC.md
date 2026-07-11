# SPV Context Provider — Re-integration onto v4.1-dev

Status: DRAFT for review · Branch: `review/ios-spv-quorums` (PR #3417 merged onto `v4.1-dev`)

## 1. Problem

PR #3417 wires the SPV-synced masternode quorum data into the Platform SDK's
`ContextProvider` so proof verification uses locally-synced, trustless quorum
public keys instead of the centralized `TrustedHttpContextProvider`
(`quorums.*.networks.dash.org`).

The PR was authored against a Swift/FFI layout that `v4.1-dev` has since
**deleted and restructured** (827 commits of drift). The merge compiles on the
Rust side after conflict resolution, but the PR's integration surface is
**architecturally obsolete**:

- `dash_sdk_create_with_spv_context(config, spv_client: *mut c_void)` casts to
  `*mut FFIDashSpvClient` — a type base **no longer produces in live code**.
  Nothing can call it correctly.
- Base deleted the Swift files the PR builds on: `SPVClient.swift`,
  `WalletService.swift`, `UnifiedAppState.swift`. SPV is now owned by
  `PlatformWalletManager` (`platform_wallet_manager_spv_*` FFI over the
  manager handle).
- The PR's standalone `SpvContextProvider` (hand-walks
  `engine.masternode_lists_around_height`) overlaps with a lookup base already
  ships: `SpvRuntime::get_quorum_public_key` (`spv/runtime.rs:156`) — but base's
  version is **dead code (zero callers, zero tests) and contains a byte-order
  bug**: it applies `QuorumHash::from_byte_array(qh).reverse()`. The
  `.reverse()` is **wrong**. `quorum_hash` flows drive-abci→tenderdash→SDK in
  internal byte order (proven by the consensus-equality check at
  `finalize_block_proposal/v0/mod.rs:131`), and the dash-spv engine keys its
  quorum map in that same internal order, so the correct lookup key is
  `QuorumHash::from_byte_array(qh)` with **no reverse** — which is exactly what
  the PR's provider AND the old `dash-spv-ffi` reference provider both do.
  Base's `.reverse()` makes every real lookup miss → `QuorumNotFound` →
  fail-closed rejection, silently masked by the Swift trusted fallback (feature
  looks alive but never uses SPV).

## 2. Chosen approach

Replace the dead `FFIDashSpvClient` path with a thin `ContextProvider` that
**delegates to `SpvRuntime::get_quorum_public_key`** (base's method walks back
up to 4 active quorum windows and skips `Invalid` entries — strictly more
robust than the PR's single-nearest-list walk, which false-misses a signing
quorum selected several DKG intervals back), wired through the
`PlatformWalletManager` handle.

**Prerequisite fix (Layer 0):** first correct base's dead method — remove the
erroneous `.reverse()` in `SpvRuntime::get_quorum_public_key`
(`spv/runtime.rs:168`) so it becomes `QuorumHash::from_byte_array(quorum_hash)`,
and land a regression test (§5) that pins the correct byte order with a real
proof. Delegating to it *before* this fix would silently disable SPV.

Three layers on top:

### Layer 1 — `platform-wallet` (generic crate, gated `spv-context`)

Rewrite `spv_context_provider.rs`:

```rust
pub struct SpvContextProvider {
    spv: Arc<SpvRuntime>,          // non-generic; SpvRuntime is concrete
    handle: tokio::runtime::Handle, // captured at construction (see risk #1)
    network: Network,
}

impl ContextProvider for SpvContextProvider {
    fn get_quorum_public_key(&self, qt: u32, qh: [u8;32], h: u32)
        -> Result<[u8;48], ContextProviderError>
    {
        // Bridge sync trait method -> the (reversal-corrected) async lookup.
        // Called from inside the SDK's BigStackRuntime block_on (verify path).
        // block_in_place avoids the nested-runtime panic; the STORED handle
        // (not Handle::current()) keeps this correct even if a future caller
        // invokes us off the runtime thread. BigStackRuntime is multi-thread
        // (confirmed: Builder::new_multi_thread, rs-sdk-ffi/src/runtime.rs:60),
        // so block_in_place does not panic.
        tokio::task::block_in_place(|| {
            self.handle.block_on(self.spv.get_quorum_public_key(qt, qh, h))
        })
        .map_err(|e| ContextProviderError::InvalidQuorum(e.to_string()))
    }
    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, _> {
        match self.network {
            Mainnet => Ok(1_888_888), Testnet => Ok(1_289_520),
            Devnet | Regtest => Ok(1),           // FIX vs PR: Regtest was Err
            _ => Err(...),
        }
    }
    fn get_data_contract(..) -> Ok(None)          // served by SDK cache
    fn get_token_configuration(..) -> Ok(None)
}
```

Deletes the PR's engine-walking logic, the separate `Arc<RwLock<Engine>>`, the
`try_read()` design, and the missing-reversal bug — all now base's concern.

`PlatformWalletManager` already stores `spv_manager: Arc<SpvRuntime>` and
exposes an Arc-cloning accessor (`manager/accessors.rs`).

### Layer 2 — FFI entry point lives in `platform-wallet-ffi` (NOT rs-sdk-ffi)

`rs-sdk-ffi` cannot reach the manager handle: base made
`platform-wallet-ffi → rs-sdk-ffi`, so the reverse edge is a cycle. The manager
storage (`PLATFORM_WALLET_MANAGER_STORAGE : HandleStorage<PlatformWalletManager<FFIPersister>>`)
lives in `platform-wallet-ffi`, which **can** depend on `rs-sdk-ffi`.

**Required rs-sdk-ffi change (Blocker 1):** `ContextProviderWrapper` is
`pub(crate)` and there is no public API to turn a native `impl ContextProvider`
into a `*mut ContextProviderHandle` (only a C-callbacks constructor exists).
Make the wrapper usable from `platform-wallet-ffi`: mark
`ContextProviderWrapper` + its `new` `pub`, so the FFI entry point can build the
handle directly:

```rust
// platform-wallet-ffi call site
let wrapper = Box::new(rs_sdk_ffi::ContextProviderWrapper::new(provider));
let cp_handle = Box::into_raw(wrapper) as *mut rs_sdk_ffi::ContextProviderHandle;
```

New fn (note: builds `DashSDKConfigExtended`, not a bare `DashSDKConfig` —
Blocker 2; `dash_sdk_create_extended` takes `*const DashSDKConfigExtended`):

```rust
// platform-wallet-ffi/src/spv.rs
pub unsafe extern "C" fn platform_wallet_manager_create_sdk_with_spv_context(
    manager_handle: Handle,
    config: *const DashSDKConfig,           // borrowed; copied into Extended
) -> DashSDKResult {
    // 1. Arc<SpvRuntime> = STORAGE.with_item(handle, |m| m.spv_arc())   (owned clone)
    // 2. let provider = platform_wallet::SpvContextProvider::new(spv, handle, network)
    // 3. let cp_handle = rs_sdk_ffi::context_provider_handle_from_provider(provider)
    // 4. let ext = DashSDKConfigExtended {
    //        base_config: (*config with all 7 fields),
    //        context_provider: cp_handle, core_sdk_handle: null,
    //    };
    //    rs_sdk_ffi::dash_sdk_create_extended(&ext)
}
```

Removes from `rs-sdk-ffi`: the dead `dash_sdk_create_with_spv_context`, the
`dash-spv-ffi` dependency, and the `platform-wallet` `spv-context` feature
dependency (moves to `platform-wallet-ffi`). Also removes `dash-spv-ffi` from
`[workspace.dependencies]` (only added during merge to prop up the dead path).

### Layer 3 — Swift

- `SDK.swift`: replace `init(network:spvClientHandle:)` with
  `init(network:walletManager:)` calling the new FFI fn with
  `walletManager.handle`.
- Accept base's deletion of `SPVClient.swift` / `WalletService.swift` /
  `UnifiedAppState.swift` (drop the PR's edits to them).
- `AppState.swift`: redo "SPV-first, trusted fallback" against
  `PlatformWalletManager` (resolve 7 conflict hunks: take base + re-add the
  fallback wired to the manager). `OptionsView.swift` toggle stays.

## 3. Alternatives rejected

- **Port the PR's standalone provider (hold `Arc<RwLock<Engine>>`, sync
  `try_read`).** Rejected: duplicates base's lookup, carries the
  `try_read` spurious-failure bug (High) and the missing hash-reversal bug, and
  must be kept in sync with base by hand.
- **Put the FFI fn in `rs-sdk-ffi` taking the manager handle.** Rejected:
  cannot resolve `PlatformWalletManager<FFIPersister>` without depending on
  `platform-wallet-ffi` → dependency cycle.
- **arc-swap snapshot published by SPV sync (lock-free reads).** Best long-term
  for the async-bridge cost, but requires upstream `dash-spv` changes. Out of
  scope; revisit if the block_on bridge proves problematic.

## 4. Failure modes / risks

0. **Byte-order regression (HIGHEST — the feature-killer).** Base's
   `.reverse()` must be removed (Layer 0). Without the fix, every lookup misses
   and SPV is silently dead behind the fallback. Pinned by a real-proof
   regression test (§5). This is the one that decides whether the feature works
   at all.
1. **Async-bridge re-entrancy.** Bridge runs inside `BigStackRuntime::block_on`.
   `block_in_place` is valid because BigStackRuntime is **confirmed
   multi-threaded** (`Builder::new_multi_thread`, `rs-sdk-ffi/src/runtime.rs:60`)
   — no panic. No deadlock: `get_quorum_at_height` (dash-spv rev `1860089`) is
   **pure in-memory** (two brief tokio-RwLock reads, no network I/O), and every
   `SpvRuntime.client` writer drops its guard before any long await, so the
   single external blocked reader can always be woken. NOTE the behavioral
   change: base's `.read().await` **waits** on write contention — the lookup is
   now **fail-slow** (blocks until the writer releases), not fail-fast (the PR's
   `try_read()` errored). Under a large QRINFO `apply_diff` this briefly stalls
   the verify worker (throughput risk, not deadlock). Validation: a Rust test
   that calls the provider from inside `block_on` while a writer holds the
   client lock and asserts it **returns the correct key once the writer
   releases** (NOT that it errors — the old fail-fast expectation is gone).
2. **Quorum coverage during sync.** Same as base: if SPV hasn't synced the list
   at the proof's `core_chain_locked_height`, lookup errors → proof rejected
   (fail-closed). Trusted fallback (Swift) covers construction; per-lookup
   misses still surface as errors (documented; acceptable for v1).
3. **Lifetime.** Provider holds `Arc<SpvRuntime>`; safe if the manager is
   dropped while the SDK lives. But if SPV is stopped/restarted (network
   switch), the `Arc<SpvRuntime>` may point at a stopped runtime → lookups
   error until re-init. Swift must rebuild the SDK on network switch (base's
   AppState already tears down/rebuilds).

## 5. Verification plan

- Rust: `cargo clippy -p platform-wallet --features spv-context -p platform-wallet-ffi -p rs-sdk-ffi --all-targets`; `cargo fmt --check`.
- **Byte-order regression test (risk #0, mandatory):** with a real testnet
  proof + a synced masternode list containing the signing quorum, assert
  `SpvRuntime::get_quorum_public_key` returns the correct 48-byte key. Must be
  RED against the `.reverse()` version and GREEN after removal — this is the
  proof the fix is real, not a tautology. Do NOT rely on the manual iOS toggle
  to catch this (too easy to skip; the fallback masks it).
- Rust test (risk #1): provider returns the correct key from within a
  `block_on`, and **completes with the correct key once a holding writer
  releases** (fail-slow) — assert no panic/deadlock; do NOT assert an error.
- **`get_platform_activation_height` values RESOLVED.** The PR's
  `1_888_888`/`1_289_520` were `dash-spv-ffi` "needs verification" placeholders.
  Matched to the production `rs-sdk-trusted-context-provider` values instead
  (Mainnet `2_132_092`, Testnet `1_090_319`, Devnet/Regtest `1`) so the SPV and
  trusted paths gate proof verification identically.
- iOS: `cd packages/swift-sdk && ./build_ios.sh`; build SwiftExampleApp
  (`iPhone 17` sim per repo note); manual: switch to testnet, confirm proof
  verification succeeds against SPV quorums with the trusted-fallback toggle OFF.
- Confirm no consensus/wire impact (this is client-side proof verification only;
  the `.reverse()` removal touches a zero-caller method, not consensus code).

## 6. Out of scope

- Composite provider (SPV primary + trusted fallback) inside Rust so per-lookup
  misses degrade gracefully — follow-up.
- `verified`-status gate on quorum entries (base's `get_quorum_at_height`
  concern; track separately).
- Tests for the broader SPV sync path.
