//! SH-018 — Shield from a Core L1 asset lock (Type 18).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-018.
//! Priority: P1. (Wave H + Core-L1 gate.) MAY run RED until the Core-L1
//! plumbing is complete — that is acceptable and expected; a RED here
//! pins the missing harness/asset-lock seam rather than a passing happy
//! path.
//!
//! # Flagged production gaps (do NOT fix from inside the test)
//!
//! 1. **No public `PlatformWallet::shielded_shield_from_asset_lock`
//!    wrapper.** The four other spend types have public wrappers
//!    (`platform_wallet.rs:560/604/652/721`); shield-from-asset-lock
//!    exists only as the inner free function
//!    `operations::shield_from_asset_lock` (`operations.rs:269`). This
//!    test calls the inner path directly. **Follow-up DX gap** — file a
//!    public-wrapper issue.
//! 2. **No test seam returning the one-time asset-lock private key.**
//!    `AssetLockManager::create_funded_asset_lock_proof` returns
//!    `(AssetLockProof, DerivationPath, OutPoint)` but NOT the private
//!    key bytes `shield_from_asset_lock(private_key: &[u8])` requires,
//!    and no public helper derives the key from `(seed, path)`. This is
//!    the Core-L1 asset-lock-builder seam Wave H flags as RED-acceptable.
//!
//! Because gap (2) blocks a correct call, this test pins the proof-build
//! half and surfaces the missing seam as a documented RED. Wiring the
//! private-key seam (and ideally the public wrapper) is the Core-L1
//! follow-up.

use std::time::Duration;

use crate::framework::prelude::*;

/// Core (Layer-1) duffs the test wallet is funded with so the asset-lock
/// builder's coin selection has a confirmed UTXO. Gated behind
/// `PLATFORM_WALLET_E2E_BANK_CORE_GATE`.
const TEST_WALLET_CORE_FUNDING: u64 = 100_000;
#[allow(dead_code)]
const SHIELD_AMOUNT: u64 = 50_000_000;
#[allow(dead_code)]
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_018_shield_from_asset_lock() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Core-L1 gate: this panics (RED) if SPV / Core funding isn't
    // available, which documents the gate rather than a shield-path
    // defect. Mirrors CR-003 / AL-001.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet (Core-L1 gate)");

    let pre_lock_core = s.test_wallet.core_balance_confirmed();
    assert!(
        pre_lock_core >= TEST_WALLET_CORE_FUNDING,
        "Core-L1 gate: confirmed Core balance {pre_lock_core} < {TEST_WALLET_CORE_FUNDING}"
    );

    // GAP (2): the asset-lock builder does not return the one-time
    // private key, and no public helper derives it from (seed, path), so
    // a correct `operations::shield_from_asset_lock(private_key, …)` call
    // cannot be constructed test-side. Surface the missing seam as a
    // documented RED rather than weakening the assertion or fabricating a
    // key. Wiring this seam (proof + one-time private key) is the Core-L1
    // follow-up.
    panic!(
        "SH-018 RED-by-design: Core-L1 asset-lock-builder seam incomplete — \
         no test path returns the one-time private key required by \
         operations::shield_from_asset_lock, and there is no public \
         PlatformWallet::shielded_shield_from_asset_lock wrapper. \
         Wiring the private-key seam is the Core-L1 follow-up (do NOT \
         weaken this assertion or add production code from inside the test)."
    );
}
