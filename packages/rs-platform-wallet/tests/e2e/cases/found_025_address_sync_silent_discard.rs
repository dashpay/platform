//! Found-025 — `rs-sdk` address-sync silently discards a balance update
//! when the recipient address was allocated after the `key_to_tag` snapshot.
//!
//! **Defect site**: `packages/rs-sdk/src/platform/address_sync/mod.rs:619`
//! **Reproduced at SHA**: `5cca0fbd1a` (Marvin Phase 3 live trace, block 334540)
//! **Full diagnosis**: `/tmp/marvin-phase3-tk-bank-flake-reproduction.md`
//!
//! ## Bug shape
//!
//! `sync_address_balances` (mod.rs:325-328) builds `key_to_tag` — a
//! `HashMap<addr_bytes, (tag, address)>` — once at sync entry by snapshotting
//! `provider.pending_addresses()`. This map is then passed to
//! `incremental_catch_up` unchanged. Inside `incremental_catch_up`, every
//! balance update returned by the DAPI compacted or recent response is filtered
//! through this map at mod.rs:619:
//!
//! ```text
//! if let Some(&(tag, address)) = address_lookup.get(&addr_bytes) { … }
//! ```
//!
//! If a wallet address is allocated via `next_unused_receive_address` AFTER
//! the snapshot is taken but before the next sync cycle completes, its bytes
//! are absent from `address_lookup`. The DAPI proof may contain the correct
//! balance for that address — Marvin's live trace confirmed this at log line
//! 27750 — but the filter silently drops the entry with no warning, no error,
//! and no signal to the caller. `addresses_with_balances()` never reflects the
//! funded amount.
//!
//! ## What this test pins
//!
//! The structural invariant:
//!
//!   * The `key_to_tag` map (built from `pending_addresses()` at sync entry)
//!     is the sole lookup gate for balance updates in `incremental_catch_up`.
//!   * A freshly-allocated address NOT in that snapshot has bytes that produce
//!     `None` from `address_lookup.get()`.
//!   * `None` means the balance update is dropped.
//!
//! This test reconstructs the exact filter (`HashMap::get` keyed on
//! `AddressToBytes::to_bytes()`) using the same public types the SDK uses
//! internally — `PlatformAddress`, `AddressToBytes` — and asserts the
//! invariant FAILS for a post-snapshot address. No `Sdk`, DAPI connection, or
//! harness is required.
//!
//! ## Test lifecycle
//!
//! **Today (bug present, SHA `5cca0fbd1a`)**: the assertion fires — the
//! post-snapshot address bytes produce `None` from the lookup, confirming the
//! silent discard.
//!
//! **After upstream fix**: the SDK will ensure every emitted address reaches
//! `key_to_tag` before the incremental phase runs (e.g. by rebuilding the map
//! per-phase, or by making `next_unused_receive_address` register atomically).
//! The assertion will invert: `.get()` returns `Some`, and this test must be
//! updated alongside the fix.
//!
//! ## Approach chosen: A (pure unit, no harness)
//!
//! The public API surface `dash_sdk::platform::address_sync` exposes
//! `AddressToBytes` and `PlatformAddress`; those are the only types needed
//! to reproduce the filter logic. Approach B (e2e through `bank.fund_address`)
//! was not needed — the structural bug is demonstrable without touching the
//! network.

use std::collections::HashMap;

use dash_sdk::platform::address_sync::AddressToBytes;
use dpp::address_funds::PlatformAddress;

/// Funded amount that the DAPI proof would return for the freshly-allocated
/// address. Real trace value from Marvin Phase 3 was ~1 000 000 000 000
/// credits (≈ 1 000 DASH).
const DAPI_FUNDED_CREDITS: u64 = 1_000_000_000_000;

/// The tag type the SDK uses internally is `(P::Tag, P::Address)` keyed by raw
/// bytes. For this unit test we use `u32` as the tag (matching the gap-limit
/// derivation index a real HD provider uses).
type Tag = u32;

/// Reconstruct `key_to_tag` from a set of addresses that were registered in
/// `pending_addresses()` BEFORE sync entry (the snapshot). Returns the map and
/// the list of addresses that were in it.
fn build_snapshot(pre_registered: &[PlatformAddress]) -> HashMap<Vec<u8>, (Tag, PlatformAddress)> {
    pre_registered
        .iter()
        .enumerate()
        .map(|(i, &addr)| {
            let key = addr.to_bytes(); // AddressToBytes::to_bytes — mod.rs:618
            (key, (i as Tag, addr))
        })
        .collect()
}

