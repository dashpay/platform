//! SH-002 — Round-trip: shield then unshield back to a transparent
//! address (Type 15 → 17).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-002.
//! Priority: P0.
//!
//! Shields into Orchard account 0, then unshields part of it to a fresh
//! transparent address. The spend leg REQUIRES the FileBacked store
//! (the in-memory `witness()` is a hard `Err` — Found-027, pinned by
//! SH-005); the harness `bind_shielded` always uses FileBacked.
//!
//! Expected outcome: PASS against the FileBacked store.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const UNSHIELD_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_002_shield_unshield_round_trip() {
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

    // Shield leg.
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

    // Unshield leg to a fresh transparent address.
    let addr_dst = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive addr_dst");
    let addr_dst_bech32m = addr_dst.to_bech32m_string(s.ctx.bank().network());
    s.test_wallet
        .platform_wallet()
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &addr_dst_bech32m,
            UNSHIELD_AMOUNT,
            prover,
        )
        .await
        .expect("shielded_unshield_to");

    // The unshielded credits land on the transparent address.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("addr_dst unshield never observed");

    // The shielded account retains the change note (minus the shielded
    // fee). Re-scan and read the residual; assert it dropped by at least
    // the unshield amount and is strictly below the pre-unshield balance.
    handle.sync().await;
    let residual = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-unshield shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    let max_change = SHIELD_AMOUNT - UNSHIELD_AMOUNT;
    assert!(
        residual < max_change,
        "shielded change must be below SHIELD_AMOUNT - UNSHIELD_AMOUNT ({max_change}) \
         after the shielded fee; observed {residual}"
    );
    assert!(
        residual > 0,
        "shielded change note must be retained (observed {residual})"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
