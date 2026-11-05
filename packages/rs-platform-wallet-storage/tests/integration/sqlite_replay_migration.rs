use std::collections::BTreeMap;

use dashcore::{hashes::Hash, OutPoint};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::changeset::recorded_history::ReplayEvidence;
use platform_wallet::changeset::{CoreChangeSet, PlatformWalletPersistence};
use platform_wallet::test_support::core_fixtures::{
    block, outside, pay, records_of, spend_of, stored_utxo, wallet_with_receive_address,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

use crate::common::{authorize, checked_instant_lock, fresh_persister, registration};

fn rewind_to_v020(path: &std::path::Path) {
    rusqlite::Connection::open(path)
        .unwrap()
        .execute_batch(
            "DROP INDEX IF EXISTS idx_core_utxos_funding;
         DROP INDEX IF EXISTS idx_core_utxos_claimant;
         DELETE FROM refinery_schema_history WHERE version >= 21;",
        )
        .unwrap();
}

#[tokio::test]
async fn should_rescan_plain_wallet_without_erasing_another_wallet_mined_instant_lock() {
    let (persister, _directory, path) = fresh_persister();
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(131)], vec![pay(100_000, &address)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let locked = authorize(
        &wallet,
        &info,
        &address,
        spend_of(
            &[OutPoint::new(funding.txid(), 0)],
            vec![pay(99_000, &address)],
        ),
    );
    records.extend(records_of(&mut info, &mut wallet, &locked, block(101)).await);
    let (raw_lock, evidence) = checked_instant_lock(&locked).await;
    let mut changes = registration(
        &wallet,
        Some(CoreChangeSet {
            records,
            new_utxos: vec![stored_utxo(&locked, 0, address)],
            instant_locks_for_non_final_records: BTreeMap::from([(locked.txid(), raw_lock)]),
            synced_height: Some(150),
            ..Default::default()
        }),
    );
    changes.wallet_metadata.as_mut().unwrap().birth_height = 80;
    persister.store(wallet.wallet_id, changes).unwrap();

    let (mut plain, mut plain_info, plain_address) = wallet_with_receive_address();
    let plain_tx = spend_of(&[outside(132)], vec![pay(10_000, &plain_address)]);
    let plain_records = records_of(&mut plain_info, &mut plain, &plain_tx, block(100)).await;
    let mut changes = registration(
        &plain,
        Some(CoreChangeSet {
            records: plain_records,
            new_utxos: vec![stored_utxo(&plain_tx, 0, plain_address)],
            synced_height: Some(150),
            ..Default::default()
        }),
    );
    changes.wallet_metadata.as_mut().unwrap().birth_height = 60;
    persister.store(plain.wallet_id, changes).unwrap();
    drop(persister);
    rewind_to_v020(&path);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    assert!(
        reopened
            .get_core_tx_record(plain.wallet_id, &plain_tx.txid())
            .unwrap()
            .is_none(),
        "the wallet without IS evidence must rebuild mined history by rescan"
    );
    let conn = reopened.lock_conn_for_test();
    let height = |wallet: &[u8; 32]| {
        conn.query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            [wallet.as_slice()],
            |row| row.get::<_, u32>(0),
        )
        .unwrap()
    };
    assert_eq!(height(&plain.wallet_id), 59);
    assert_eq!(height(&wallet.wallet_id), 79);
    let raw: Vec<u8> = conn
        .query_row(
            "SELECT islock_blob FROM core_instant_locks WHERE wallet_id=?1 AND txid=?2",
            rusqlite::params![
                wallet.wallet_id.as_slice(),
                locked.txid().as_byte_array().as_slice()
            ],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!raw.is_empty());
    drop(conn);
    assert!(reopened
        .load_with_replay(&ReplayEvidence::default())
        .is_err());
    assert!(reopened
        .get_core_tx_record(wallet.wallet_id, &locked.txid())
        .unwrap()
        .is_some());
    let mut loaded = reopened.load_with_replay(&evidence).unwrap();
    let mut state = loaded.state.wallets.remove(&wallet.wallet_id).unwrap();
    assert!(state
        .wallet_info
        .instant_send_locks()
        .contains(&locked.txid()));
    state.wallet_info.truncate_above(100).unwrap();
    assert!(
        state.wallet_info.accounts.standard_bip44_accounts[&0]
            .transactions()
            .contains_key(&locked.txid()),
        "the authenticated IS must survive removal of its mining block after migration"
    );
}

