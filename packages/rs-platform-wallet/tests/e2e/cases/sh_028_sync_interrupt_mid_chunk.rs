//! SH-028 — ADVERSARIAL: interrupt sync mid-chunk + resume — no
//! double-count/loss [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-028. Priority: P1. HIGH-if-fails.
//!
//! Attack: cancel `sync_notes_across` between fetch and append, then
//! resume; the append-once gate (`sync.rs:276-289`, gated on `tree_size`)
//! must prevent double-append. Post-resume, a spend must still build a
//! valid witness (proves no shardtree corruption).
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! `sync_notes_across` is `pub(super)` and fetches from the SDK
//! internally; there is no injectable sync source nor a cancellation
//! hook between fetch and store-write. The scaffolded `MockSyncSource`
//! cannot wire without a production `SyncSource` seam. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING` (the sync-source
//! variant).

#![cfg(feature = "shielded")]

use crate::framework::shielded::adversarial_enabled;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_028_sync_interrupt_mid_chunk() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_028",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-028 RED-by-gap: sync_notes_across is pub(super) with no injectable sync source or \
         mid-chunk cancellation hook; a SyncSource production seam is required to drive the \
         interrupt-and-resume attack."
    );
}
