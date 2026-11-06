use std::collections::BTreeMap;
use std::sync::Arc;

use dash_spv::test_utils::{InstantLockFixture, MockNetworkManager};
use dash_spv::{ClientConfig, DashSpvClient};
use dashcore::{Network, OutPoint};
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
    let fixture = InstantLockFixture::new(Network::Testnet, &locked).unwrap();
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
    for recorded in [true, false] {
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
        let locked_records =
            records_of(&mut info, &mut wallet, &locked, TransactionContext::Mempool).await;
        if recorded {
            records.extend(locked_records);
        }
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
        assert!(
            persister
                .get_core_tx_record(wallet.wallet_id, &locked.txid())
                .unwrap()
                .is_some()
                == recorded
        );
        let loaded = persister.load_with_replay(&evidence).unwrap();
        assert!(!loaded.outcomes[&wallet.wallet_id]
            .defeated
            .contains(&locked.txid()));
        assert!(
            persister
                .get_core_tx_record(wallet.wallet_id, &locked.txid())
                .unwrap()
                .is_some()
                == recorded
        );
        drop(persister);
        let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
        let loaded = reopened.load_with_replay(&evidence).unwrap();
        assert!(loaded.state.wallets[&wallet.wallet_id]
            .unused_asset_locks
            .values()
            .any(|locks| locks.contains_key(&out_point)));
        assert!(
            reopened
                .get_core_tx_record(wallet.wallet_id, &locked.txid())
                .unwrap()
                .is_some()
                == recorded
        );
    }
}

#[tokio::test]
async fn should_atomically_withhold_final_asset_lock_input_at_nonzero_vout_and_reopen() {
    use dashcore::transaction::special_transaction::asset_lock::AssetLockPayload;
    use dashcore::transaction::TransactionPayload;
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    for status in [AssetLockStatus::ChainLocked, AssetLockStatus::Consumed] {
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = spend_of(
            &[outside(197)],
            vec![pay(20_000, &address), pay(100_000, &address)],
        );
        let records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
        let spent = OutPoint::new(funding.txid(), 1);
        let available = OutPoint::new(funding.txid(), 0);
        let mut transaction = spend_of(
            &[spent],
            vec![dashcore::TxOut {
                value: 99_000,
                script_pubkey: dashcore::script::Builder::new()
                    .push_opcode(dashcore::opcodes::all::OP_RETURN)
                    .into_script(),
            }],
        );
        transaction.version = 3;
        transaction.special_transaction_payload = Some(TransactionPayload::AssetLockPayloadType(
            AssetLockPayload::new(vec![pay(99_000, &address)]),
        ));
        let transaction = authorize(&wallet, &info, &address, transaction);
        let out_point = OutPoint::new(transaction.txid(), 0);
        let (persister, _directory, path) = fresh_persister();
        let mut changes = registration(
            &wallet,
            Some(CoreChangeSet {
                records,
                new_utxos: vec![
                    stored_utxo(&funding, 0, address.clone()),
                    stored_utxo(&funding, 1, address),
                ],
                ..Default::default()
            }),
        );
        changes.asset_locks = Some(AssetLockChangeSet {
            asset_locks: [(
                out_point,
                AssetLockEntry {
                    out_point,
                    transaction: transaction.clone(),
                    account_index: 0,
                    funding_type: AssetLockFundingType::IdentityTopUp,
                    identity_index: 0,
                    amount_duffs: 99_000,
                    status,
                    proof: None,
                    operation_id: Some([7; 16]),
                },
            )]
            .into(),
            ..Default::default()
        });
        persister.store(wallet.wallet_id, changes).unwrap();
        let encoded = blob::encode_outpoint(&spent).unwrap();
        // Simulate the legacy host projection that left a finalized input unspent.
        persister.lock_conn_for_test().execute(
        "UPDATE core_utxos SET spent=0, spent_in_txid=NULL, winner_mined_height=NULL WHERE wallet_id=?1 AND outpoint=?2",
        rusqlite::params![wallet.wallet_id.as_slice(), &encoded],
    ).unwrap();
        persister.lock_conn_for_test().execute_batch(
        "CREATE TEMP TRIGGER reject_final_hold BEFORE UPDATE OF spent ON core_utxos WHEN NEW.spent=1 BEGIN SELECT RAISE(ABORT, 'injected final hold failure'); END;"
    ).unwrap();
        let error = persister
            .load_with_replay(&ReplayEvidence::default())
            .unwrap_err();
        assert!(format!("{error:?}").contains("injected final hold failure"));
        let read_claim = |store: &SqlitePersister| {
            store.lock_conn_for_test().query_row(
        "SELECT spent, spent_in_txid, winner_mined_height FROM core_utxos WHERE wallet_id=?1 AND outpoint=?2",
        rusqlite::params![wallet.wallet_id.as_slice(), &encoded],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<Vec<u8>>>(1)?, row.get::<_, Option<i64>>(2)?)),
    ).unwrap()
        };
        assert_eq!(read_claim(&persister), (0, None, None));
        persister
            .lock_conn_for_test()
            .execute_batch("DROP TRIGGER reject_final_hold")
            .unwrap();
        let loaded = persister
            .load_with_replay(&ReplayEvidence::default())
            .unwrap();
        let restored = &loaded.state.wallets[&wallet.wallet_id].wallet_info;
        let account = &restored.accounts.standard_bip44_accounts[&0];
        assert_eq!(restored.balance.total(), 20_000);
        assert!(account.utxos.contains_key(&available));
        assert!(!account
            .spendable_utxos(200)
            .iter()
            .any(|coin| coin.outpoint == spent));
        assert_eq!(
            read_claim(&persister),
            (1, Some(transaction.txid().to_byte_array().to_vec()), None)
        );
        drop(persister);
        let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
        let loaded = reopened
            .load_with_replay(&ReplayEvidence::default())
            .unwrap();
        assert_eq!(
            loaded.state.wallets[&wallet.wallet_id]
                .wallet_info
                .balance
                .total(),
            20_000
        );
        assert!(loaded.state.wallets[&wallet.wallet_id]
            .wallet_info
            .utxos()
            .iter()
            .all(|coin| coin.outpoint != spent));
        assert!(
            loaded.outcomes[&wallet.wallet_id]
                .withheld_asset_lock_inputs
                .is_empty(),
            "durable hold makes explicit retry idempotent"
        );
        assert_eq!(
            read_claim(&reopened),
            (1, Some(transaction.txid().to_byte_array().to_vec()), None)
        );
    }
}
