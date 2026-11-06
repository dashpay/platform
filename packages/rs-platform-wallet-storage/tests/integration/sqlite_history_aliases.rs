//! Canonical redelivery of a migrated historical AssetUnlock identity.
//! Both literal fixtures were emitted by d631aaca3632cc713143476e363c9a9aca038d2e.
//! The signed fixture uses the public test key [0x19; 32] and SIGHASH_ALL;
//! signature preservation is tested independently of chain/quorum validity.

use crate::common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::sighash::SighashCache;
use dashcore::{consensus, Address, Network, OutPoint, Txid};
use dpp::secp256k1::{ecdsa, ecdsa::Signature, Message, PublicKey};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::Utxo;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::sqlite::schema::blob;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use rusqlite::params;

fn migrated_history() -> (
    SqlitePersister,
    tempfile::TempDir,
    TransactionRecord,
    TransactionRecord,
) {
    migrated_fixture(include_str!("../fixtures/v019_asset_unlock_v2.txt"))
}

fn migrated_fixture(
    fixture: &str,
) -> (
    SqlitePersister,
    tempfile::TempDir,
    TransactionRecord,
    TransactionRecord,
) {
    let (persister, temp, path) = fresh_persister();
    let wallet = wid(0x79);
    ensure_wallet_meta(&persister, &wallet);
    let mut records = Vec::new();
    {
        let conn = persister.lock_conn_for_test();
        conn.execute_batch(
            "DROP TABLE core_transaction_inputs; DROP TABLE core_transaction_record_originals; \
             DELETE FROM refinery_schema_history WHERE version >= 20;",
        )
        .unwrap();
        for line in fixture.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            let bytes = hex::decode(fields[2]).unwrap();
            let mut record: TransactionRecord = blob::decode(&bytes).unwrap();
            assert_eq!(
                record.txid,
                Txid::from_slice(&hex::decode(fields[1]).unwrap()).unwrap()
            );
            // Remove only the test copy's accounting evidence: raw consumer bytes stay literal.
            if !record.transaction.input.is_empty() {
                record.input_details.clear();
                record.net_amount = 0;
            }
            conn.execute(
                "INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, ?3)",
                params![wallet.as_slice(), record.txid.as_byte_array().as_slice(), blob::encode(&record).unwrap()],
            ).unwrap();
            records.push(record);
        }
        let version: i64 = conn
            .query_row(
                "SELECT max(version) FROM refinery_schema_history",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 19);
    }
    drop(persister);
    let migrated = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    {
        let conn = migrated.lock_conn_for_test();
        let version: i64 = conn
            .query_row(
                "SELECT max(version) FROM refinery_schema_history",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 20);
    }
    let consumer = records.pop().unwrap();
    let funding = records.pop().unwrap();
    assert_ne!(funding.txid, funding.transaction.txid());
    assert_eq!(
        consumer.transaction.input[0].previous_output,
        OutPoint::new(funding.txid, 0)
    );
    (migrated, temp, funding, consumer)
}

