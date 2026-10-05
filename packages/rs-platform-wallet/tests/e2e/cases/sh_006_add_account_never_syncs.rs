//! SH-006 — `shielded_add_account` post-bind: notes for the added
//! account never sync (Found-028 pin).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-006.
//! Priority: P1. **RED-by-design.**
//!
//! `shielded_add_account` inserts the new account's `OrchardKeySet` into
//! the per-wallet keys slot but does NOT call `coordinator.register_wallet`
//! with the expanded account set, so the coordinator's IVK fan-out never
//! learns the new account's IVK and notes paid to it are never
//! discovered. The doc-comment admits this as a "caveat" — documenting a
//! silent fund-invisibility footgun does not make it not-a-bug.
//!
//! This test binds account 0, adds account 1 via `shielded_add_account`,
//! pays a private note to account 1 (self-transfer from account 0), then
//! asserts CORRECT behaviour: account 1's balance reflects the note. That
//! assertion FAILS today (the coordinator never scanned account 1's IVK),
//! which is the Found-028 finding.

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
async fn sh_006_add_account_never_syncs() {
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

    // Bind ONLY account 0, then add account 1 post-bind.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    s.test_wallet
        .platform_wallet()
        .shielded_add_account(&s.test_wallet.seed_bytes(), 1)
        .await
        .expect("shielded_add_account");

    // The per-wallet slot was updated — this part works.
    let indices = s
        .test_wallet
        .platform_wallet()
        .shielded_account_indices()
        .await;
    assert!(
        indices.contains(&1),
        "shielded_account_indices must include the added account 1; observed {indices:?}"
    );

    // Shield into account 0, then pay a private note to account 1.
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
        .expect("shielded_transfer_to account 1");

    // CORRECT behaviour: account 1 should reflect the note. This wait
    // FAILS today (Found-028 — the coordinator never scanned account 1's
    // IVK), making the case RED-by-design.
    let acct1 =
        wait_for_shielded_balance(&s.test_wallet, &handle, 1, TRANSFER_AMOUNT, STEP_TIMEOUT)
            .await
            .expect(
                "Found-028: account 1's note was never synced — shielded_add_account does not \
                 re-register on the coordinator. This assertion is RED-by-design and pins the bug.",
            );
    assert_eq!(
        acct1, TRANSFER_AMOUNT,
        "shielded_balances[1] must equal the note value (Found-028 pin); observed {acct1}"
    );

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
