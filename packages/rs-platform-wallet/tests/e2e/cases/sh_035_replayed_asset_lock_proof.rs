//! SH-035 — ADVERSARIAL: replayed Type-18 asset-lock proof — backend
//! MUST reject (single-use) [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-035. Priority: P1 (Core-L1
//! gated). CRITICAL-if-it-fails (double-shield from one L1 lock = value
//! forgery).
//!
//! Attack: shield-from-asset-lock (Type 18) with a valid `AssetLockProof`,
//! then resubmit the SAME proof in a second Type-18 transition. An
//! asset-lock outpoint is single-use; the second must fail
//! (already-used / outpoint-spent consensus error).
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Two gaps stack here: (1) the SH-018 Core-L1 seam — no test path
//! returns the one-time asset-lock private key required by
//! `operations::shield_from_asset_lock`, and there is no public
//! `shielded_shield_from_asset_lock` wrapper; (2) the
//! `reuse_asset_lock_proof` capture/replay seam. Both are needed before
//! this abuse case can reach the backend. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING` + the SH-018 case docs.

#![cfg(feature = "shielded")]

use crate::framework::shielded::adversarial_enabled;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_035_replayed_asset_lock_proof() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_035",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-035 RED-by-gap: stacks the SH-018 Core-L1 private-key gap (no test seam returns the \
         one-time asset-lock key, no public shielded_shield_from_asset_lock wrapper) with the \
         asset-lock-proof reuse seam. Both must land before this can reach the backend."
    );
}
