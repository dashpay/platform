//! SH-034 — ADVERSARIAL: tampered binding signature — backend MUST
//! reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-034. Priority: P1.
//! CRITICAL-if-it-fails (value-balance binding bypass).
//!
//! Attack: flip bytes in `SerializedBundle.binding_signature` (64 bytes)
//! and broadcast. The binding signature commits to the value balance; a
//! tampered signature must fail Orchard bundle verification.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Mutating `binding_signature` needs a captured valid-build's serialized
//! bundle; shielded `operations::*` expose no build-only capture seam.
//! See `framework::shielded::ADVERSARIAL_SEAM_MISSING`.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_034_tampered_binding_signature() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_034",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-034 RED-by-gap: tampering binding_signature needs the captured serialized bundle to \
         mutate post-build; no public shielded capture seam. {ADVERSARIAL_SEAM_MISSING}"
    );
}