fn canonical_redelivery(persister: &SqlitePersister, funding: &TransactionRecord, with_coin: bool) {
    let mut canonical = funding.clone();
    canonical.txid = canonical.transaction.txid();
    canonical.output_details.clear();
    let new_utxos = if with_coin {
        let output = canonical.transaction.output[0].clone();
        vec![Utxo {
            outpoint: OutPoint::new(canonical.txid, 0),
            address: Address::from_script(&output.script_pubkey, Network::Testnet).unwrap(),
            txout: output,
            height: 0,
            is_coinbase: false,
            is_confirmed: false,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        }]
    } else {
        vec![]
    };
    persister
        .store(
            wid(0x79),
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![canonical],
                    new_utxos,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
}

fn assert_consumer_bytes_and_raw_index_unchanged(
    persister: &SqlitePersister,
    original: &TransactionRecord,
) {
    let loaded = persister
        .get_core_tx_record(wid(0x79), &original.txid)
        .unwrap()
        .unwrap();
    assert_eq!(
        consensus::serialize(&loaded.transaction),
        consensus::serialize(&original.transaction)
    );
    let conn = persister.lock_conn_for_test();
    let indexed: Vec<u8> = conn
        .query_row(
            "SELECT outpoint FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2",
            params![
                wid(0x79).as_slice(),
                original.txid.as_byte_array().as_slice()
            ],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        blob::decode_outpoint(&indexed).unwrap(),
        original.transaction.input[0].previous_output
    );
}

#[test]
fn should_keep_one_history_record_after_migration_and_canonical_redelivery() {
    let (persister, _temp, funding, consumer) = migrated_history();
    canonical_redelivery(&persister, &funding, false);
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
    let conn = persister.lock_conn_for_test();
    let count: i64 = conn.query_row(
        "SELECT count(*) FROM core_transactions WHERE wallet_id = ?1 AND txid IN (?2, ?3) AND record_blob IS NOT NULL",
        params![wid(0x79).as_slice(), funding.txid.as_byte_array().as_slice(), funding.transaction.txid().as_byte_array().as_slice()],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(
        count, 1,
        "one body must not become two history records through a hash-policy upgrade"
    );
}

#[test]
fn should_repair_a_legacy_raw_prevout_when_canonical_funding_arrives() {
    let (persister, _temp, funding, consumer) = migrated_history();
    canonical_redelivery(&persister, &funding, true);
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
    let loaded = persister
        .get_core_tx_record(wid(0x79), &consumer.txid)
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.net_amount, -150_000,
        "canonical funding must reach the raw legacy prevout's accounting"
    );
    assert_eq!(loaded.input_details.len(), 1);
}

#[test]
fn should_merge_owned_details_when_alias_and_canonical_history_already_coexist() {
    let (persister, _temp, funding, consumer) = migrated_history();
    let mut partial = funding.clone();
    partial.txid = partial.transaction.txid();
    partial.output_details.clear();
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, ?3)",
            params![wid(0x79).as_slice(), partial.txid.as_byte_array().as_slice(), blob::encode(&partial).unwrap()],
        ).unwrap();
    }
    canonical_redelivery(&persister, &funding, false);
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
    let loaded = persister
        .get_core_tx_record(wid(0x79), &partial.txid)
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.output_details.len(),
        1,
        "known ownership must survive alias reconciliation"
    );
}

fn seed_record(persister: &SqlitePersister, wallet: [u8; 32], record: &TransactionRecord) {
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR REPLACE INTO core_transactions (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![wallet.as_slice(), record.txid.as_byte_array().as_slice(), record.block_info().map(|b| b.height()), record.block_info().map(|b| b.block_hash().to_byte_array().to_vec()), record.block_info().map(|b| b.timestamp()), record.block_info().is_some(), blob::encode(record).unwrap()],
    ).unwrap();
}

fn confirmed() -> key_wallet::transaction_checking::TransactionContext {
    key_wallet::transaction_checking::TransactionContext::InBlock(
        key_wallet::transaction_checking::BlockInfo::new(
            42,
            dashcore::BlockHash::from_byte_array([42; 32]),
            420,
        ),
    )
}

#[test]
fn should_preserve_active_confirmed_alias_context_on_late_mempool_adoption() {
    let (persister, _temp, mut funding, _) = migrated_history();
    funding.context = confirmed();
    funding.fee = Some(1_000);
    funding.label = "old funding".to_owned();
    seed_record(&persister, wid(0x79), &funding);
    let mut late = funding.clone();
    late.context = key_wallet::transaction_checking::TransactionContext::Mempool;
    late.fee = None;
    late.label.clear();
    canonical_redelivery(&persister, &late, false);
    let loaded = persister
        .get_core_tx_record(wid(0x79), &funding.txid)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.context, funding.context);
    assert_eq!(loaded.fee, funding.fee);
    assert_eq!(loaded.label, funding.label);
}