#[tokio::test]
async fn should_preserve_context_and_terminal_tracked_proofs_without_raw_lock_rows() {
    use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
    use dpp::prelude::AssetLockProof;
    use key_wallet::transaction_checking::TransactionContext;
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;

    for tracked in [false, true] {
        let (persister, _directory, path) = fresh_persister();
        let (mut wallet, mut info, address) = wallet_with_receive_address();
        let funding = spend_of(&[outside(133)], vec![pay(100_000, &address)]);
        let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
        let locked = authorize(
            &wallet,
            &info,
            &address,
            spend_of(
                &[OutPoint::new(funding.txid(), 0)],
                vec![pay(99_000, &address)],
            ),
        );
        let (raw, evidence) = checked_instant_lock(&locked).await;
        let context = if tracked {
            TransactionContext::Mempool
        } else {
            TransactionContext::InstantSend(raw.clone())
        };
        records.extend(records_of(&mut info, &mut wallet, &locked, context).await);
        let mut changes = registration(
            &wallet,
            Some(CoreChangeSet {
                records,
                new_utxos: vec![stored_utxo(&funding, 0, address)],
                synced_height: Some(150),
                ..Default::default()
            }),
        );
        if tracked {
            let core = changes.core.as_mut().unwrap();
            core.spent_utxos.push(core.new_utxos[0].clone());
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
                        status: AssetLockStatus::Consumed,
                        proof: Some(AssetLockProof::Instant(InstantAssetLockProof::new(
                            raw,
                            locked.clone(),
                            0,
                        ))),
                        operation_id: None,
                    },
                )]),
                ..Default::default()
            });
        }
        persister.store(wallet.wallet_id, changes).unwrap();
        drop(persister);
        rewind_to_v020(&path);
        let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        assert!(
            reopened
                .get_core_tx_record(wallet.wallet_id, &funding.txid())
                .unwrap()
                .is_some(),
            "tracked={tracked}"
        );
        if !tracked {
            assert!(
                reopened
                    .load_with_replay(&ReplayEvidence::default())
                    .is_err(),
                "a stored InstantSend context still needs proof"
            );
        }
        let no_evidence = ReplayEvidence::default();
        let loaded = reopened
            .load_with_replay(if tracked { &no_evidence } else { &evidence })
            .unwrap();
        let state = &loaded.state.wallets[&wallet.wallet_id];
        assert_eq!(
            state
                .wallet_info
                .instant_send_locks()
                .contains(&locked.txid()),
            !tracked
        );
        if tracked {
            assert!(state
                .wallet_info
                .accounts
                .all_funding_accounts()
                .iter()
                .all(|account| !account
                    .utxos
                    .contains_key(&OutPoint::new(funding.txid(), 0))));
            assert!(
                state.unused_asset_locks.values().all(BTreeMap::is_empty),
                "reading a consumed proof must not reactivate its lifecycle"
            );
        }
    }
}

#[tokio::test]
async fn should_preserve_a_chain_locked_rival_when_rescanning_an_instant_lock_wallet() {
    use key_wallet::transaction_checking::TransactionContext;
    let (persister, _directory, path) = fresh_persister();
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(134)], vec![pay(100_000, &address)]);
    let mut records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let locked = authorize(
        &wallet,
        &info,
        &address,
        spend_of(
            &[OutPoint::new(funding.txid(), 0)],
            vec![pay(99_000, &address)],
        ),
    );
    let rival = spend_of(
        &[OutPoint::new(funding.txid(), 0)],
        vec![pay(98_000, &address)],
    );
    let mut rival_info = info.clone();
    records.extend(records_of(&mut info, &mut wallet, &locked, TransactionContext::Mempool).await);
    let TransactionContext::InBlock(block) = block(102) else {
        unreachable!()
    };
    records.extend(
        records_of(
            &mut rival_info,
            &mut wallet,
            &rival,
            TransactionContext::InChainLockedBlock(block),
        )
        .await,
    );
    let (raw, _) = checked_instant_lock(&locked).await;
    persister
        .store(
            wallet.wallet_id,
            registration(
                &wallet,
                Some(CoreChangeSet {
                    records,
                    new_utxos: vec![stored_utxo(&funding, 0, address)],
                    instant_locks_for_non_final_records: [(locked.txid(), raw)].into(),
                    ..Default::default()
                }),
            ),
        )
        .unwrap();
    drop(persister);
    rewind_to_v020(&path);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    assert!(reopened
        .get_core_tx_record(wallet.wallet_id, &rival.txid())
        .unwrap()
        .is_some());
    let loaded = reopened
        .load_with_replay(&ReplayEvidence::default())
        .unwrap();
    assert!(loaded.outcomes[&wallet.wallet_id]
        .defeated
        .contains(&locked.txid()));
    assert!(!loaded.outcomes[&wallet.wallet_id]
        .defeated
        .contains(&rival.txid()));
}

