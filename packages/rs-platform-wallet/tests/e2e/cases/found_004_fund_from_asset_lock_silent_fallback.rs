//! Found-004 — `transfer` / `withdraw` / `fund_from_asset_lock`
//! silently fall back to `address_index = 0` on lookup miss.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-004).
//! Pinned status: SCAFFOLD-IGNORED — see "Why this is a scaffold" below.
//!
//! ## Bug shape
//!
//! All three sites (`transfer.rs:157-167`, `withdrawal.rs:142-152`,
//! `fund_from_asset_lock.rs:130-140`) build a
//! `PlatformAddressBalanceEntry` whose `address_index` is computed via
//!
//! ```ignore
//! let address_index = account
//!     .addresses
//!     .addresses
//!     .iter()
//!     .find_map(|(&idx, info)| /* match p2pkh */)
//!     .unwrap_or(0);
//! ```
//!
//! If the address truly is not in the pool (defensive case — e.g.
//! caller passed an address that doesn't belong to the account), the
//! entry persists with `address_index = 0`, mis-attributing the
//! balance update to whichever address sits at index 0.
//!
//! ## Why this is a scaffold
//!
//! The bug at `fund_from_asset_lock.rs:130-140` is technically
//! unreachable through the public API today: the
//! `contains_platform_address` guard at line 77 rejects foreign
//! addresses BEFORE the unwrap_or(0) path runs. The bug class still
//! matters (sibling `transfer` / `withdraw` paths have looser guards;
//! a future refactor that softens the contains-check would unmask the
//! fallback), but exercising it from `tests/e2e/` requires either:
//!
//! 1. Crate-internal access to the `find_map` site — bypassing the
//!    contains_platform_address gate. The unwrap_or(0) lives in a
//!    `pub(crate)`-adjacent helper, not in the trait surface.
//! 2. A foreign-address input route through `transfer` or
//!    `withdrawal` — both of which currently expose `Explicit*` input
//!    maps that the e2e harness does not have foreign-address
//!    builders for (every test address is derived from the same test
//!    seed, so the contains-check passes).
//!
//! Neither lever exists today in the e2e framework. This file is a
//! scaffold so the next harness extension that does add one (e.g. a
//! `foreign_platform_address` helper) wires the test body in without
//! re-discovering the bug class.
//!
//! ## FAILS UNTIL
//!
//! Either:
//! - The harness gains a foreign-address builder (then this test
//!   should drive `transfer` / `withdraw` with the foreign address
//!   and pin the changeset shape from the spec's assertion list); OR
//! - The bug is fixed (then this test inverts to assert the typed
//!   `AddressNotInPool` error).

/// Placeholder test that documents the scaffold. Kept under
/// `#[ignore]` with a clear unblocker description so the next pass
/// can drop the ignore and fill in the body.
#[ignore = "Found-004 — blocked on harness extension. The bug class \
            (unwrap_or(0) in three sibling sites) is documented in \
            TEST_SPEC.md, but exercising it from tests/e2e/ today \
            requires either (a) a foreign-PlatformAddress builder in \
            the framework (no current test address is foreign to its \
            account), or (b) crate-internal access to the find_map \
            site bypassing the public-API contains-check gate. \
            Neither lever exists today. Drop this ignore once the \
            harness gains a foreign-address helper."]
#[test]
fn found_004_fund_from_asset_lock_silent_fallback_scaffold() {
    // TODO(harness): once a `foreign_platform_address` builder lands
    // in `framework/wallet_factory.rs`, port the spec's scenario into
    // this body:
    //
    //   1. Build account A with addresses addr_at_0, addr_at_1, addr_at_2.
    //   2. Construct a transfer / fund call referencing a PlatformAddress
    //      that is NOT in any of A's pools.
    //   3. Inspect the returned PlatformAddressChangeSet.
    //
    // Assertions (from TEST_SPEC.md Found-004):
    //   - Changeset must NOT contain `(address: foreign_addr,
    //     address_index: 0)` — that's a corrupted persistence row.
    //   - Either reject with a typed error before producing a
    //     changeset entry, OR omit the foreign address entirely.
    //
    // Today's expected behaviour (bug-pin, red until fix):
    //   - The entry is attributed to index 0 and written to the
    //     persister.
    //
    // After fix: log + skip the entry instead of attributing to
    // index 0; or fail the call with a typed error.
}
