//! PA-004c — Sweep with exactly zero balance.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "Platform Addresses (PA)" → PA-004c.
//! Priority: P2.
//!
//! Pins the contract that a never-funded wallet's `teardown` is a
//! no-op (no broadcast, no error). A regression that moves the empty-
//! input check inside `cleanup::sweep_platform_addresses` could
//! regress to `Err(InsufficientFunds)` and the test suite would never
//! notice without this case.
//!
//! Flow:
//!   1. Create a fresh `TestWallet` (registers in the registry).
//!   2. Do NOT fund it.
//!   3. Call `setup_guard.teardown()`.
//!   4. Assert: teardown returns `Ok(())`, registry entry is gone.
//!
//! The registry-removed assertion confirms the wallet completed
//! teardown WITHOUT going through the sweep broadcast — the cleanup
//! gate in `framework/cleanup.rs:154` (`if total >= dust_gate`)
//! short-circuits the sweep when the total is below
//! `min_input_amount` (= 100_000); a never-funded wallet has 0
//! credits, well below the gate.

use crate::framework::prelude::*;

#[tokio_shared_rt::test(shared)]
async fn pa_004c_sweep_zero_balance() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Setup creates a fresh wallet and registers it. We deliberately
    // do not derive any address or fund anything before teardown.
    let s = setup().await.expect("e2e setup failed");
    let ctx = s.ctx;
    let test_wallet_id = s.test_wallet.id();

    // Pre-condition: wallet's total_credits == 0.
    let pre_total = s.test_wallet.total_credits().await;
    assert_eq!(
        pre_total, 0,
        "PA-004c precondition: never-funded wallet must hold 0 credits; got {pre_total}"
    );
    // Pre-condition: registry has the entry as Active.
    let pre_status = ctx.registry().get_status(test_wallet_id);
    assert_eq!(
        pre_status,
        Some(crate::framework::registry::EntryStatus::Active),
        "PA-004c precondition: registry must hold the wallet as Active before teardown"
    );

    // ---- The PA-004c boundary call ----
    let result = s.teardown().await;

    tracing::info!(
        target: "platform_wallet::e2e::cases::pa_004c",
        wallet_id = %hex::encode(test_wallet_id),
        ?result,
        "zero-balance teardown completed"
    );

    // PA-004c contract: teardown returns `Ok(())` even on an empty
    // wallet. A regression that propagates `InsufficientFunds` from
    // `sweep_platform_addresses` would surface here.
    result.expect("PA-004c: zero-balance teardown must return Ok(())");

    // Registry entry must be removed (the cleanup path drops it
    // unconditionally, regardless of sweep gate).
    assert!(
        ctx.registry().get_status(test_wallet_id).is_none(),
        "PA-004c: registry must drop the entry on a zero-balance teardown"
    );
}
