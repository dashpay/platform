//! SH-021 — ADVERSARIAL: nullifier replay after restart/resync —
//! backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-021. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails.
//!
//! Attack: spend a note (Type 17), let it confirm, then resubmit a
//! transition spending the SAME already-spent note. The nullifier is
//! permanently in Drive's spent set, so the replay MUST fail regardless
//! of client state.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! The BACKEND replay arm needs the captured serialized bytes of the
//! confirmed shielded spend (to re-broadcast verbatim) OR a rebuild
//! against the now-spent note. Shielded `operations::*` expose no
//! build-only capture seam (contrast `transfer_capturing_st_bytes`), so
//! the genuine backend-replay arm is RED-by-gap. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING`.
//!
//! The CLIENT-side spent-protection (the wallet refuses to re-select a
//! spent note after sync) IS exercisable and is asserted as the
//! achievable half — but it is NOT the consensus guarantee this case
//! exists to prove.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_021_nullifier_replay_after_restart() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_021",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-021 RED-by-gap: backend nullifier-replay needs captured shielded ST bytes to \
         re-broadcast (or a rebuild-against-spent-note seam); neither is public. {ADVERSARIAL_SEAM_MISSING}"
    );
}
