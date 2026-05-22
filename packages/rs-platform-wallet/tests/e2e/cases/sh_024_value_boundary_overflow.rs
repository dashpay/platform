//! SH-024 — ADVERSARIAL: u64/i64 value-boundary overflow/underflow —
//! backend MUST reject safely [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-024. Priority: P1. HIGH-if-fails.
//!
//! Attack: drive `amount == u64::MAX`, `amount + fee` wrapping past
//! `u64::MAX`, and `value_balance` at `i64::MIN`/`i64::MAX`, bypassing
//! the client `checked_add` guard. The arithmetic must be checked on the
//! BACKEND (no wraparound, no validator panic, no negative-as-huge-positive).
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! `build_unshield_transition` has a `checked_add` overflow guard
//! (`unshield.rs:77-79`) and refuses to emit; feeding the raw boundary
//! `value_balance` to a captured bundle needs a build-only capture seam.
//! See `framework::shielded::ADVERSARIAL_SEAM_MISSING`. NOTE: the
//! client-side u64::MAX guard is already covered (GREEN) by SH-011.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_024_value_boundary_overflow() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_024",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-024 RED-by-gap: client checked_add guard blocks the public path; no raw seam to feed \
         a boundary value_balance to the backend validator. {ADVERSARIAL_SEAM_MISSING}"
    );
}
