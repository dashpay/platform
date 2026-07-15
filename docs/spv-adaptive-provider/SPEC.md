# Adaptive SPV Context Provider

## Purpose

Platform proof verification uses one context-provider identity for the lifetime of an SDK. Runtime policy changes update atomic state inside that provider; they never replace the SDK provider. This removes clone-sensitive provider swapping while preserving trusted, SPV, and automatic quorum-source behavior.

## Design

`rs-sdk-ffi` owns `AdaptiveContextProvider` so neither `dash-sdk` nor `platform-wallet` depends on the other. It contains:

- the trusted provider used at SDK construction;
- an initially empty, atomically replaceable SPV source containing an `Arc<dyn ContextProvider>` and a synchronous readiness callback;
- an `AtomicU8` mode: Auto, SPV, or Trusted.

The trusted SDK constructor installs `Arc<AdaptiveContextProvider>` in `SdkBuilder` exactly once and retains that same `Arc` in `SDKWrapper`. Creating a `PlatformWalletManager` from an SDK automatically creates a `SpvContextProvider` for its `SpvRuntime` and populates the adaptive provider's SPV source. Source population changes only the adaptive provider's internal empty slot; it does not mutate `dash_sdk::Sdk` or replace its context provider. Re-populating the source is rejected so an SDK cannot silently change to a different manager/runtime.

`SpvRuntime` maintains a lock-free readiness bit from dash-spv progress callbacks. It is true only when both header and masternode-list progress are present and `Synced`, exactly matching the current Swift Auto policy. Start, stop, and run-loop exit clear readiness. The adaptive provider reads this bit synchronously, so quorum lookup does not block on an async lock merely to select a source.

## Routing

For `get_quorum_public_key`:

- Trusted always calls the trusted provider.
- SPV calls only the SPV provider. An absent source or an unready source returns an error; it never falls back to trusted.
- Auto calls trusted while the SPV source is absent or not ready, then calls only SPV once ready. An SPV lookup miss after readiness is an error and never falls back to trusted.

`get_data_contract` and `get_token_configuration` always call the trusted provider. That provider resolves embedded system contracts and the SDK-populated cache and has no fallback provider, preserving the existing composite provider's trust boundary.

`get_platform_activation_height` returns the existing per-network constant directly: mainnet 2,132,092; testnet 1,090,319; devnet/regtest 1.

The SPV provider continues to reject any quorum type other than `network.platform_type()` before lookup, reverse the proof's display-order quorum hash through `SpvRuntime::quorum_lookup_key`, and fail closed outside a multi-thread Tokio runtime. The adaptive layer never retries a failed SPV lookup against trusted state.

## FFI and Swift

The SDK FFI exposes mode set/get and active-source inspection. Mode values are validated at the boundary. `PlatformWalletManager.configure(sdk:)` passes the SDK handle to manager creation; manager creation obtains the inner SDK clone and attaches its SPV source before returning.

Active-source inspection reports Trusted in Trusted mode; SPV in forced-SPV mode once a source exists; and, in Auto, Trusted until the source is ready and SPV afterwards. Forced SPV with no populated source reports Trusted for the indicator even though quorum lookups fail closed, because no SPV source exists to identify as active.

The install/restore FFIs and Swift `SDK.attachSpvQuorums` / `restoreTrustedQuorums` APIs are removed. Swift exposes a single mode-setting method. `AppState.applyQuorumMode` always calls it once with Auto/SPV/Trusted and then refreshes the active-source indicator; it does not decide routing or attach providers. Existing progress observation remains useful for refreshing the "Proof Quorum Source" indicator as Auto becomes ready.

## Feasibility and Security Review

The construction cycle is resolved at the FFI boundary: the SDK creates the fixed adaptive provider first; standard manager creation synchronously borrows the opaque SDK handle, clones its inner `Sdk`, constructs `SpvRuntime`, and then fills the adaptive provider's internal SPV slot before returning. The manager retains the cloned `Sdk`, never the raw SDK handle. `rs-sdk-ffi` only knows a trait-object provider plus readiness callback, while `rs-platform-wallet-ffi` creates both from `platform-wallet`; therefore no Rust dependency cycle is introduced. The advanced raw-inner-SDK manager constructor may remain available, but cannot attach an SPV source because it has no owning SDK handle.

The provider and readiness callback are published together in one immutable SPV-source object, preventing a provider/readiness mismatch. The slot is write-once: a second manager cannot redirect an SDK's SPV trust root. Mode values use acquire/release atomic ordering and invalid FFI values are rejected.

Each quorum call snapshots mode, source, and readiness. A concurrent readiness transition may affect the next call, but cannot create a trusted fallback after a call selected SPV. If SPV stops after selection, the SPV lookup fails closed. If readiness becomes true just after Auto selected trusted, that single in-flight call retains the prior trusted selection and subsequent calls use SPV; this is an expected boundary race, not a trust downgrade controlled by proof data. Readiness is explicitly cleared before every start/restart, during stop, and on every run-loop exit including errors.

Readiness is derived from trusted local sync state, not from whether a requested quorum happens to exist. Consequently an attacker-induced SPV lookup miss after sync cannot trigger trusted retry. Contract/token routing is structurally independent of mode and SPV state, retaining only the trusted provider's cache and embedded-system-contract behavior.

## Verification

- Unit-test adaptive routing for all three modes, absent/unready SPV fail-closed behavior, Auto readiness transition, post-readiness SPV miss behavior, and trusted-only contract/token routing.
- Retain the quorum-hash byte-order regression test and the SPV quorum-type/current-thread fail-closed behavior.
- Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-features`.
- Build the simulator framework, then clean-build `SwiftExampleApp` with `xcodebuild`.
- On a synced testnet device, select SPV and perform proof-verified Platform queries; confirm successful results and the `SPV context provider served quorum public key` log.
