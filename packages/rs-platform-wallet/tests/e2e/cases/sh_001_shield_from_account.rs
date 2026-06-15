//! SH-001 — Shield from a platform-payment account into the Orchard
//! shielded pool (Type 15).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-001.
//! Priority: P0.
//!
//! Bank-funds one transparent platform address, binds Orchard account 0
//! on a per-test FileBacked coordinator, then shields half the balance
//! into the shielded pool and asserts the shielded balance reflects the
//! exact amount after a sync.
//!
//! Expected outcome: PASS — the shield path is fully implemented on this
//! branch (`shield` sources real on-chain nonces via
//! `fetch_inputs_with_nonce` with a `checked_add(1)` overflow guard).
//!
//! Gated behind the `e2e` cargo feature (which pulls in `shielded`); the
//! prover warm-up is ~30 s on the first SH case in the process.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Credits the bank delivers to the funding address. Sized to cover the
/// shielded amount plus the shield transition's `DeductFromInput(0)` fee
/// headroom (the wallet reserves 1e9 credits on input 0).
const FUNDING_CREDITS: u64 = 1_200_000_000;

/// Credits shielded into the pool. The note value is exactly this — the
/// fee comes off the transparent input via `DeductFromInput(0)`.
const SHIELD_AMOUNT: u64 = 50_000_000;

/// Per-step deadline for balance observations.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_001_shield_from_account() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let addr_1 = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_1");

    s.ctx
        .bank()
        .fund_address(&addr_1, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");

    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_1,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_1 funding never observed");

    // Refresh the wallet's local balance map so the shield input
    // selection sees the funded address.
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");

    // Bind Orchard account 0 on a fresh FileBacked coordinator and warm
    // the shared prover.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let prover = shielded_prover();

    // Type 15 — shield from the transparent payment account 0 into
    // Orchard account 0. `broadcast_and_wait` proves inclusion.
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            s.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("shield_from_account");

    // The note is on-chain but not scanned until sync; poll until the
    // shielded balance reaches the shielded amount exactly.
    let shielded =
        wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
            .await
            .expect("shielded balance never reached SHIELD_AMOUNT");

    assert_eq!(
        shielded, SHIELD_AMOUNT,
        "shielded_balances[0] must equal the shielded amount exactly \
         (note value = shielded amount, fee deducted from the transparent input); \
         observed {shielded}"
    );

    // Best-effort teardown sweep: drain the residual shielded balance
    // back to the bank, then the standard transparent teardown.
    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
