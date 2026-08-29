//! SH-004 — `shielded_balances` reflects a shielded note only after a
//! coordinator sync.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-004.
//! Priority: P1.
//!
//! Pins that balances read from the LOCAL store, not a live chain query:
//! before `coordinator.sync` the on-chain note is invisible; after a
//! forced sync it appears exactly. Also confirms the map is filtered to
//! this wallet's id (a second bound wallet's notes never leak in — here
//! we only assert the single-account exact-value shape).
//!
//! Expected outcome: PASS.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{bind_shielded, shielded_prover, teardown_sweep_shielded};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 1_200_000_000;
const SHIELD_AMOUNT: u64 = 50_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_004_balance_after_sync() {
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

    // BEFORE any sync: the note is on-chain but not scanned into the
    // local store, so the balance map must not yet include it.
    let pre = handle
        .balances(&s.test_wallet)
        .await
        .expect("pre-sync shielded_balances");
    assert_eq!(
        pre.get(&0).copied().unwrap_or(0),
        0,
        "shielded_balances must read from the local store: account 0 must be absent / 0 \
         before coordinator.sync; observed {:?}",
        pre.get(&0)
    );

    // Drive forced syncs until the note is scanned in, then assert the
    // exact value (not just "non-empty").
    let deadline = std::time::Instant::now() + STEP_TIMEOUT;
    let post = loop {
        handle.sync().await;
        let bal = handle
            .balances(&s.test_wallet)
            .await
            .expect("post-sync shielded_balances");
        if bal.get(&0).copied().unwrap_or(0) >= SHIELD_AMOUNT {
            break bal;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "shielded note never scanned into the local store within {STEP_TIMEOUT:?}"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    assert_eq!(
        post.get(&0).copied(),
        Some(SHIELD_AMOUNT),
        "post-sync shielded_balances must equal {{0: {SHIELD_AMOUNT}}} exactly; observed {post:?}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