#[test]
fn should_keep_chainlocked_alias_context_when_canonical_snapshot_is_only_in_block() {
    let (persister, _temp, mut funding, _) = migrated_history();
    let block = *confirmed().block_info().unwrap();
    funding.context =
        key_wallet::transaction_checking::TransactionContext::InChainLockedBlock(block);
    seed_record(&persister, wid(0x79), &funding);
    let mut late = funding.clone();
    late.context = key_wallet::transaction_checking::TransactionContext::InBlock(block);
    canonical_redelivery(&persister, &late, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .context,
        funding.context
    );
}

fn snapshot(persister: &SqlitePersister, table: &str) -> Vec<Vec<rusqlite::types::Value>> {
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare(&format!("SELECT * FROM {table} ORDER BY 1, 2"))
        .unwrap();
    let columns = stmt.column_count();
    stmt.query_map([], |row| (0..columns).map(|i| row.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[test]
fn should_resolve_multiple_legacy_instances_without_mutating_archived_originals() {
    let (persister, temp, funding, consumer) = migrated_history();
    let mut second = funding.clone();
    if let Some(
        dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType(
            payload,
        ),
    ) = &mut second.transaction.special_transaction_payload
    {
        payload.quorum_sig = [0x43; 96].into();
    }
    second.txid = second.transaction.instance_hash();
    assert_ne!(second.txid, funding.txid);
    assert_eq!(second.transaction.txid(), funding.transaction.txid());
    seed_record(&persister, wid(0x79), &second);
    canonical_redelivery(&persister, &funding, false);
    let archive = snapshot(&persister, "core_transaction_record_originals");
    let rows = snapshot(&persister, "core_transactions");
    canonical_redelivery(&persister, &second, false);
    assert_eq!(
        snapshot(&persister, "core_transaction_record_originals"),
        archive
    );
    assert_eq!(snapshot(&persister, "core_transactions"), rows);
    drop(persister);
    let persister =
        SqlitePersister::open(SqlitePersisterConfig::new(temp.path().join("wallet.db"))).unwrap();
    for id in [funding.txid, second.txid, funding.transaction.txid()] {
        let record = persister
            .get_core_tx_record(wid(0x79), &id)
            .unwrap()
            .unwrap();
        assert_eq!(record.txid, funding.transaction.txid());
        assert_eq!(record.transaction, funding.transaction);
        assert_eq!(record.output_details.len(), 1);
    }
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
}

#[test]
fn should_preserve_claims_and_physical_metadata_anchors_during_history_adoption() {
    let (persister, _temp, funding, _) = migrated_history();
    let output = funding.transaction.output[0].clone();
    let utxo = Utxo {
        outpoint: OutPoint::new(funding.txid, 0),
        address: Address::from_script(&output.script_pubkey, Network::Testnet).unwrap(),
        txout: output,
        height: 0,
        is_coinbase: false,
        is_confirmed: false,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    persister
        .store(
            wid(0x79),
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE core_utxos SET spent = 1, spent_in_txid = ?1, winner_mined_height = 17",
            params![funding.txid.as_byte_array().as_slice()],
        )
        .unwrap();
    }
    let before = snapshot(&persister, "core_utxos");
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(snapshot(&persister, "core_utxos"), before);
    let conn = persister.lock_conn_for_test();
    let anchor: (Option<Vec<u8>>, Option<u32>, i64) = conn.query_row(
        "SELECT record_blob, height, finalized FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
        params![wid(0x79).as_slice(), funding.txid.as_byte_array().as_slice()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).unwrap();
    assert_eq!(anchor, (None, None, 0));
}

#[test]
fn should_not_resurrect_history_or_metadata_from_archive_after_canonical_removal() {
    let (persister, _temp, funding, _) = migrated_history();
    canonical_redelivery(&persister, &funding, false);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![
                wid(0x79).as_slice(),
                funding.transaction.txid().as_byte_array().as_slice()
            ],
        )
        .unwrap();
    }
    assert!(persister
        .get_core_tx_record(wid(0x79), &funding.txid)
        .unwrap()
        .is_none());
    assert!(persister
        .get_core_tx_record(wid(0x79), &funding.transaction.txid())
        .unwrap()
        .is_none());
    canonical_redelivery(&persister, &funding, false);
    assert!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .output_details
            .is_empty(),
        "archive proves identity, never ownership"
    );
}

#[test]
fn should_rollback_conflicting_confirmed_alias_contexts() {
    let (persister, _temp, mut funding, _) = migrated_history();
    funding.context = confirmed();
    seed_record(&persister, wid(0x79), &funding);
    let before = snapshot(&persister, "core_transactions");
    let archive = snapshot(&persister, "core_transaction_record_originals");
    let mut incoming = funding.clone();
    incoming.txid = incoming.transaction.txid();
    incoming.context = key_wallet::transaction_checking::TransactionContext::InBlock(
        key_wallet::transaction_checking::BlockInfo::new(
            43,
            dashcore::BlockHash::from_byte_array([43; 32]),
            430,
        ),
    );
    let error = persister
        .store(
            wid(0x79),
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![incoming],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    let platform_wallet::changeset::PersistenceError::Backend { source, .. } = error else {
        panic!("backend error expected")
    };
    assert!(matches!(
        source.downcast_ref::<platform_wallet_storage::WalletStorageError>(),
        Some(platform_wallet_storage::WalletStorageError::HistoryAliasContextConflict { .. })
    ));
    assert_eq!(snapshot(&persister, "core_transactions"), before);
    assert_eq!(
        snapshot(&persister, "core_transaction_record_originals"),
        archive
    );
}

#[test]
fn should_reject_corrupt_alias_evidence_in_strict_and_recovery_without_writes() {
    let (persister, temp, funding, _) = migrated_history();
    canonical_redelivery(&persister, &funding, false);
    {
        let conn = persister.lock_conn_for_test();
        let mut tampered = funding.clone();
        tampered.transaction.output[0].value += 1;
        conn.execute("UPDATE core_transaction_record_originals SET record_blob = ?1 WHERE wallet_id = ?2 AND txid = ?3", params![blob::encode(&tampered).unwrap(), wid(0x79).as_slice(), funding.txid.as_byte_array().as_slice()]).unwrap();
    }
    assert!(persister
        .get_core_tx_record(wid(0x79), &funding.txid)
        .is_err());
    assert!(persister
        .get_core_tx_record(wid(0x79), &funding.transaction.txid())
        .is_err());
    let before = snapshot(&persister, "core_transactions");
    let mut incoming = funding.clone();
    incoming.txid = incoming.transaction.txid();
    assert!(persister
        .store(
            wid(0x79),
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![incoming],
                    ..Default::default()
                }),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(snapshot(&persister, "core_transactions"), before);
    drop(persister);
    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(temp.path().join("wallet.db"))
            .with_load_policy(platform_wallet_storage::LoadPolicy::Recovery),
    )
    .unwrap();
    assert!(recovery
        .get_core_tx_record(wid(0x79), &funding.txid)
        .is_err());
    assert!(recovery
        .get_core_tx_record(wid(0x79), &funding.transaction.txid())
        .is_err());
    assert_eq!(snapshot(&recovery, "core_transactions"), before);
}

#[test]
fn should_keep_alias_projection_wallet_scoped_and_legacy_only_identity_unchanged() {
    let (persister, _temp, funding, _) = migrated_history();
    let other = wid(0x80);
    ensure_wallet_meta(&persister, &other);
    seed_record(&persister, other, &funding);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .txid,
        funding.txid
    );
    assert!(persister
        .get_core_tx_record(wid(0x79), &funding.transaction.txid())
        .unwrap()
        .is_none());
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(
        persister
            .get_core_tx_record(other, &funding.txid)
            .unwrap()
            .unwrap()
            .txid,
        funding.txid
    );
    assert!(persister
        .get_core_tx_record(other, &funding.transaction.txid())
        .unwrap()
        .is_none());
}

