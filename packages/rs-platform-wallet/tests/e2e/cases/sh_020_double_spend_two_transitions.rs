//! SH-020 — ADVERSARIAL: double-spend the same note across two
//! transitions (Type 16/17) — backend MUST reject the second [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-020. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: build two distinct, individually-valid spends of the SAME
//! shielded note (same nullifier) and broadcast both. The wallet's
//! `reserve_unspent_notes` prevents two LOCAL spends from picking the
//! same note — a client convenience, not the consensus guarantee — so
//! the attack BYPASSES it by building the second transition directly
//! against the same `SpendableNote`.
//!
//! Correct backend behavior: exactly ONE accepted; the second rejected
//! with a nullifier-already-spent consensus error (`NullifierAlreadySpentError`,
//! code 40901). RED if both accepted (double-spend — CRITICAL fund
//! forgery), neither accepted, or the balance is wrong.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Reaching Drive with a SECOND transition built against an
//! already-reserved/spent note requires the wallet's private
//! `extract_spends_and_anchor` + `reserve_unspent_notes`-bypass build
//! seam, or a captured-bytes replay seam. Neither is public — shielded
//! `operations::*` build AND broadcast internally and expose no
//! build-only capture (contrast transparent `transfer_capturing_st_bytes`).
//! See `framework::shielded::ADVERSARIAL_SEAM_MISSING`. This case is
//! RED-by-gap until a build-only shielded capture seam exists.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{
    adversarial_enabled, build_against_note, ADVERSARIAL_SEAM_MISSING,
};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_020_double_spend_two_transitions() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_020",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    // The attack needs to build a second spend against the same note
    // WITHOUT the local reservation. That seam is not public.
    let built = build_against_note();
    assert!(
        built.is_ok(),
        "SH-020 RED-by-gap: cannot reach the backend with a second spend of the same note. {ADVERSARIAL_SEAM_MISSING}"
    );
    // Once the seam lands: broadcast both, assert the first is Ok and the
    // second fails NullifierAlreadySpentError; assert the shielded
    // balance reflects exactly ONE debit (no double-spend, no mint).
}
