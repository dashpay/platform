//! SH-012 — Sync watermark idempotency: `coordinator.sync(force)` twice
//! yields stable balances.
//!
//! **RED-by-design (Found-032)**: `shielded_shield_from_account` fails with
//! `ShieldedInsufficientBalance { available: 0, required: 2120000000 }` because
//! `sync_balances()` does not populate the local balance map for `addr_1`. Funding
//! was chain-confirmed via `wait_for_address_balance_chain_confirmed_n` (DAPI
//! chain-query path), but `sync_balances()` returns 0 new entries and never
//! refreshes the local map — `select_shield_inputs` sees `available = 0`.
//! See TEST_SPEC.md Found-032.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-012.
//! Priority: P2.
//!
//! Shields a note, forces two syncs in a row, and asserts the shielded
//! balance is identical after each (no double-append — a second append at
//! an existing position would corrupt shardtree and surface as an anchor
//! error at the next spend). The strong end-to-end check: a spend still
//! succeeds post-double-sync, and the spendable note's value survived the
//! 115-byte serialize→store→deserialize round-trip exactly.
//!
//! Expected outcome: PASS.

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
const UNSHIELD_AMOUNT: u64 = 15_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_012_sync_watermark_idempotency() {
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
        .expect(
            "Found-032 (RED-by-design): shielded_shield_from_account fails with \
             ShieldedInsufficientBalance (available=0) because sync_balances() \
             does not refresh the local balance map when the incremental DAPI delta \
             returns 0 new entries. See TEST_SPEC.md Found-032.",
        );
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("shielded balance never reached SHIELD_AMOUNT");

    // Two forced syncs in a row; balances must be byte-identical.
    handle.sync().await;
    let first = handle
        .balances(&s.test_wallet)
        .await
        .expect("balances after first forced sync");
    handle.sync().await;
    let second = handle
        .balances(&s.test_wallet)
        .await
        .expect("balances after second forced sync");
    assert_eq!(
        first, second,
        "shielded_balances must be identical after a second forced sync (no double-append); \
         first={first:?} second={second:?}"
    );
    assert_eq!(
        second.get(&0).copied(),
        Some(SHIELD_AMOUNT),
        "the note value must survive the serialize→store→deserialize round-trip exactly; \
         observed {second:?}"
    );

    // Strong end-to-end check: a spend still succeeds after the
    // double-sync (a double-append would corrupt shardtree and surface
    // here as an anchor / witness error).
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
        .expect("spend after double-sync must succeed (no shardtree corruption)");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &addr_dst,
        UNSHIELD_AMOUNT,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("post-double-sync unshield destination never observed");

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
