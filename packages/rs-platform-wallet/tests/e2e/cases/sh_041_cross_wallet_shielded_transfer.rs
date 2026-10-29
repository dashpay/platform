//! SH-041 — Shielded transfer between distinct wallets (Type 16).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-041. Priority: P1.

use std::time::Duration;

use dpp::shielded::compute_minimum_shielded_fee;

use crate::framework::setup;
use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for, wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const TRANSFER_AMOUNT: u64 = 500_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_041_should_transfer_shielded_funds_between_wallets() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();
    let sender = setup().await.expect("sender setup");
    let funding_address = sender
        .test_wallet
        .next_unused_address()
        .await
        .expect("funding address");
    sender
        .ctx
        .bank()
        .fund_address(&funding_address, FUNDING_CREDITS)
        .await
        .expect("bank funds sender");
    let funded = wait_for_address_balance_chain_confirmed_n(
        sender.ctx.sdk(),
        &funding_address,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("sender funding visible");
    assert_eq!(funded, FUNDING_CREDITS);
    sender
        .test_wallet
        .sync_balances()
        .await
        .expect("refresh funded input");
    let handle = bind_shielded(&sender.test_wallet, &[0], &sender.ctx.workdir)
        .await
        .expect("bind sender shielded account");
    let receiver = setup().await.expect("receiver setup");
    assert_ne!(sender.test_wallet.id(), receiver.test_wallet.id());
    let receiver_handle = bind_shielded(&receiver.test_wallet, &[0], &receiver.ctx.workdir)
        .await
        .expect("bind receiver shielded account");
    assert_eq!(
        receiver_handle
            .balances(&receiver.test_wallet)
            .await
            .expect("initial receiver balance")[&0],
        0
    );
    let recipient = shielded_default_address_43(&receiver.test_wallet, 0)
        .await
        .expect("receiver Orchard address");
    sender
        .test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            SHIELD_AMOUNT,
            sender.test_wallet.address_signer(),
            shielded_prover(),
        )
        .await
        .expect("fund sender shielded pool");
    let shielded_before =
        wait_for_shielded_balance(&sender.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
            .await
            .expect("sender note must sync before spending");
    assert_eq!(shielded_before, SHIELD_AMOUNT);
    sender
        .test_wallet
        .platform_wallet()
        .shielded_transfer_to(
            &handle.coordinator,
            &sender.test_wallet.seed_bytes(),
            0,
            &recipient,
            TRANSFER_AMOUNT,
            [0u8; 36],
            shielded_prover(),
        )
        .await
        .expect("transfer between distinct wallets");
    let received = wait_for_shielded_balance(
        &receiver.test_wallet,
        &receiver_handle,
        0,
        TRANSFER_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("recipient must discover its own note through sync");
    assert_eq!(
        received, TRANSFER_AMOUNT,
        "receiver gains exactly the requested amount"
    );
    // One input and a recipient/change pair produce the two-action minimum.
    let fee = compute_minimum_shielded_fee(2, sender.ctx.sdk().version()).expect("pool spend fee");
    let expected_change = shielded_before
        .checked_sub(TRANSFER_AMOUNT)
        .and_then(|value| value.checked_sub(fee))
        .expect("note covers amount plus fee");
    assert!(expected_change > 0);
    let change = wait_for(
        || async {
            handle.sync().await;
            let balances = handle.balances(&sender.test_wallet).await.ok()?;
            let current = balances.get(&0).copied()?;
            (current == expected_change).then_some(current)
        },
        STEP_TIMEOUT,
    )
    .await
    .expect("sender change note must sync to the exact balance");
    assert_eq!(
        change, expected_change,
        "sender loses exactly amount plus the protocol fee"
    );
    assert_eq!(shielded_before - change, TRANSFER_AMOUNT + fee);
    let bank_address = sender
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(sender.ctx.bank().network());
    teardown_sweep_shielded(&receiver.test_wallet, &receiver_handle, &bank_address).await;
    receiver.teardown().await.expect("recipient teardown");
    teardown_sweep_shielded(&sender.test_wallet, &handle, &bank_address).await;
    sender.teardown().await.expect("sender teardown");
}
