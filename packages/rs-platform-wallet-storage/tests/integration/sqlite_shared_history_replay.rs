use std::collections::BTreeMap;
use std::sync::Arc;

use dash_spv::test_utils::{InstantLockFixture, MockNetworkManager};
use dash_spv::{ClientConfig, DashSpvClient};
use dashcore::{hashes::Hash, OutPoint};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
use dpp::prelude::AssetLockProof;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::recorded_history::ReplayEvidence;
use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
use platform_wallet::changeset::{
    CoreChangeSet, PersistenceCapabilities, PlatformWalletPersistence,
};
use platform_wallet::key_wallet_manager::test_utils::MockWallet;
use platform_wallet::test_support::core_fixtures::{
    block, outside, pay, records_of, spend_of, stored_utxo, wallet_with_receive_address,
};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet_storage::sqlite::schema::blob;
use tokio::sync::RwLock;

use crate::common::{
    authorize, checked_instant_lock, fresh_persister, registration, LoadPolicy, SqlitePersister,
    SqlitePersisterConfig,
};

#[tokio::test]
async fn should_retry_sqlite_history_with_verified_lock_before_unsafe_amount_filter() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(122)], vec![pay(100_000, &address)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let coin = OutPoint::new(funding.txid(), 0);
    let locked = authorize(
        &wallet,
        &info,
        &address,
        spend_of(&[coin], vec![pay(99_000, &address)]),
    );
    let competitor = spend_of(&[coin], vec![pay(98_000, &address)]);
    let mut alternative = info.clone();
    records.extend(records_of(&mut info, &mut wallet, &locked, TransactionContext::Mempool).await);
    records.extend(records_of(&mut alternative, &mut wallet, &competitor, block(101)).await);
    let fixture = InstantLockFixture::new(&locked).unwrap();
    let (persister, _directory, path) = fresh_persister();
    persister
        .store(
            wallet.wallet_id,
            registration(
                &wallet,
                Some(CoreChangeSet {
                    records,
                    new_utxos: vec![stored_utxo(&funding, 0, address)],
                    instant_locks_for_non_final_records: BTreeMap::from([(
                        locked.txid(),
                        fixture.instant_lock().clone(),
                    )]),
                    ..Default::default()
                }),
            ),
        )
        .unwrap();
    let mut unsafe_record = persister
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .unwrap();
    unsafe_record.input_details[0].value = u64::MAX;
    persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_transactions SET record_blob = ?3 WHERE wallet_id = ?1 AND txid = ?2",
            rusqlite::params![
                wallet.wallet_id.as_slice(),
                locked.txid().as_byte_array().as_slice(),
                blob::encode(&unsafe_record).unwrap()
            ],
        )
        .unwrap();

    assert!(
        persister
            .load_with_replay(&ReplayEvidence::default())
            .is_err(),
        "missing proof must fail before SQL effects"
    );
    assert!(persister
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());

    let spv_directory = tempfile::TempDir::new().unwrap();
    let client = DashSpvClient::new(
        ClientConfig::new(dashcore::Network::Testnet)
            .without_filters()
            .with_storage_path(spv_directory.path()),
        MockNetworkManager::new(),
        Arc::new(RwLock::new(MockWallet::new())),
        Vec::new(),
    )
    .await
    .unwrap();
    assert!(client
        .verify_instant_lock(fixture.instant_lock())
        .await
        .unwrap()
        .is_none());
    fixture.install(&client).await.unwrap();
    let proof = client
        .verify_instant_lock(fixture.instant_lock())
        .await
        .unwrap()
        .unwrap();
    let evidence = ReplayEvidence {
        instant_locks: BTreeMap::from([(proof.txid(), proof)]),
    };
    let ready = persister.load_with_replay(&evidence).unwrap();
    assert!(!ready.outcomes[&wallet.wallet_id]
        .defeated
        .contains(&locked.txid()));
    assert!(persister
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());

    let mut changed = fixture.instant_lock().clone();
    changed.signature = Default::default();
    persister
        .lock_conn_for_test()
        .execute(
            "UPDATE core_instant_locks SET islock_blob = ?3 WHERE wallet_id = ?1 AND txid = ?2",
            rusqlite::params![
                wallet.wallet_id.as_slice(),
                locked.txid().as_byte_array().as_slice(),
                blob::encode(&changed).unwrap()
            ],
        )
        .unwrap();
    assert!(
        persister.load_with_replay(&evidence).is_err(),
        "prefetched proof must be rebound to the authoritative snapshot"
    );
    drop(persister);
    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(path).with_load_policy(LoadPolicy::Recovery),
    )
    .unwrap();
    assert!(
        recovery.load_with_replay(&evidence).is_err(),
        "Recovery must not hide a missing required proof"
    );
    assert!(recovery
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());
}

