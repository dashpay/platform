//! Found-012 — `validate_or_upgrade_proof` and `wait_for_proof` only
//! consult `standard_bip44_accounts`, missing CoinJoin / non-BIP-44
//! funding accounts.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-012).
//! Pinned status: SCAFFOLD-IGNORED — blocked on harness extension.
//!
//! ## Bug shape
//!
//! Three lookup sites:
//! - `wallet/asset_lock/sync/proof.rs:43-54` (`validate_or_upgrade_proof`)
//! - `wallet/asset_lock/sync/proof.rs:289-322` (`wait_for_proof`)
//! - `wallet/asset_lock/sync/recovery.rs:104-110` (`resolve_status_from_info`)
//!
//! All three walk `info.core_wallet.accounts.standard_bip44_accounts
//! .get(&account_index)` and bail with "Transaction not found" if the
//! BIP-44 lookup misses. But `account_index` on the tracked lock can
//! refer to a CoinJoin account, an identity account, or any non-BIP-44
//! funding source. A real CoinJoin-funded asset lock has its tx in
//! `coinjoin_accounts` (or wherever), not `standard_bip44_accounts`.
//! The wallet can't resolve the chain status, can't upgrade IS to CL,
//! and `wait_for_proof` returns "transaction not found" even though
//! the chain has the tx.
//!
//! ## Why this is a scaffold
//!
//! The e2e harness has no CoinJoin-funded test wallet setup today.
//! `setup_with_core_funded_test_wallet` lands funds on BIP-44 account 0
//! (the bug-free path); there is no
//! `setup_with_coinjoin_funded_test_wallet` companion that would
//! exercise the bug. Adding that helper requires:
//!
//! - A CoinJoin-account derivation path in `framework/wallet_factory.rs`.
//! - A bank-side helper that funds the CoinJoin account.
//! - A way to track an `AssetLockBuilder` build that draws from the
//!   CoinJoin account.
//!
//! ## Blocked on
//!
//! This scaffold is `#[ignore]`d (no assertions yet, so without the ignore it
//! would report a meaningless PASS). It is unblocked when the harness gains a
//! CoinJoin-funded test wallet helper. Once that lands, the scenario in
//! TEST_SPEC.md Found-012 wires straight into this file's body — wire the
//! harness extension and fill in the `TODO(harness)` block below.

/// Ignored scaffold for the Found-012 bug class — no assertions yet. The
/// `#[ignore]` keeps it visible in the suite summary as explicitly skipped
/// rather than a silent green; the next harness extension drops the ignore and
/// fills in the body per the TODO below.
#[test]
#[ignore = "scaffold — blocked on a CoinJoin-funded wallet harness builder; see module docs"]
fn found_012_account_type_tunnel_vision_scaffold() {
    // TODO(harness): once a CoinJoin-funded test wallet helper lands,
    // port the spec's scenario into this body:
    //
    //   1. Build a test wallet with a CoinJoin (or other non-BIP-44)
    //      account containing a confirmed UTXO.
    //   2. Build + broadcast an asset-lock funded from that account.
    //   3. Track the asset lock via the AssetLockManager — the
    //      tracked lock's `account_index` should refer to the
    //      CoinJoin account.
    //   4. Call `asset_locks().wait_for_proof(&out_point, 10s)`.
    //
    // Assertions (from TEST_SPEC.md Found-012):
    //   - `wait_for_proof` returns Ok(_) within the timeout, OR
    //   - errors with a CLEAR account-type-mismatch message — never
    //     a generic "Transaction not found in account N" message
    //     that masks the real cause.
    //
    // Today's expected behaviour (bug-pin, red until fix):
    //   - All three lookup sites walk only `standard_bip44_accounts`.
    //   - The CoinJoin-funded asset lock silently fails proof
    //     discovery with a misleading "transaction not found" error.
    //
    // After fix: walk every account collection, not just
    // `standard_bip44_accounts`; or carry the account *kind*
    // alongside `account_index` on `TrackedAssetLock`.
}
