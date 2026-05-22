//! SH-025 — ADVERSARIAL: forged/tampered/substituted Halo-2 proof —
//! verifier MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-025. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails (total break of shielded
//! soundness).
//!
//! Attack: build a valid transition, then flip bytes in
//! `SerializedBundle.proof` — single-bit flip, truncation, all-zeros,
//! and a proof copied from a DIFFERENT valid transition (substitution).
//! Every variant must fail Orchard proof verification.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Mutating `proof` bytes requires a captured valid-build's serialized
//! `SerializedBundle`/ST, which shielded `operations::*` never expose
//! (they build AND broadcast internally). The scaffolded `TamperingProver`
//! returns a real proving key, so on its own it produces a VALID proof —
//! genuine forgery still needs the byte-mutation-after-build seam. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING`.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_025_forged_proof() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_025",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-025 RED-by-gap: forging/tampering the proof needs the captured serialized bundle to \
         mutate proof bytes post-build; no public shielded capture seam. {ADVERSARIAL_SEAM_MISSING}"
    );
}
