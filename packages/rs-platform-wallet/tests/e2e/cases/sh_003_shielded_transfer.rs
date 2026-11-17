//! SH-003 — Shielded → shielded private transfer between two accounts of
//! one wallet (Type 16).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-003.
//! Priority: P0.
//!
//! Binds Orchard accounts [0, 1] AT BIND TIME (not via
//! `shielded_add_account`, which is broken — Found-028/SH-006), shields
//! into account 0, then privately transfers to account 1's default
//! Orchard address.
//!
//! Canary for multi-subwallet sync routing: account 1 must discover its
//! note via the non-driver trial-decryption loop. If routing regresses,
//! `shielded_balances[1]` stays 0.
//!
//! Expected outcome: PASS.

use std::time::Duration;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const TRANSFER_AMOUNT: u64 = 20_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_003_shielded_transfer() {
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

    // Bind both Orchard accounts at bind time.
    let handle = bind_shielded(&s.test_wallet, &[0, 1], &s.ctx.workdir)
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
        .expect("account 0 shielded balance never reached SHIELD_AMOUNT");

    // Private transfer to account 1's default Orchard address.
    let acct1_addr = shielded_default_address_43(&s.test_wallet, 1)
        .await
        .expect("account 1 default Orchard address");
    s.test_wallet
        .platform_wallet()
        .shielded_transfer_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &acct1_addr,
            TRANSFER_AMOUNT,
            [0u8; 36],
            prover,
        )
        .await
        .expect("shielded_transfer_to");

    // Account 1 receives the private note (multi-subwallet sync routing).
    let acct1 =
        wait_for_shielded_balance(&s.test_wallet, &handle, 1, TRANSFER_AMOUNT, STEP_TIMEOUT)
            .await
            .expect("account 1 never received the private note");
    assert_eq!(
        acct1, TRANSFER_AMOUNT,
        "shielded_balances[1] must equal the transfer amount exactly; observed {acct1}"
    );

    // Sender retains the change (minus the shielded fee).
    handle.sync().await;
    let acct0 = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-transfer shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    let max_change = SHIELD_AMOUNT - TRANSFER_AMOUNT;
    assert!(
        acct0 < max_change && acct0 > 0,
        "sender change must be below SHIELD_AMOUNT - TRANSFER_AMOUNT ({max_change}) after fee \
         and strictly positive; observed {acct0}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
