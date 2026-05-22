//! SH-029 — ADVERSARIAL: reorg / out-of-order blocks / rescan-from-0 —
//! balance converges, no phantom funds [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-029. Priority: P1. HIGH-if-fails.
//!
//! Attack: feed sync (a) out-of-order positions, (b) a reorg that rolls
//! back then re-appends a different set, (c) `next_start_index == 0`
//! rescan-from-0 (`sync.rs:235-241`). Balances must converge to the
//! canonical chain state; the `tree_size` gate must make rescan-from-0
//! idempotent; no rolled-back commitment retained as spendable.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Requires a scriptable mock sync source returning reordered /
//! rolled-back / from-zero note chunks. `sync_notes_across` fetches from
//! the SDK directly with no injection point. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING` (sync-source variant).

#![cfg(feature = "shielded")]

use crate::framework::shielded::adversarial_enabled;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_029_reorg_rescan() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_029",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-029 RED-by-gap: no scriptable mock sync source — sync_notes_across fetches from the \
         SDK with no injection point; a SyncSource production seam is required to script \
         reorg / out-of-order / rescan-from-0 chunks."
    );
}
