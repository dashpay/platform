//! SH-033 — ADVERSARIAL: duplicate nullifier WITHIN one bundle — backend
//! MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-033. Priority: P1.
//! CRITICAL-if-it-fails (double-spend within one tx).
//!
//! Attack: one transition whose Orchard bundle spends the same note twice
//! (two actions, identical nullifier) — an intra-transition double-spend.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Constructing a bundle with a duplicated `SpendableNote` needs the raw
//! dpp bundle builder (`build_spend_bundle`, `pub(crate)`) or a build-only
//! shielded capture seam. Neither is public. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING`.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_033_duplicate_nullifier_in_bundle() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_033",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-033 RED-by-gap: building a bundle with a duplicated SpendableNote needs the raw \
         dpp bundle builder (pub(crate)) or a capture seam. {ADVERSARIAL_SEAM_MISSING}"
    );
}