fn assert_signed_consumer(funding: &TransactionRecord, consumer: &TransactionRecord) {
    let pushes: Vec<_> = consumer.transaction.input[0]
        .script_sig
        .instructions()
        .map(|instruction| {
            instruction
                .unwrap()
                .push_bytes()
                .unwrap()
                .as_bytes()
                .to_vec()
        })
        .collect();
    assert_eq!(pushes.len(), 2);
    assert_eq!(*pushes[0].last().unwrap(), 1, "SIGHASH_ALL");
    let signature = Signature::from_der(&pushes[0][..pushes[0].len() - 1]).unwrap();
    let public = PublicKey::from_slice(&pushes[1]).unwrap();
    let script = Address::p2pkh(
        &dashcore::PublicKey::from_slice(&pushes[1]).unwrap(),
        Network::Testnet,
    )
    .script_pubkey();
    assert_eq!(script, funding.transaction.output[0].script_pubkey);
    let hash = SighashCache::new(&consumer.transaction)
        .legacy_signature_hash(0, &script, 1)
        .unwrap();
    ecdsa::verify(
        &signature,
        Message::from_digest(hash.to_byte_array()),
        &public,
    )
    .unwrap();
    let mut rekeyed = consumer.transaction.clone();
    rekeyed.input[0].previous_output.txid = funding.transaction.txid();
    let changed_hash = SighashCache::new(&rekeyed)
        .legacy_signature_hash(0, &script, 1)
        .unwrap();
    assert!(
        ecdsa::verify(
            &signature,
            Message::from_digest(changed_hash.to_byte_array()),
            &public
        )
        .is_err(),
        "rewriting the legacy prevout would invalidate this signature"
    );
}

