//! SH-008 — Unshield insufficient-balance: typed error with exact
//! `available`/`required`.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-008.
//! Priority: P1.
//!
//! Shields a small note, then requests an unshield far above it. The
//! failure is pre-build (no proof paid) and carries the structured
//! `(available, required)` with the fee folded into `required`. A
//! follow-up satisfiable unshield must succeed, proving the reservation
//! was released by `cancel_pending`.
//!
//! Expected outcome: PASS.

use std::time::Duration;

use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

// SHIELD_AMOUNT must cover the SATISFIABLE unshield plus the shielded fee
// (~1e9, folded into the spend's requirement); the OVERDRAW stays well
// above the shielded balance so it still trips ShieldedInsufficientBalance.
const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const OVERDRAW_AMOUNT: u64 = 2_000_000_000;
const SATISFIABLE_AMOUNT: u64 = 3_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_008_unshield_insufficient_balance() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();

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
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");

    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
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
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());

    // Overdraw: far above the only note's value → typed error, no proof.
    let result = s
        .test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            OVERDRAW_AMOUNT,
            prover,
        )
        .await;
    match result {
        Err(PlatformWalletError::ShieldedInsufficientBalance {
            available,
            required,
        }) => {
            assert_eq!(
                available, SHIELD_AMOUNT,
                "available must equal the only note's value ({SHIELD_AMOUNT}); observed {available}"
            );
            assert!(
                required > OVERDRAW_AMOUNT,
                "required must fold the fee into the requirement (required > amount); \
                 required={required} amount={OVERDRAW_AMOUNT}"
            );
        }
        other => panic!(
            "expected ShieldedInsufficientBalance {{ available, required }}; observed {other:?}"
        ),
    }

    // Follow-up satisfiable unshield must succeed — proves the
    // reservation taken during the failed attempt was released.
    s.test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            SATISFIABLE_AMOUNT,
            prover,
        )
        .await
        .expect("satisfiable unshield after release must succeed");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        SATISFIABLE_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("satisfiable unshield destination never observed");

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
