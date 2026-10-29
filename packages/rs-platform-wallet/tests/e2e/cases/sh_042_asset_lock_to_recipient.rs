//! SH-042 — Core asset lock → another wallet's shielded pool (Type 18).
//! Spec: `tests/e2e/TEST_SPEC.md` §3 → SH-042. Priority: P1.

use std::time::Duration;

use dpp::address_funds::OrchardAddress;
use dpp::balances::credits::CREDITS_PER_DUFF;
use platform_wallet::{AssetLockFunding, AssetLockStatus};

use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::signer::SeedBackedCoreSigner;
use crate::framework::tx_accounting::{assert_tracked_lock_accounting, record_fee};
use crate::framework::wait::wait_for;
use crate::framework::{setup, setup_with_core_funded_test_wallet, CORE_FUNDING_TIMEOUT};

// Same Core funding and fee headroom as SH-018.
const CORE_FUNDING_DUFFS: u64 = 1_750_000;
const ASSET_LOCK_DUFFS: u64 = 1_500_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_042_should_fund_foreign_shielded_recipient_from_asset_lock() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let sender = setup_with_core_funded_test_wallet(CORE_FUNDING_DUFFS)
        .await
        .expect("Core-funded sender setup");
    let receiver = setup().await.expect("receiver setup");
    assert_ne!(sender.test_wallet.id(), receiver.test_wallet.id());
    let wallet = sender.test_wallet.platform_wallet();
    let handle = bind_shielded(&sender.test_wallet, &[0], &sender.ctx.workdir)
        .await
        .expect("bind sender shielded account");
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
    let recipient_raw = shielded_default_address_43(&receiver.test_wallet, 0)
        .await
        .expect("receiver Orchard address");
    let recipient = OrchardAddress::from_raw_bytes(&recipient_raw).expect("valid receiver address");
    let core_before = sender.test_wallet.core_balance_confirmed();
    assert_eq!(core_before, CORE_FUNDING_DUFFS);
    // Reuse AL-002's read-only fee quote; no drain operation is registered.
    let preview = wallet
        .shielded_wallet_drain_preview(0, 0)
        .await
        .expect("pool fee quote");
    let lock_credits = ASSET_LOCK_DUFFS
        .checked_mul(CREDITS_PER_DUFF)
        .expect("lock credits fit");
    let expected_received = lock_credits
        .checked_sub(preview.pool_fee_credits)
        .expect("lock covers the pool fee");
    assert!(
        preview.pool_fee_credits > 0 && expected_received >= lock_credits / 2,
        "pool fee must stay within SH-018's post-fee bound"
    );

    let signer =
        SeedBackedCoreSigner::new(sender.test_wallet.seed_bytes(), sender.ctx.config.network);
    wallet
        .shielded_fund_from_asset_lock(
            &handle.coordinator,
            AssetLockFunding::FromWalletBalance {
                amount_duffs: ASSET_LOCK_DUFFS,
                account_index: 0,
            },
            vec![(recipient, None)],
            &signer,
            shielded_prover(),
            None,
            0,
            None,
            None,
        )
        .await
        .expect("fund foreign recipient from asset lock");

    let received = wait_for_shielded_balance(
        &receiver.test_wallet,
        &receiver_handle,
        0,
        expected_received,
        STEP_TIMEOUT,
    )
    .await
    .expect("recipient must discover its own note through sync");
    assert_eq!(
        received, expected_received,
        "receiver gets lock credits minus the exact pool fee"
    );
    assert_eq!(
        handle
            .balances(&sender.test_wallet)
            .await
            .expect("sender shielded balance")[&0],
        0,
        "foreign recipient note must not belong to sender"
    );

    // Startup state excludes consumed locks; the tracked history retains them.
    let locks = wallet.asset_locks().list_tracked_locks().await;
    assert_eq!(
        locks.len(),
        1,
        "exactly one asset lock must have funded the recipient"
    );
    let lock = &locks[0];
    assert_eq!(lock.amount, ASSET_LOCK_DUFFS);
    assert_eq!(lock.status, AssetLockStatus::Consumed);
    assert_tracked_lock_accounting(&sender.ctx.persister, sender.test_wallet.id(), lock).await;
    let record = sender
        .ctx
        .persister
        .live_record(sender.test_wallet.id(), &lock.out_point.txid)
        .expect("funding transaction record");
    let core_fee = record_fee(&record).expect("input value covers all outputs");
    assert!(
        core_fee > 0 && core_fee < CORE_FUNDING_DUFFS - ASSET_LOCK_DUFFS,
        "Core fee must fit SH-018's funding headroom: {core_fee}"
    );
    let expected_core = core_before
        .checked_sub(ASSET_LOCK_DUFFS)
        .and_then(|balance| balance.checked_sub(core_fee))
        .expect("Core input covers lock plus fee");
    let core_after = wait_for(
        || async {
            let current = sender.test_wallet.core_balance_confirmed();
            (current == expected_core).then_some(current)
        },
        CORE_FUNDING_TIMEOUT,
    )
    .await
    .expect("SPV must observe the exact post-lock Core balance");
    assert_eq!(core_before - core_after, ASSET_LOCK_DUFFS + core_fee);
    assert_eq!(
        (core_before - core_after) * CREDITS_PER_DUFF,
        received + preview.pool_fee_credits + core_fee * CREDITS_PER_DUFF,
        "Core debit must equal receiver credits plus both fees"
    );

    let bank_address = sender
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(sender.ctx.bank().network());
    teardown_sweep_shielded(&receiver.test_wallet, &receiver_handle, &bank_address).await;
    receiver.teardown().await.expect("recipient teardown");
    teardown_sweep_shielded(&sender.test_wallet, &handle, &bank_address).await;
    sender
        .teardown()
        .await
        .expect("sender Core and address teardown");
}
