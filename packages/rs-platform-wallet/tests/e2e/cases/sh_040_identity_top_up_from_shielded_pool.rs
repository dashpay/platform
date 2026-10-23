//! SH-040 — Shielded pool → existing identity top-up (Type 22).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-040. Priority: P1.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::identity::{accessors::IdentityGettersV0, Identity};
use dpp::shielded::compute_shielded_identity_top_up_fee;

use crate::framework::setup;
use crate::framework::setup_with_n_identities;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::{
    wait_for, wait_for_address_balance_chain_confirmed_n, wait_for_identity_balance,
    CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const FUNDING_CREDITS: u64 = 2_220_000_000;
const SHIELD_AMOUNT: u64 = 1_120_000_000;
const TRANSFER_AMOUNT: u64 = 500_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_040_should_top_up_existing_identity_from_shielded_pool() {
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
    let receiver = setup_with_n_identities(1, 100_000_000)
        .await
        .expect("register existing recipient identity");
    assert_ne!(sender.test_wallet.id(), receiver.base.test_wallet.id());
    let identity = &receiver.identities[0];
    let identity_before = Identity::fetch(receiver.base.ctx.sdk(), identity.id)
        .await
        .expect("fetch recipient identity")
        .expect("recipient exists")
        .balance();
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
    let proven_balance = sender
        .test_wallet
        .platform_wallet()
        .shielded_identity_top_up_from_pool(
            &handle.coordinator,
            &sender.test_wallet.seed_bytes(),
            0,
            &identity.id,
            TRANSFER_AMOUNT,
            shielded_prover(),
        )
        .await
        .expect("top up existing identity from pool");
    let identity_after = wait_for_identity_balance(
        receiver.base.ctx.sdk(),
        identity.id,
        identity_before + TRANSFER_AMOUNT,
        STEP_TIMEOUT,
    )
    .await
    .expect("recipient identity must receive the top-up on chain");
    assert_eq!(identity_after, identity_before + TRANSFER_AMOUNT);
    assert_eq!(
        proven_balance,
        Some(identity_after),
        "top-up result must attest the exact identity balance"
    );
    let refreshed = wait_for(
        || async {
            let balance = receiver
                .base
                .test_wallet
                .platform_wallet()
                .identity()
                .refresh_identity_balance(&identity.id)
                .await
                .ok()?;
            (balance == identity_after).then_some(balance)
        },
        STEP_TIMEOUT,
    )
    .await
    .expect("recipient wallet must sync its own identity balance");
    assert_eq!(refreshed, identity_before + TRANSFER_AMOUNT);

    // One input and a recipient/change pair produce the two-action minimum.
    let fee = compute_shielded_identity_top_up_fee(2, sender.ctx.sdk().version())
        .expect("pool spend fee");
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
    receiver.teardown().await.expect("recipient teardown");
    teardown_sweep_shielded(&sender.test_wallet, &handle, &bank_address).await;
    sender.teardown().await.expect("sender teardown");
}