#[test]
fn should_preserve_known_key_signed_consumer_through_migration_and_redelivery() {
    let fixture = include_str!("../fixtures/v019_asset_unlock_v2_signed.txt");
    let originals: Vec<TransactionRecord> = fixture
        .lines()
        .map(|line| {
            blob::decode(&hex::decode(line.split_whitespace().nth(2).unwrap()).unwrap()).unwrap()
        })
        .collect();
    assert_signed_consumer(&originals[0], &originals[1]);
    let (persister, _temp, funding, consumer) = migrated_fixture(fixture);
    canonical_redelivery(&persister, &funding, true);
    let loaded = persister
        .get_core_tx_record(wid(0x79), &consumer.txid)
        .unwrap()
        .unwrap();
    assert_signed_consumer(&funding, &loaded);
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
    assert_eq!(
        consensus::serialize(&loaded.transaction),
        consensus::serialize(&originals[1].transaction)
    );
    assert_eq!(loaded.net_amount, -150_000);
}

#[test]
fn should_preserve_active_canonical_context_while_adopting_existing_legacy_sibling() {
    let (persister, _temp, funding, _) = migrated_history();
    let mut canonical = funding.clone();
    canonical.txid = canonical.transaction.txid();
    canonical.context = confirmed();
    seed_record(&persister, wid(0x79), &canonical);
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .context,
        canonical.context
    );
}

#[test]
fn should_accept_same_block_promotion_with_newly_known_position() {
    let (persister, _temp, mut funding, _) = migrated_history();
    funding.context = confirmed();
    seed_record(&persister, wid(0x79), &funding);
    let mut canonical = funding.clone();
    canonical.txid = canonical.transaction.txid();
    canonical.context = key_wallet::transaction_checking::TransactionContext::InChainLockedBlock(
        confirmed().block_info().unwrap().with_position(7),
    );
    canonical_redelivery(&persister, &canonical, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .context,
        canonical.context
    );
}

#[test]
fn should_not_promote_later_canonical_context_from_archived_or_blobless_alias() {
    let (persister, _temp, mut funding, _) = migrated_history();
    funding.context = confirmed();
    seed_record(&persister, wid(0x79), &funding);
    canonical_redelivery(&persister, &funding, false);
    funding.context = key_wallet::transaction_checking::TransactionContext::Mempool;
    canonical_redelivery(&persister, &funding, false);
    let loaded = persister
        .get_core_tx_record(wid(0x79), &funding.txid)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.context, key_wallet::transaction_checking::TransactionContext::Mempool,
        "ordinary canonical upsert keeps its existing policy; archive/anchors never promote context");
}

