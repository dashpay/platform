//! SH-023 — ADVERSARIAL: fee underpayment below `compute_minimum_shielded_fee`
//! — backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-023. Priority: P1. HIGH-if-fails.
//!
//! Attack: a spend declaring a fee BELOW
//! `compute_minimum_shielded_fee(num_actions, version)` (zero, or just
//! under the floor). Drive must enforce the same floor the client
//! derives; a divergence is itself a finding (fee-market bypass / spam).
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! `build_unshield_transition` rejects `Some(f) if f < min_fee`
//! INTERNALLY (`unshield.rs:60-65`), so the public path cannot emit an
//! under-floor transition. Reaching the backend with one needs the raw
//! build seam. See `framework::shielded::ADVERSARIAL_SEAM_MISSING`.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_023_fee_underpayment() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_023",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-023 RED-by-gap: the dpp builder enforces the min-fee floor internally; no raw seam \
         to submit an under-floor fee to the backend. {ADVERSARIAL_SEAM_MISSING}"
    );
}