#[test]
fn should_not_attest_operation_status_after_replay_cleanup() {
    let (persister, _tmp, _path) = fresh_persister();
    let capabilities = persister.persistence_capabilities();
    assert!(capabilities.contains(PersistenceCapabilities::TRACKED_ASSET_LOCKS));
    assert!(
        !capabilities.contains(PersistenceCapabilities::ASSET_LOCK_OPERATION_IDS),
        "replay cleanup has no durable operation tombstone to distinguish absence from never sent"
    );
}

#[tokio::test]
async fn should_verify_tracked_funding_before_atomic_history_settlement() {
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(125)], vec![pay(100_000, &address)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let coin = OutPoint::new(funding.txid(), 0);
    let locked = authorize(
        &wallet,
        &info,
        &address,
        spend_of(&[coin], vec![pay(99_000, &address)]),
    );
    let competitor = spend_of(&[coin], vec![pay(98_000, &address)]);
    let mut alternative = info.clone();
    records.extend(records_of(&mut info, &mut wallet, &locked, TransactionContext::Mempool).await);
    records.extend(records_of(&mut alternative, &mut wallet, &competitor, block(101)).await);
    let (instant_lock, evidence) = checked_instant_lock(&locked).await;
    let (persister, _directory, path) = fresh_persister();
    let mut changes = registration(
        &wallet,
        Some(CoreChangeSet {
            records,
            new_utxos: vec![stored_utxo(&funding, 0, address)],
            ..Default::default()
        }),
    );
    let out_point = OutPoint::new(locked.txid(), 0);
    changes.asset_locks = Some(AssetLockChangeSet {
        asset_locks: BTreeMap::from([(
            out_point,
            AssetLockEntry {
                out_point,
                transaction: locked.clone(),
                account_index: 0,
                funding_type: AssetLockFundingType::IdentityTopUp,
                identity_index: 0,
                amount_duffs: 99_000,
                status: AssetLockStatus::InstantSendLocked,
                proof: Some(AssetLockProof::Instant(InstantAssetLockProof::new(
                    instant_lock,
                    locked.clone(),
                    0,
                ))),
                operation_id: None,
            },
        )]),
        ..Default::default()
    });
    persister.store(wallet.wallet_id, changes).unwrap();
    assert!(
        persister
            .load_with_replay(&ReplayEvidence::default())
            .is_err(),
        "a tracked funding proof must be required before any SQL settlement"
    );
    assert!(persister
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());
    let loaded = persister.load_with_replay(&evidence).unwrap();
    assert!(!loaded.outcomes[&wallet.wallet_id]
        .defeated
        .contains(&locked.txid()));
    assert!(persister
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());
    drop(persister);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let loaded = reopened.load_with_replay(&evidence).unwrap();
    assert!(loaded.state.wallets[&wallet.wallet_id]
        .unused_asset_locks
        .values()
        .any(|locks| locks.contains_key(&out_point)));
    assert!(reopened
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());
}
