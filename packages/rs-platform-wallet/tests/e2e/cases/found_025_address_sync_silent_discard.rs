//! Found-025 — `rs-sdk` address-sync silently discards a balance update when
//! a recipient address was allocated after the `key_to_tag` snapshot.
//!
//! # Status: pin deleted — pending upstream test-hook surface
//!
//! The prior pin in this file (SHA history: `cf9b6d2ba4`) was a Found-022-style
//! fake: it built a local `HashMap<Vec<u8>, (tag, address)>` from two
//! pre-registered addresses, then asserted that `.get()` returned `Some` for a
//! third address that was never inserted. That assertion fires regardless of
//! whether the upstream defect exists — it is `std::collections::HashMap`
//! semantics, not SDK behaviour. After any genuine upstream fix the pin would
//! still panic red and falsely report regression, leaving no real coverage for
//! the actual bug — the same disease as Found-022 (the prior pin asserted on a
//! local `HashMap` the SDK never touches).
//!
//! # Why the retarget is blocked
//!
//! The upstream defect is the address-sync race in
//! [`dash_sdk::platform::address_sync::sync_address_balances`] at
//! `packages/rs-sdk/src/platform/address_sync/mod.rs:325-328` and
//! `mod.rs:451-462`: `key_to_tag` is built once from
//! `provider.pending_addresses()` at function entry and passed into
//! `incremental_catch_up` by reference, so any address that the provider emits
//! later (via `next_unused_receive_address` or sibling) is invisible to the
//! filter at `mod.rs:619`.
//!
//! Driving that function from this test crate requires an
//! [`AddressProvider`](dash_sdk::platform::address_sync::AddressProvider)
//! mock whose `pending_addresses()` grows between phases. The trait is
//! already public, so the provider mock is feasible. The blocker is the
//! `&Sdk` argument: every code path past the early-return at `mod.rs:334`
//! issues live DAPI requests with grovedb-proof verification — either
//! `run_full_tree_scan` (full mode) or `incremental_catch_up`'s
//! `RecentAddressBalanceChanges::fetch_with_metadata_and_proof` (incremental
//! mode). `Sdk::new_mock()` cannot synthesize the grovedb proof bytes the
//! verifier expects, and the testnet bank harness is not available in this
//! environment.
//!
//! To pin Found-025 deterministically the upstream `rs-sdk` crate needs one of:
//!
//! 1. An injectable transport seam on `sync_address_balances` so a test can
//!    return canned `RecentAddressBalanceChanges` / `CompactedAddressBalanceChanges`
//!    payloads without grovedb verification (e.g. a `#[cfg(test)]`
//!    `sync_address_balances_with_transport` variant).
//! 2. A factored-out inner function that takes a pre-built `key_to_tag` and
//!    a list of `(addr_bytes, AddressFunds)` updates, producing the same
//!    filtering decision — this would localise the race observable to a
//!    pure-data assertion the test crate could drive.
//! 3. A test-only hook on `AddressProvider` that the engine consults after
//!    each phase to refresh `key_to_tag` (the fix itself), at which point
//!    the pin would assert the refresh happened.
//!
//! Each of these is a public-API change in `rs-sdk` requiring user input —
//! per Marvin's `red-by-design` retarget protocol the pin is deleted rather
//! than landed broken a second time.
//!
//! # When the upstream landing happens
//!
//! Re-implement this pin as an integration test driving `sync_address_balances`
//! with a `GrowingAddressProvider` whose `pending_addresses()` returns the
//! base set on the first poll and an extended set on subsequent polls. The
//! assertion is `result.found.contains_key(&(third_tag, third_address))` after
//! the function returns. Bug-present: `false`. Bug-fixed: `true`.
//!
//! See `TEST_SPEC.md` §3 Found-025 for the full scenario and assertion shape.

// Intentionally empty — no `#[test]` until the upstream test-hook surface
// lands. Tracked in TEST_SPEC.md Found-025 as `red-by-design — pending
// upstream test-hook surface`.