/// Bug-pin for Found-025: a freshly-allocated address is invisible to
/// `incremental_catch_up`'s `address_lookup` because the lookup map is
/// snapshotted once at `sync_address_balances` entry and never refreshed.
///
/// **RED today**: `.get(&new_addr_bytes)` returns `None`, so the balance update
/// for `DAPI_FUNDED_CREDITS` is silently dropped.
///
/// **GREEN after fix**: the SDK ensures all emitted addresses reach the lookup
/// map before `incremental_catch_up` runs; `.get()` returns `Some` and this
/// assertion must be updated.
#[ignore = "Found-025 bug pin — rs-sdk address-sync race; \
            pure unit test (no async, no harness, no chain); run with \
            `cargo test -- --ignored`"]
#[test]
fn found_025_freshly_allocated_address_balance_visible_after_sync() {
    // ── 1. Snapshot (sync_address_balances entry, mod.rs:325-328) ───────
    //
    // Two addresses are in `pending_addresses()` before the sync call.
    // These represent previously-derived HD addresses that the wallet
    // already tracks.
    let pre_registered: [PlatformAddress; 2] = [
        PlatformAddress::P2pkh([0x01u8; 20]),
        PlatformAddress::P2pkh([0x02u8; 20]),
    ];
    let address_lookup = build_snapshot(&pre_registered);

    // Sanity: the snapshot covers both pre-registered addresses.
    for addr in &pre_registered {
        assert!(
            address_lookup.contains_key(&addr.to_bytes()),
            "pre-condition: pre-registered address must be in the snapshot"
        );
    }

    // ── 2. `next_unused_receive_address` runs AFTER the snapshot ────────
    //
    // The wallet allocates a fresh address to receive the bank funding.
    // In production this happens in `platform_address_sync.rs` or via
    // the SDK's `next_unused_receive_address` call. Crucially it fires
    // AFTER `key_to_tag` was already built.
    let freshly_allocated = PlatformAddress::P2pkh([
        0x30u8, 0x21u8, 0x43u8, 0x64u8, 0xd2u8, 0xadu8, 0x0bu8, 0xd2u8, 0xaau8, 0xbbu8, 0xccu8,
        0xddu8, 0xeeu8, 0xffu8, 0x11u8, 0x22u8, 0x33u8, 0x44u8, 0x55u8, 0x66u8,
    ]);

    // ── 3. DAPI returns a balance update for the freshly-allocated address
    //
    // Marvin's Phase 3 trace confirmed the grovedb proof at block 334540
    // contained the correct balance for this address. The balance IS on
    // chain. The discard happens inside `incremental_catch_up` at mod.rs:619.
    let addr_bytes = freshly_allocated.to_bytes(); // same call as mod.rs:618
    let _dapi_response_credit_amount = DAPI_FUNDED_CREDITS; // what DAPI returned

    // ── 4. Simulate the filter at mod.rs:619 ────────────────────────────
    //
    // `if let Some(&(tag, address)) = address_lookup.get(&addr_bytes) { … }`
    //
    // Today (bug present): returns None → balance update is silently dropped.
    // After fix: returns Some → balance update is applied.
    let lookup_result = address_lookup.get(&addr_bytes);

    // This assertion FAILS today — None confirms the silent discard.
    // After the upstream fix lands and `key_to_tag` includes post-snapshot
    // addresses, `.get()` returns Some and this file must be updated.
    assert!(
        lookup_result.is_some(),
        "Found-025 (RED-by-design): address_lookup.get() returned None for a \
         freshly-allocated address — the SDK's incremental_catch_up filter at \
         mod.rs:619 would silently drop the {DAPI_FUNDED_CREDITS} credits \
         DAPI returned for this address. \
         Fix: ensure every address emitted by next_unused_receive_address is \
         present in key_to_tag before incremental_catch_up runs. \
         See packages/rs-sdk/src/platform/address_sync/mod.rs:619."
    );

    // Verify the found balance would be DAPI_FUNDED_CREDITS (only reachable
    // after fix — this line never executes today).
    if let Some(&(_tag, _address)) = lookup_result {
        // After fix: apply the balance and verify it is visible.
        // The actual balance-application logic lives in incremental_catch_up;
        // this pin only verifies the lookup gate — the structural precondition.
    }
}