#[tokio::test]
async fn should_preserve_unreadable_evidence_in_recovery_without_destructive_cleanup() {
    use crate::common::LoadPolicy;
    let (persister, _directory, path) = fresh_persister();
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(135)], vec![pay(10_000, &address)]);
    let records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    persister
        .store(
            wallet.wallet_id,
            registration(
                &wallet,
                Some(CoreChangeSet {
                    records,
                    new_utxos: vec![stored_utxo(&funding, 0, address)],
                    ..Default::default()
                }),
            ),
        )
        .unwrap();
    persister
        .lock_conn_for_test()
        .execute("UPDATE core_transactions SET record_blob=X'010203'", [])
        .unwrap();
    drop(persister);
    rewind_to_v020(&path);
    assert!(SqlitePersister::open(SqlitePersisterConfig::new(&path)).is_err());
    let reopened = SqlitePersister::open(
        SqlitePersisterConfig::new(&path)
            .with_load_policy(LoadPolicy::Recovery)
            .with_auto_backup_dir(Some(path.with_extension("recovery-backups"))),
    )
    .unwrap();
    let conn = reopened.lock_conn_for_test();
    let body: Vec<u8> = conn
        .query_row("SELECT record_blob FROM core_transactions", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(body, [1, 2, 3]);
    assert_eq!(
        conn.query_row("SELECT count(*) FROM core_utxos", [], |row| row
            .get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn should_roll_back_the_entire_rescan_on_a_storage_write_failure() {
    use crate::common::LoadPolicy;
    let (persister, _directory, path) = fresh_persister();
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(136)], vec![pay(10_000, &address)]);
    let records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    persister
        .store(
            wallet.wallet_id,
            registration(
                &wallet,
                Some(CoreChangeSet {
                    records,
                    new_utxos: vec![stored_utxo(&funding, 0, address)],
                    synced_height: Some(150),
                    ..Default::default()
                }),
            ),
        )
        .unwrap();
    drop(persister);
    rewind_to_v020(&path);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_rescan BEFORE DELETE ON core_transactions BEGIN SELECT RAISE(ABORT, 'injected migration failure'); END").unwrap();
    for policy in [LoadPolicy::Strict, LoadPolicy::Recovery] {
        let result = SqlitePersister::open(
            SqlitePersisterConfig::new(&path)
                .with_load_policy(policy)
                .with_auto_backup_dir(Some(path.with_extension(format!("{policy:?}-backups")))),
        );
        let error = match result {
            Ok(_) => panic!("injected write must fail"),
            Err(error) => error,
        };
        assert!(
            format!("{error:?}").contains("injected migration failure"),
            "{error:?}"
        );
        let state: (u32, u32, u32, u32) = conn
            .query_row(
                "SELECT (SELECT max(version) FROM refinery_schema_history),
                (SELECT count(*) FROM core_transactions), (SELECT count(*) FROM core_utxos),
                (SELECT synced_height FROM core_sync_state)",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(state, (20, 1, 1, 150));
    }
}

#[tokio::test]
async fn should_keep_v020_accounting_archive_when_v021_rescans_the_record() {
    use platform_wallet_storage::sqlite::schema::blob;
    let (persister, _directory, path) = fresh_persister();
    let (mut wallet, mut info, address) = wallet_with_receive_address();
    let funding = spend_of(&[outside(137)], vec![pay(10_000, &address)]);
    let records = records_of(&mut info, &mut wallet, &funding, block(100)).await;
    let mut legacy = records[0].clone();
    legacy.net_amount = 0;
    let original = blob::encode(&legacy).unwrap();
    persister
        .store(
            wallet.wallet_id,
            registration(
                &wallet,
                Some(CoreChangeSet {
                    records,
                    new_utxos: vec![stored_utxo(&funding, 0, address)],
                    ..Default::default()
                }),
            ),
        )
        .unwrap();
    let conn = persister.lock_conn_for_test();
    conn.execute("UPDATE core_transactions SET record_blob=?1", [&original])
        .unwrap();
    conn.execute_batch(
        "DROP TABLE core_transaction_inputs; DROP TABLE core_transaction_record_originals;
        DROP INDEX idx_core_utxos_funding; DROP INDEX idx_core_utxos_claimant;
        DELETE FROM refinery_schema_history WHERE version>=20;",
    )
    .unwrap();
    drop(conn);
    drop(persister);
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
    let conn = reopened.lock_conn_for_test();
    let archived: Vec<u8> = conn
        .query_row(
            "SELECT record_blob FROM core_transaction_record_originals",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        archived, original,
        "V021 must preserve V020's original bytes after clearing the live record"
    );
    let counts: (u32, u32, u32) = conn.query_row("SELECT (SELECT count(*) FROM core_transactions),
        (SELECT count(*) FROM core_transaction_inputs),
        (SELECT count(*) FROM sqlite_master WHERE type='index' AND name IN ('idx_core_utxos_funding','idx_core_utxos_claimant','idx_core_transaction_inputs_outpoint'))",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    assert_eq!(counts, (0, 0, 3));
}