#[test]
fn should_repair_existing_canonical_coin_consumers_when_alias_is_adopted_without_coin_redelivery() {
    let (persister, _temp, funding, consumer) = migrated_history();
    let output = funding.transaction.output[0].clone();
    let coin = Utxo {
        outpoint: OutPoint::new(funding.transaction.txid(), 0),
        address: Address::from_script(&output.script_pubkey, Network::Testnet).unwrap(),
        txout: output,
        height: 0,
        is_coinbase: false,
        is_confirmed: false,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    persister
        .store(
            wid(0x79),
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![coin],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &consumer.txid)
            .unwrap()
            .unwrap()
            .net_amount,
        0
    );
    let coins = snapshot(&persister, "core_utxos");
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &consumer.txid)
            .unwrap()
            .unwrap()
            .net_amount,
        -150_000
    );
    assert_eq!(snapshot(&persister, "core_utxos"), coins);
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
}

#[test]
fn should_preserve_unrepresentable_mempool_accounting_during_alias_adoption() {
    let (persister, _temp, mut funding, _) = migrated_history();
    funding.transaction.output[0].value = u64::MAX;
    funding.output_details[0].value = u64::MAX;
    funding.txid = funding.transaction.instance_hash();
    funding.net_amount = 123;
    seed_record(&persister, wid(0x79), &funding);
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .net_amount,
        123
    );
}

#[test]
fn should_not_recompute_alias_accounting_without_owned_details() {
    let (persister, _temp, mut funding, _) = migrated_history();
    funding.output_details.clear();
    funding.net_amount = 123;
    seed_record(&persister, wid(0x79), &funding);
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .net_amount,
        123
    );
}

#[test]
fn should_rollback_archiving_and_anchor_changes_when_canonical_upsert_fails_then_retry() {
    let (persister, _temp, funding, consumer) = migrated_history();
    let before = snapshot(&persister, "core_transactions");
    let archive = snapshot(&persister, "core_transaction_record_originals");
    {
        let conn = persister.lock_conn_for_test();
        conn.execute_batch("CREATE TEMP TRIGGER reject_canonical_insert BEFORE INSERT ON core_transactions BEGIN SELECT RAISE(ABORT, 'injected canonical upsert failure'); END;").unwrap();
    }
    let mut canonical = funding.clone();
    canonical.txid = canonical.transaction.txid();
    assert!(persister
        .store(
            wid(0x79),
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    records: vec![canonical],
                    ..Default::default()
                }),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(snapshot(&persister, "core_transactions"), before);
    assert_eq!(
        snapshot(&persister, "core_transaction_record_originals"),
        archive
    );
    assert_consumer_bytes_and_raw_index_unchanged(&persister, &consumer);
    persister
        .lock_conn_for_test()
        .execute_batch("DROP TRIGGER reject_canonical_insert;")
        .unwrap();
    canonical_redelivery(&persister, &funding, false);
    assert_eq!(
        persister
            .get_core_tx_record(wid(0x79), &funding.txid)
            .unwrap()
            .unwrap()
            .txid,
        funding.transaction.txid()
    );
}

#[test]
fn should_preserve_recovery_height_drift_policy_during_alias_projection() {
    let (persister, temp, funding, _) = migrated_history();
    let mut canonical = funding.clone();
    canonical.txid = canonical.transaction.txid();
    seed_record(&persister, wid(0x79), &canonical);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "UPDATE core_transactions SET height = 999 WHERE wallet_id = ?1 AND txid = ?2",
            params![
                wid(0x79).as_slice(),
                funding.txid.as_byte_array().as_slice()
            ],
        )
        .unwrap();
    }
    assert!(persister
        .get_core_tx_record(wid(0x79), &canonical.txid)
        .is_err());
    let before = snapshot(&persister, "core_transactions");
    drop(persister);
    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(temp.path().join("wallet.db"))
            .with_load_policy(platform_wallet_storage::LoadPolicy::Recovery),
    )
    .unwrap();
    let loaded = recovery
        .get_core_tx_record(wid(0x79), &funding.txid)
        .unwrap()
        .unwrap();
    assert_eq!(loaded.txid, canonical.txid);
    assert_eq!(loaded.context, funding.context);
    assert_eq!(snapshot(&recovery, "core_transactions"), before);
}
