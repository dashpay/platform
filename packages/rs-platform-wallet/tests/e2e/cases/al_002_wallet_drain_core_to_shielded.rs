//! AL-002 — Core→Shielded wallet drain, end to end.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Asset Lock (AL)" → AL-002.
//! Priority: P1.
//!
//! Drives the pooled wallet drain the way the FFI
//! `platform_wallet_manager_shielded_fund_from_wallet_drain` does: register
//! the UUID-scoped operation, then `shielded_fund_from_asset_lock` with
//! `AssetLockFunding::DrainWalletBalance`. The drain needs
//! `PersistenceCapabilities::WALLET_DRAIN` and an inline-committing backend;
//! the harness persister passes through what its `SqlitePersister` attests,
//! which covers both.
//!
//! 1. Fund the test wallet's Core account (one bank send).
//! 2. `shielded_wallet_drain_preview`: `Ready`, the whole funded balance is
//!    a candidate, a recipient amount is quoted.
//! 3. Status of a fresh UUID before the drain: `NotSent`.
//! 4. Register the operation and drain into the wallet's own Orchard
//!    address.
//! 5. Status: `Completed` with the lock's outpoint.
//! 6. Accounting: the lock spent every candidate duff (`amount + fee ==
//!    candidate balance`), its wallet-level records are `Internal` with
//!    `net == -(amount + fee)` live and after a reload, the shielded balance
//!    is exactly `amount × CREDITS_PER_DUFF − pool_fee`, and nothing is left
//!    to drain.
//! 7. Reload: the reopened store holds the lock with the operation UUID in
//!    `DrainCompleted`, and a second registration of the same UUID still
//!    reads `Completed` with the same outpoint.

#![cfg(feature = "shielded")]

use std::sync::Arc;
use std::time::Duration;

use dpp::address_funds::OrchardAddress;
use dpp::balances::credits::CREDITS_PER_DUFF;
use platform_wallet::wallet::asset_lock::{AssetLockOperationContext, AssetLockOperationState};
use platform_wallet::wallet::shielded::wallet_drain::WalletDrainPreviewStatus;
use platform_wallet::{AssetLockFunding, AssetLockStatus};
use rand::rngs::OsRng;
use rand::RngCore;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance,
};
use crate::framework::signer::SeedBackedCoreSigner;
use crate::framework::tx_accounting::assert_tracked_lock_accounting;

