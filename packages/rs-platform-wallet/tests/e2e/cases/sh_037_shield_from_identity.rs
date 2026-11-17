//! SH-037 — Identity → own shielded pool (Type 21).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-037. Priority: P1.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::identity::{accessors::IdentityGettersV0, Identity};
use dpp::shielded::compute_shielded_verification_fee;

use crate::framework::setup_with_n_identities;
use crate::framework::shielded::{
    bind_shielded, shielded_prover, teardown_sweep_shielded, wait_for_shielded_balance,
};
use crate::framework::wait::wait_for_identity_balance_change;

const IDENTITY_FUNDING: u64 = 2_000_000_000;
// Enough to reclaim the receiver's note through the standard unshield sweep.
const SHIELD_AMOUNT: u64 = 500_000_000;
// Same fee headroom as SH-001 / SH-036; includes the metered identity write.
const FEE_HEADROOM: u64 = 1_000_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_037_should_shield_from_identity() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();
    let sender = setup_with_n_identities(1, IDENTITY_FUNDING)
        .await
        .expect("fund and register sender identity");
    let wallet = &sender.base.test_wallet;
    let identity = &sender.identities[0];
    let handle = bind_shielded(wallet, &[0], &sender.base.ctx.workdir)
        .await
        .expect("bind sender shielded account");
    assert_eq!(
        handle
            .balances(wallet)
            .await
            .expect("initial receiver balance")[&0],
        0
    );
    let before = Identity::fetch(sender.base.ctx.sdk(), identity.id)
        .await
        .expect("fetch funded identity")
        .expect("identity exists")
        .balance();

    let proven_balance = wallet
        .platform_wallet()
        .shielded_shield_from_identity(
            &handle.coordinator,
            0,
            &identity.id,
            SHIELD_AMOUNT,
            identity.signer.as_ref(),
            shielded_prover(),
        )
        .await
        .expect("shield from identity");

    let received = wait_for_shielded_balance(wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("receiver must discover its own note through sync");
    assert_eq!(
        received, SHIELD_AMOUNT,
        "receiver gains the exact shield amount"
    );
    let after =
        wait_for_identity_balance_change(sender.base.ctx.sdk(), identity.id, before, STEP_TIMEOUT)
            .await
            .expect("identity debit must be visible on chain");
    assert_eq!(
        after, proven_balance,
        "returned balance must match the on-chain identity"
    );
    let fee = before
        .checked_sub(after)
        .and_then(|debit| debit.checked_sub(SHIELD_AMOUNT))
        .expect("identity must pay the amount plus its fee");
    // Shield storage is metered; only the proof-verification component is fixed.
    let minimum_fee = compute_shielded_verification_fee(2, sender.base.ctx.sdk().version())
        .expect("two-action proof verification fee");
    assert!(
        fee >= minimum_fee && fee < FEE_HEADROOM,
        "unexpected identity shield fee: {fee}"
    );
    let bank_address = sender
        .base
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(sender.base.ctx.bank().network());
    teardown_sweep_shielded(wallet, &handle, &bank_address).await;
    sender
        .teardown()
        .await
        .expect("sender identity and address teardown");
}
