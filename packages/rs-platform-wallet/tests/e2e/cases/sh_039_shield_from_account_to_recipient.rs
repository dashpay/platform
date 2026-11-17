//! SH-039 — Platform address → another wallet’s shielded pool (Type 15).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-039. Priority: P1.

use std::time::Duration;

use dash_sdk::{platform::Fetch, query_types::AddressInfo};
use dpp::shielded::compute_shielded_verification_fee;

use crate::framework::setup;
use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for, wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const TRANSFER_AMOUNT: u64 = 500_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_039_should_shield_from_account_to_recipient() {
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
        .shielded_shield_from_account_to_recipient(
            &handle.coordinator,
            0,
            0,
            &recipient,
            TRANSFER_AMOUNT,
            [0u8; 36],
            sender.test_wallet.address_signer(),
            shielded_prover(),
        )
        .await
        .expect("shield to foreign recipient");
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
    let remaining = wait_for(
        || async {
            let info = AddressInfo::fetch(sender.ctx.sdk(), funding_address)
                .await
                .ok()
                .flatten()?;
            (info.balance < funded).then_some(info.balance)
        },
        STEP_TIMEOUT,
    )
    .await
    .expect("sender's on-chain balance must decrease");
    let fee = funded
        .checked_sub(remaining)
        .and_then(|debit| debit.checked_sub(TRANSFER_AMOUNT))
        .expect("sender must pay the amount plus fee");
    // Shield storage is metered; only the proof-verification component is fixed.
    let minimum_fee = compute_shielded_verification_fee(2, sender.ctx.sdk().version())
        .expect("two-action proof verification fee");
    // SH-001 reserves 1e9 transparent credits for the fee-bearing input.
    assert!(
        fee >= minimum_fee && fee < 1_000_000_000,
        "unexpected shield fee: {fee}"
    );
    assert_eq!(
        handle
            .balances(&sender.test_wallet)
            .await
            .expect("sender shielded balance")[&0],
        0,
        "the recipient's note must not be spendable by the sender"
    );
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