/// Core duffs the bank sends the test wallet in ONE transaction; the drain
/// locks all of it minus the L1 fee. Comfortably above Type 18's
/// ~2.13e8-credit pool fee (see SH-018).
const TEST_WALLET_CORE_FUNDING: u64 = 1_750_000;
/// BIP-44 account the drain pools from (account 0 is the funded one).
const FUNDING_ACCOUNT_INDEX: u32 = 0;
const SHIELDED_ACCOUNT: u32 = 0;
/// Single real note, no anonymity-set fillers; the preview must quote the
/// same action count the drain uses.
const DUMMY_OUTPUTS: usize = 0;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn al_002_wallet_drain_core_to_shielded() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: one bank send → one confirmed UTXO on BIP44 account 0.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet (Core-L1 gate)");
    let network = s.ctx.config.network;
    let wallet = s.test_wallet.platform_wallet();
    let wallet_id = wallet.wallet_id();

    let handle = bind_shielded(&s.test_wallet, &[SHIELDED_ACCOUNT], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let recipient_raw = shielded_default_address_43(&s.test_wallet, SHIELDED_ACCOUNT)
        .await
        .expect("shielded default address for bound account");
    let recipient = OrchardAddress::from_raw_bytes(&recipient_raw)
        .expect("valid Orchard recipient address from bound account");

    // Step 2: the read-only quote.
    let preview = wallet
        .shielded_wallet_drain_preview(FUNDING_ACCOUNT_INDEX, DUMMY_OUTPUTS)
        .await
        .expect("shielded_wallet_drain_preview");
    tracing::info!(
        target: "platform_wallet::e2e::cases::al_002",
        ?preview,
        "AL-002: drain preview"
    );
    assert_eq!(preview.status, WalletDrainPreviewStatus::Ready);
    assert!(
        preview.candidate_balance_duffs >= TEST_WALLET_CORE_FUNDING,
        "the funded balance must be a drain candidate: candidate {} < funded {}",
        preview.candidate_balance_duffs,
        TEST_WALLET_CORE_FUNDING
    );
    assert!(
        preview.estimated_recipient_credits.is_some(),
        "a Ready preview must quote the recipient amount"
    );

    // Step 3: a fresh UUID has not reached the network.
    let mut operation_id = [0u8; 16];
    OsRng.fill_bytes(&mut operation_id);
    let before = wallet
        .shielded_wallet_drain_operation_status(operation_id)
        .await
        .expect("drain status before the drain");
    assert_eq!(before.state, AssetLockOperationState::NotSent);
    assert_eq!(before.out_point, None);

    // Step 4: register, then drain (same sequence as the FFI entry point).
    let operation = Arc::new(AssetLockOperationContext::new(operation_id));
    let registration = wallet.register_wallet_drain_operation(Arc::clone(&operation));
    let core_signer = SeedBackedCoreSigner::new(s.test_wallet.seed_bytes(), network);
    wallet
        .shielded_fund_from_asset_lock(
            &handle.coordinator,
            AssetLockFunding::DrainWalletBalance {
                account_index: FUNDING_ACCOUNT_INDEX,
                minimum_lock_duffs: None,
                operation: Arc::clone(&operation),
            },
            vec![(recipient, None)],
            &core_signer,
            shielded_prover(),
            None,
            DUMMY_OUTPUTS,
            None,
            None,
        )
        .await
        .expect("shielded_fund_from_asset_lock (DrainWalletBalance)");
    drop(registration);

    // Step 5: a returned drain has committed its outcome; the status reads
    // the durable row, not a guess.
    let completed = wallet
        .shielded_wallet_drain_operation_status(operation_id)
        .await
        .expect("drain status after the drain");
    assert_eq!(completed.state, AssetLockOperationState::Completed);
    let out_point = completed
        .out_point
        .expect("a completed drain names its asset-lock outpoint");

    // Step 6 + 7: the durable lock, from a reopened copy of the store.
    let start_state = s
        .ctx
        .persister
        .reloaded_start_state()
        .expect("load reopened harness store");
    let lock = start_state
        .wallets
        .get(&wallet_id)
        .expect("reopened store must hold the AL-002 wallet")
        .unused_asset_locks
        .values()
        .find_map(|locks| locks.get(&out_point))
        .cloned()
        .expect("reopened store must hold the drained asset lock");
    assert_eq!(lock.operation_id, Some(operation_id));
    assert_eq!(lock.status, AssetLockStatus::DrainCompleted);

    // Every stored record and the reloaded row: Internal, net == -(lock + fee).
    assert_tracked_lock_accounting(&s.ctx.persister, wallet_id, &lock).await;

    // The drain spent every candidate duff: lock value plus L1 fee.
    let fee = s
        .ctx
        .persister
        .live_record(wallet_id, &out_point.txid)
        .and_then(|record| record.fee)
        .expect("the drained lock's wallet-level record carries its fee");
    assert_eq!(
        lock.amount + fee,
        preview.candidate_balance_duffs,
        "the drain must lock every candidate duff (amount {} + fee {fee})",
        lock.amount
    );

    // Production shields `lock_value − pool_fee` (consensus surplus is
    // structurally zero), and this wallet holds no other note, so the
    // shielded balance is exact.
    let expected_shielded = lock
        .amount
        .checked_mul(CREDITS_PER_DUFF)
        .and_then(|credits| credits.checked_sub(preview.pool_fee_credits))
        .expect("lock value covers the pool fee");
    let shielded = wait_for_shielded_balance(
        &s.test_wallet,
        &handle,
        SHIELDED_ACCOUNT,
        expected_shielded,
        STEP_TIMEOUT,
    )
    .await
    .expect("shielded balance never reached the drained amount");
    assert_eq!(
        shielded, expected_shielded,
        "shielded_balances[{SHIELDED_ACCOUNT}] must be lock {} × {CREDITS_PER_DUFF} − pool fee {}",
        lock.amount, preview.pool_fee_credits
    );

    let after = wallet
        .shielded_wallet_drain_preview(FUNDING_ACCOUNT_INDEX, DUMMY_OUTPUTS)
        .await
        .expect("shielded_wallet_drain_preview after the drain");
    assert_eq!(
        (after.candidate_input_count, after.candidate_balance_duffs),
        (0, 0),
        "nothing may be left to drain"
    );

    // Reusing the UUID never makes it look unsent or points elsewhere.
    let reuse = wallet
        .register_wallet_drain_operation(Arc::new(AssetLockOperationContext::new(operation_id)));
    let reread = wallet
        .shielded_wallet_drain_operation_status(operation_id)
        .await
        .expect("drain status on a re-registered UUID");
    drop(reuse);
    assert_eq!(reread.state, AssetLockOperationState::Completed);
    assert_eq!(reread.out_point, Some(out_point));

    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
