//! SH-026 — ADVERSARIAL: stale/wrong anchor — backend MUST reject
//! AnchorMismatch [INJECT] (Found-030 dynamic probe).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-026. Priority: P1. HIGH-if-fails.
//!
//! Attack: a spend whose `SerializedBundle.anchor` is a VALID-but-stale
//! earlier-checkpoint root, or random 32 bytes, while the witness paths
//! authenticate against the current root. Doubles as the Found-030
//! dynamic probe: whichever anchor depth the backend actually accepts
//! resolves the doc ambiguity between `operations.rs:601-611` ("most
//! recent checkpoint") and `file_store.rs:162-165` ("current tree state").
//!
//! Correct backend behavior: rejected (`AnchorMismatch` / "Anchor not
//! found in the recorded anchors tree"). A stale-but-in-window anchor may
//! be accepted if the protocol keeps a bounded history — pin which side
//! of Found-030 is true.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! Overriding `anchor` post-build (or passing a stale `Anchor` to the dpp
//! builder against current witnesses) needs the build-only capture seam +
//! a tree-checkpoint advancer. Neither is public. See
//! `framework::shielded::ADVERSARIAL_SEAM_MISSING`. The Found-030 doc
//! drift remains pinned statically by the spec note until this dynamic
//! probe can run.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_026_anchor_mismatch() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_026",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-026 RED-by-gap: anchor override + tree-checkpoint advancer needed to manufacture a \
         stale anchor and reach the backend; no public seam. Found-030 stays a static doc-drift \
         pin until this probe can run. {ADVERSARIAL_SEAM_MISSING}"
    );
}
