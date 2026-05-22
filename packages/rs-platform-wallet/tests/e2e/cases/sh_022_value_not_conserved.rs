//! SH-022 — ADVERSARIAL: value not conserved (outputs > inputs) —
//! backend MUST reject [INJECT].
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-022. Priority: P0
//! (consensus-critical). CRITICAL-if-it-fails (value forgery / unlimited
//! shielded-pool inflation).
//!
//! Attack: a transfer/unshield whose declared outputs exceed the spent
//! note value — minting value from nothing — by setting
//! `SerializedBundle.value_balance` inconsistent with the actual spend,
//! or passing `amount > note` to the dpp builder.
//!
//! Correct backend behavior: rejected (`ShieldedInvalidValueBalanceError`,
//! code 10822, or invalid-proof). RED if accepted.
//!
//! # PRODUCTION GAP (flagged, not fixed)
//!
//! The public dpp `build_*_transition` enforce `required > total_spent`
//! and the fee floor INTERNALLY (`unshield.rs:78-86`), so they refuse to
//! emit an out-of-input bundle. Mutating a captured valid bundle's
//! `value_balance` needs a build-only shielded capture seam, which is not
//! public. See `framework::shielded::ADVERSARIAL_SEAM_MISSING`.

#![cfg(feature = "shielded")]

use crate::framework::shielded::{adversarial_enabled, ADVERSARIAL_SEAM_MISSING};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_022_value_not_conserved() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_022",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL unset — abuse case skipped (no-op pass)"
        );
        return;
    }

    panic!(
        "SH-022 RED-by-gap: cannot construct outputs>inputs and reach the backend — the public \
         dpp builders enforce value conservation internally and there is no captured-bundle \
         value_balance-tamper seam. {ADVERSARIAL_SEAM_MISSING}"
    );
}
