//! Found-013 — `recover_asset_lock_blocking` swallows every error and
//! returns `()` — silent recovery failure.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-013).
//! Pinned status: SCAFFOLD-IGNORED — blocked on harness extension.
//!
//! ## Bug shape
//!
//! `wallet/asset_lock/sync/recovery.rs:36-88`. The function returns
//! `()`; every failure path is a silent `return`:
//! - `wallet_id` not in manager → silent return (line 52).
//! - Lock already tracked → silent return (line 55, again line 97).
//! - Persister `store` failure → logged and discarded inside
//!   `queue_asset_lock_changeset`.
//!
//! There is no signal to the caller that recovery either ran
//! successfully or failed — the doc neither mentions success/failure
//! nor offers a query path to check whether the lock is now tracked.
//!
//! ## What the spec asks for
//!
//! Construct an `AssetLockManager` whose `wallet_id` was deliberately
//! removed from the wallet manager. Call `recover_asset_lock_blocking`.
//! The caller must have SOME way to detect the failure (either via a
//! `Result<(), _>` return type, or a follow-up `is_tracked` check
//! that reflects "no, the recovery did not land").
//!
//! ## Why this is a scaffold
//!
//! `AssetLockManager::new` is `pub(crate)` — tests/ cannot construct a
//! manager with an out-of-sync `wallet_id`. The only manager handle
//! available from the e2e harness is the one
//! `setup_with_core_funded_test_wallet` returns, and that manager's
//! `wallet_id` is consistent with its `WalletManager` registration
//! by construction. Driving the "wallet_id missing" case requires
//! either:
//!
//! 1. An e2e-test-only constructor that exposes
//!    `AssetLockManager::new` with an orphan `wallet_id`. This is the
//!    natural unblocker.
//! 2. An in-crate unit test under `src/wallet/asset_lock/sync/recovery.rs`
//!    that builds a fresh `AssetLockManager` against a
//!    `WalletManager` that doesn't know the manager's `wallet_id`.
//!    The fixture exists in shape (the proof module has a
//!    `FakeRecordStore` analogue) but adding the orphan-manager test
//!    is an in-crate, not tests/e2e/, change.
//!
//! Neither path is in scope for the asset-lock test suite — the
//! scaffold documents the gap.
//!
//! ## Blocked on
//!
//! This scaffold is `#[ignore]`d (no assertions yet, so without the ignore it
//! would report a meaningless PASS). It is unblocked when either the harness
//! gains an orphan-wallet-id AssetLockManager builder, or the upstream
//! signature changes to `Result<(), PlatformWalletError>` (the spec-suggested
//! fix). The latter inverts this test from "silent failure" to "loud error
//! reaches caller" and the assertion shape flips accordingly.

/// Ignored scaffold for the Found-013 bug class — no assertions yet. The
/// `#[ignore]` keeps it visible in the suite summary as explicitly skipped
/// rather than a silent green; the next harness extension drops the ignore and
/// fills in the body per the TODO below.
#[test]
#[ignore = "scaffold — blocked on an orphan-wallet-id AssetLockManager builder; see module docs"]
fn found_013_recover_asset_lock_silent_failure_scaffold() {
    // TODO(harness): once an orphan-wallet-id AssetLockManager
    // builder lands, port the spec's scenario into this body:
    //
    //   1. Construct an `AssetLockManager` whose `wallet_id` was
    //      deliberately removed from the underlying `WalletManager`.
    //   2. Call `recover_asset_lock_blocking(tx, amount,
    //      account_index, funding_type, identity_index, out_point,
    //      proof=None)`.
    //   3. Inspect the manager's tracked-locks list.
    //
    // Assertions (from TEST_SPEC.md Found-013):
    //   - The caller can detect the failure — either via a
    //     `Result<(), _>` return type, or a follow-up `is_tracked`
    //     check that reflects "no, the recovery did not land".
    //
    // Today's expected behaviour (bug-pin, red until fix):
    //   - The function returns `()` and the tracked-locks list is
    //     unchanged. The caller has no way to distinguish "recovery
    //     succeeded" from "wallet was missing".
    //
    // After fix (one of two acceptable shapes):
    //   - Signature changes to `Result<(), PlatformWalletError>`,
    //     callers can `?`-propagate the failure; OR
    //   - Sibling `is_tracked` accessor lands and the doc explicitly
    //     marks `recover_asset_lock_blocking` as best-effort.
}
