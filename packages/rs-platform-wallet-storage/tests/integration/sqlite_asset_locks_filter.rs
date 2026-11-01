#![allow(clippy::field_reassign_with_default)]

//! The status-filtered asset-lock reader excludes terminal `Consumed`
//! rows so a spent one-shot lock never resurrects as actionable on
//! rehydration, while the historical row stays on disk.

use crate::common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::hashes::Hash;
use dashcore::{OutPoint, Transaction, Txid};
#[cfg(feature = "shielded")]
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
#[cfg(feature = "shielded")]
use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry, PlatformWalletPersistence};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet_storage::sqlite::error::WalletStorageError;
use platform_wallet_storage::sqlite::{
    migrations,
    schema::{asset_locks, blob},
};
#[cfg(feature = "shielded")]
use platform_wallet_storage::{sqlite::schema::core_state, LoadCtx};

#[cfg(feature = "shielded")]
use dashcore::Network;
#[cfg(feature = "shielded")]
use key_wallet::account::StandardAccountType;
#[cfg(feature = "shielded")]
use platform_wallet::{
    changeset::changeset::SweepBatch,
    changeset::{
        AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, WalletMetadataEntry,
    },
    events::EventHandler,
    test_support::funded_wallet_manager,
    wallet::{
        asset_lock::{AssetLockFunding, AssetLockOperationContext, AssetLockOperationState},
        shielded::CachedOrchardProver,
    },
    PlatformEventHandler, PlatformWalletError, PlatformWalletManager,
};
#[cfg(feature = "shielded")]
use std::sync::Arc;

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen")
}

fn entry(op: OutPoint, status: AssetLockStatus) -> AssetLockEntry {
    AssetLockEntry {
        out_point: op,
        transaction: Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        },
        account_index: 0,
        funding_type: AssetLockFundingType::IdentityTopUp,
        identity_index: 0,
        operation_id: None,
        operation_revision: 0,
        expected_operation_revision: None,
        amount_duffs: 1000,
        status,
        proof: None,
    }
}

fn op(b: u8) -> OutPoint {
    OutPoint {
        txid: Txid::from_byte_array([b; 32]),
        vout: 0,
    }
}

/// Store a mix including one terminal `Consumed`. After reopen: the
/// `Consumed` row is still on disk, is absent from the filtered
/// rehydration feed, and non-terminal rows survive.
#[test]
fn rt4_consumed_excluded_from_rehydration_feed() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA4);
    ensure_wallet_meta(&persister, &w);

    let op_built = op(0x10);
    let op_cl = op(0x11);
    let op_consumed = op(0x12);
    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks
        .insert(op_built, entry(op_built, AssetLockStatus::Built));
    cs.asset_locks
        .insert(op_cl, entry(op_cl, AssetLockStatus::ChainLocked));
    cs.asset_locks
        .insert(op_consumed, entry(op_consumed, AssetLockStatus::Consumed));
    persister
        .store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();

    // (a) the Consumed row is still physically on disk.
    let consumed_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM asset_locks WHERE wallet_id = ?1 AND status = 'consumed'",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(consumed_rows, 1, "Consumed row must persist on disk");

    // Unfiltered reader still returns the Consumed entry...
    let unfiltered =
        asset_locks::load_state(&conn, &w, &platform_wallet_storage::LoadCtx::strict()).unwrap();
    let all_ops: Vec<_> = unfiltered
        .values()
        .flat_map(|m| m.keys().copied())
        .collect();
    assert!(
        all_ops.contains(&op_consumed),
        "unfiltered load_state must still see Consumed (historical)"
    );

    // (b)+(c) the filtered rehydration feed excludes Consumed, keeps
    // the rest.
    let feed = asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .unwrap();
    drop(conn);
    let feed_ops: Vec<_> = feed.values().flat_map(|m| m.keys().copied()).collect();
    assert!(
        !feed_ops.contains(&op_consumed),
        "Consumed must NOT resurrect in the rehydration feed"
    );
    assert!(feed_ops.contains(&op_built), "Built must survive");
    assert!(feed_ops.contains(&op_cl), "ChainLocked must survive");
    assert_eq!(feed_ops.len(), 2);
}

/// An all-consumed wallet yields an empty rehydration feed, no error.
#[test]
fn a2_all_consumed_yields_empty_feed() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA5);
    ensure_wallet_meta(&persister, &w);
    let o = op(0x20);
    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks
        .insert(o, entry(o, AssetLockStatus::Consumed));
    persister
        .store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);
    let p2 = reopen(&path);
    let conn = p2.lock_conn_for_test();
    let feed = asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
        .unwrap();
    drop(conn);
    assert!(feed.is_empty(), "all-consumed wallet → empty feed");
}

/// A completed drain must remain discoverable by UUID after restart, without
/// making legacy consumed locks actionable again.
#[test]
fn should_restore_consumed_drain_operation_tombstones() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA6);
    ensure_wallet_meta(&persister, &w);
    let outpoint = op(0x30);
    let operation_id = [0xC5; 16];
    for status in [AssetLockStatus::Built, AssetLockStatus::Consumed] {
        let mut row = entry(outpoint, status);
        row.operation_id = Some(operation_id);
        persister
            .store(
                w,
                platform_wallet::changeset::PlatformWalletChangeSet {
                    asset_locks: Some(AssetLockChangeSet {
                        asset_locks: [(outpoint, row)].into_iter().collect(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        persister.flush(w).unwrap();
    }
    drop(persister);
    let reopened = reopen(&path);
    let conn = reopened.lock_conn_for_test();
    let locks =
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .unwrap();
    let restored = &locks[&0][&outpoint];
    assert_eq!(restored.operation_id, Some(operation_id));
    assert_eq!(restored.status, AssetLockStatus::Consumed);
}

#[test]
fn should_preserve_operation_id_when_legacy_snapshot_lacks_it() {
    let (persister, _tmp, _) = fresh_persister();
    let w = wid(0xA7);
    ensure_wallet_meta(&persister, &w);
    let outpoint = op(0x31);
    for id in [Some([7; 16]), None] {
        let mut row = entry(outpoint, AssetLockStatus::Broadcast);
        row.operation_id = id;
        persister
            .store(
                w,
                platform_wallet::changeset::PlatformWalletChangeSet {
                    asset_locks: Some(AssetLockChangeSet {
                        asset_locks: [(outpoint, row)].into_iter().collect(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let conn = persister.lock_conn_for_test();
    let locks =
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .unwrap();
    assert_eq!(locks[&0][&outpoint].operation_id, Some([7; 16]));
}

#[test]
fn should_migrate_legacy_blob_without_reencoding_and_reject_invalid_operation_ids() {
    use platform_wallet_storage::sqlite::migrations;
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    migrations::runner()
        .set_target(refinery::Target::Version(18))
        .run(&mut conn)
        .unwrap();
    let w = wid(0xA8);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        rusqlite::params![w.as_slice()],
    )
    .unwrap();
    let outpoint = op(0x32);
    let legacy = entry(outpoint, AssetLockStatus::Built);
    let bytes = asset_locks::encode_entry_for_test(&legacy).unwrap();
    let outpoint_bytes =
        platform_wallet_storage::sqlite::schema::blob::encode_outpoint(&outpoint).unwrap();
    conn.execute("INSERT INTO asset_locks (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob)
                  VALUES (?1, ?2, 'built', 0, 0, 1000, ?3)",
                 rusqlite::params![w.as_slice(), outpoint_bytes, bytes]).unwrap();
    migrations::run(&mut conn).unwrap();
    let (after, id): (Vec<u8>, Option<Vec<u8>>) = conn
        .query_row(
            "SELECT lifecycle_blob, operation_id FROM asset_locks",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(after, bytes, "published lifecycle blobs must not change");
    assert!(id.is_none());
    let locks =
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .unwrap();
    assert_eq!(locks[&0][&outpoint].operation_id, None);
    assert!(conn
        .execute("UPDATE asset_locks SET operation_id = ?1", [vec![1_u8; 15]])
        .is_err());
    assert!(conn
        .execute(
            "UPDATE asset_locks SET operation_id = 'abcdefghijklmnop'",
            []
        )
        .is_err());
    conn.execute("UPDATE asset_locks SET operation_id = ?1", [vec![1_u8; 16]])
        .unwrap();
    assert!(conn
        .execute("UPDATE asset_locks SET operation_id = ?1", [vec![2_u8; 16]])
        .is_err());
    assert!(conn
        .execute("UPDATE asset_locks SET operation_id = NULL", [])
        .is_err());
}

#[test]
fn should_reject_conflicting_operation_id_before_mutating_any_stored_columns() {
    fn stored_row(
        conn: &rusqlite::Connection,
        wallet_id: &[u8; 32],
    ) -> Vec<rusqlite::types::Value> {
        conn.query_row(
            "SELECT * FROM asset_locks WHERE wallet_id = ?1",
            [wallet_id.as_slice()],
            |row| {
                (0..row.as_ref().column_count())
                    .map(|index| row.get(index))
                    .collect()
            },
        )
        .unwrap()
    }

    // Exercise both the regular upsert and the consumed-row terminal guard:
    // the conflict check runs before either path touches the row.
    for status in [AssetLockStatus::Broadcast, AssetLockStatus::Consumed] {
        let (persister, _tmp, path) = fresh_persister();
        let wallet_id = wid(0xAA);
        ensure_wallet_meta(&persister, &wallet_id);
        let outpoint = op(0x34);
        let mut original = entry(outpoint, status);
        original.operation_id = Some([8; 16]);
        persister
            .store(
                wallet_id,
                platform_wallet::changeset::PlatformWalletChangeSet {
                    asset_locks: Some(AssetLockChangeSet {
                        asset_locks: [(outpoint, original)].into_iter().collect(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            )
            .unwrap();

        let mut incoming = entry(outpoint, AssetLockStatus::ChainLocked);
        incoming.operation_id = Some([9; 16]);
        incoming.account_index = 7;
        incoming.identity_index = 11;
        incoming.amount_duffs = 2_000;
        incoming.transaction.lock_time = 123;
        let changeset = AssetLockChangeSet {
            asset_locks: [(outpoint, incoming)].into_iter().collect(),
            ..Default::default()
        };
        let mut conn = persister.lock_conn_for_test();
        let before = stored_row(&conn, &wallet_id);
        let tx = conn.transaction().unwrap();
        assert!(matches!(
            asset_locks::apply(&tx, &wallet_id, &changeset),
            Err(WalletStorageError::BlobDecode {
                reason: "asset lock operation ID cannot change"
            })
        ));
        // The Rust validation must reject before SQL, rather than relying on
        // the immutable-UUID trigger or transaction rollback to preserve data.
        tx.commit().unwrap();
        assert_eq!(stored_row(&conn, &wallet_id), before);
        drop(conn);
        drop(persister);
        let reopened = reopen(&path);
        assert_eq!(
            stored_row(&reopened.lock_conn_for_test(), &wallet_id),
            before
        );
    }
}

/// A real restored manager reports durable conflict and rejects the same UUID
/// before signing, broadcasting, or mutating storage after restart.
#[cfg(feature = "shielded")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_report_conflict_and_refuse_same_uuid_drain_after_sweep_and_reopen() {
    struct NoopHandler;
    impl EventHandler for NoopHandler {}
    impl PlatformEventHandler for NoopHandler {}

    let (source, wallet_id, _, signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;
    let (persister, tmp, path) = fresh_persister();
    let snapshot = {
        let source = source.read().await;
        let wallet = source.get_wallet(&wallet_id).unwrap();
        let info = &source.get_wallet_info(&wallet_id).unwrap().core_wallet;
        PlatformWalletChangeSet {
            wallet_metadata: Some(WalletMetadataEntry {
                network: Network::Testnet,
                wallet_group_id: [0; 32],
                birth_height: 1,
            }),
            account_registrations: wallet
                .accounts
                .all_accounts()
                .into_iter()
                .map(|account| AccountRegistrationEntry {
                    account_type: account.account_type,
                    account_xpub: account.account_xpub,
                })
                .collect(),
            core: Some(CoreChangeSet {
                new_utxos: info.accounts.standard_bip44_accounts[&0]
                    .utxos
                    .values()
                    .cloned()
                    .collect(),
                ..Default::default()
            }),
            ..Default::default()
        }
    };
    persister.store(wallet_id, snapshot).unwrap();
    let operation_id = [7; 16];
    let mut lock = entry(op(0x70), AssetLockStatus::Broadcast);
    lock.out_point.txid = lock.transaction.txid();
    lock.operation_id = Some(operation_id);
    let loser = lock.out_point;
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                asset_locks: Some(AssetLockChangeSet {
                    asset_locks: [(loser, lock)].into_iter().collect(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        asset_locks::load_unconsumed(
            &persister.lock_conn_for_test(),
            &wallet_id,
            &LoadCtx::strict()
        )
        .unwrap()[&0][&loser]
            .operation_id,
        Some(operation_id)
    );
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        core_state::apply_replay_sweeps(
            &tx,
            &wallet_id,
            vec![SweepBatch {
                txids: vec![loser.txid],
                superseded_by: op(0x71).txid,
                winner_mined_height: Some(400),
                released_outpoints: vec![],
            }],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    drop(persister);
    let persister = Arc::new(reopen(&path));
    assert_eq!(
        asset_locks::load_unconsumed(
            &persister.lock_conn_for_test(),
            &wallet_id,
            &LoadCtx::strict()
        )
        .unwrap()[&0][&loser]
            .status,
        AssetLockStatus::Conflicted
    );
    let manager = PlatformWalletManager::new(
        Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .unwrap(),
        ),
        Arc::clone(&persister),
        Arc::new(NoopHandler),
    );
    manager.load_from_persistor().await.unwrap();
    manager
        .configure_shielded(tmp.path().join("shielded.db"))
        .await
        .unwrap();
    let coordinator = manager.shielded_coordinator().await.unwrap();
    let wallet = manager.get_wallet(&wallet_id).await.unwrap();
    let changes_before = persister.lock_conn_for_test().total_changes();
    let status = wallet
        .asset_locks()
        .wallet_drain_operation_status(operation_id)
        .await
        .unwrap();
    assert_eq!(status.state, AssetLockOperationState::Conflicted);
    assert_eq!(status.out_point, Some(loser));
    let drain_error = wallet
        .shielded_fund_from_asset_lock(
            &coordinator,
            AssetLockFunding::DrainWalletBalance {
                account_index: 0,
                minimum_lock_duffs: None,
                operation: Arc::new(AssetLockOperationContext::new(operation_id)),
            },
            Vec::new(),
            &signer,
            &CachedOrchardProver::new(),
            None,
            0,
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(
        persister.lock_conn_for_test().total_changes(),
        changes_before
    );
    assert_eq!(wallet.asset_locks().list_tracked_locks().await.len(), 1);
    assert!(manager.shutdown().await.all_clean());
    assert!(
        matches!(drain_error, PlatformWalletError::AssetLockConflicted(outpoint) if outpoint == loser)
    );
}

#[cfg(feature = "shielded")]
#[test]
fn should_retain_swept_operation_binding_across_restart() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet_id = wid(0xBC);
    ensure_wallet_meta(&persister, &wallet_id);
    let outpoint = op(0xBC);
    let mut lock = entry(outpoint, AssetLockStatus::Broadcast);
    lock.operation_id = Some([0xBC; 16]);
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                asset_locks: Some(AssetLockChangeSet {
                    asset_locks: [(outpoint, lock)].into_iter().collect(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        core_state::apply_replay_sweeps(
            &tx,
            &wallet_id,
            vec![SweepBatch {
                txids: vec![outpoint.txid],
                superseded_by: op(0xBD).txid,
                winner_mined_height: Some(400),
                released_outpoints: vec![],
            }],
        )
        .unwrap();
        tx.commit().unwrap();
    }
    drop(persister);
    let reopened = reopen(&path);
    let restored = asset_locks::load_unconsumed(
        &reopened.lock_conn_for_test(),
        &wallet_id,
        &LoadCtx::strict(),
    )
    .unwrap();
    let binding = restored
        .values()
        .flat_map(|locks| locks.values())
        .find(|lock| lock.operation_id == Some([0xBC; 16]));
    assert_eq!(
        binding.map(|lock| lock.out_point),
        Some(outpoint),
        "conflict cleanup must retain the original UUID binding"
    );
}

/// Conflict/recovery writes are ordered by operation generation, independently
/// of arrival order. Rollback leaves both the lifecycle and UUID binding intact.
#[cfg(feature = "shielded")]
#[test]
fn should_fence_stale_operation_snapshots_and_preserve_same_outpoint_recovery() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet_id = wid(0xBD);
    ensure_wallet_meta(&persister, &wallet_id);
    let outpoint = op(0xBD);
    let mut original = entry(outpoint, AssetLockStatus::Broadcast);
    original.operation_id = Some([0xBD; 16]);
    let upsert = |entry: AssetLockEntry| AssetLockChangeSet {
        asset_locks: [(entry.out_point, entry)].into_iter().collect(),
        ..Default::default()
    };
    let sweep = || {
        vec![SweepBatch {
            txids: vec![outpoint.txid],
            superseded_by: op(0xBE).txid,
            winner_mined_height: Some(400),
            released_outpoints: vec![],
        }]
    };
    let mut conn = persister.lock_conn_for_test();
    let tx = conn.transaction().unwrap();
    asset_locks::apply(&tx, &wallet_id, &upsert(original.clone())).unwrap();
    tx.commit().unwrap();
    // A delayed cleanup from the Built window cannot delete a dispatched UUID.
    let tx = conn.transaction().unwrap();
    asset_locks::apply(
        &tx,
        &wallet_id,
        &AssetLockChangeSet {
            removed: [outpoint].into_iter().collect(),
            ..Default::default()
        },
    )
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(
        asset_locks::load_unconsumed(&conn, &wallet_id, &LoadCtx::strict()).unwrap()[&0][&outpoint]
            .operation_id,
        Some([0xBD; 16])
    );
    let tx = conn.transaction().unwrap();
    core_state::apply_replay_sweeps(&tx, &wallet_id, sweep()).unwrap();
    tx.rollback().unwrap();
    let before = asset_locks::load_unconsumed(&conn, &wallet_id, &LoadCtx::strict()).unwrap();
    assert_eq!(before[&0][&outpoint].status, AssetLockStatus::Broadcast);
    assert_eq!(before[&0][&outpoint].operation_revision, 0);
    let tx = conn.transaction().unwrap();
    core_state::apply_replay_sweeps(&tx, &wallet_id, sweep()).unwrap();
    tx.commit().unwrap();
    let conflict = asset_locks::load_unconsumed(&conn, &wallet_id, &LoadCtx::strict()).unwrap()[&0]
        [&outpoint]
        .clone();
    assert_eq!(conflict.status, AssetLockStatus::Conflicted);
    assert_eq!(conflict.operation_revision, 1);
    assert!(conflict.proof.is_none());
    // Recovery reads cannot soften a conflicted blob into an actionable row.
    let tx = conn.transaction().unwrap();
    tx.execute(
        "UPDATE asset_locks SET status = 'broadcast' WHERE wallet_id = ?1",
        [wallet_id.as_slice()],
    )
    .unwrap();
    let changes_before = tx.total_changes();
    assert!(asset_locks::load_unconsumed(&tx, &wallet_id, &LoadCtx::recovery()).is_err());
    assert_eq!(tx.total_changes(), changes_before);
    tx.rollback().unwrap();
    let tx = conn.transaction().unwrap();
    assert!(asset_locks::apply(&tx, &wallet_id, &upsert(original)).is_err());
    tx.rollback().unwrap();
    let removal = AssetLockChangeSet {
        removed: [outpoint].into_iter().collect(),
        ..Default::default()
    };
    let tx = conn.transaction().unwrap();
    asset_locks::apply(&tx, &wallet_id, &removal).unwrap();
    tx.commit().unwrap();
    assert_eq!(
        asset_locks::load_unconsumed(&conn, &wallet_id, &LoadCtx::strict()).unwrap()[&0][&outpoint]
            .status,
        AssetLockStatus::Conflicted
    );

    let mut recovered = AssetLockEntry::from(&conflict);
    recovered.status = AssetLockStatus::RecoveredFromChain;
    recovered.proof = Some(AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 401,
        out_point: outpoint,
    }));
    // A generic snapshot carrying a proof is not authenticated reactivation.
    let tx = conn.transaction().unwrap();
    assert!(asset_locks::apply(&tx, &wallet_id, &upsert(recovered.clone())).is_err());
    tx.rollback().unwrap();
    recovered.advance_operation_revision();
    let mut wrong_outpoint = recovered.clone();
    wrong_outpoint.proof = Some(AssetLockProof::Chain(ChainAssetLockProof {
        core_chain_locked_height: 401,
        out_point: op(0xCC),
    }));
    let tx = conn.transaction().unwrap();
    assert!(asset_locks::apply(&tx, &wallet_id, &upsert(wrong_outpoint)).is_err());
    tx.rollback().unwrap();
    let tx = conn.transaction().unwrap();
    asset_locks::apply(&tx, &wallet_id, &upsert(recovered.clone())).unwrap();
    tx.commit().unwrap();
    // Delivery acknowledgement may be lost; replaying the exact CAS is safe.
    let tx = conn.transaction().unwrap();
    asset_locks::apply(&tx, &wallet_id, &upsert(recovered.clone())).unwrap();
    asset_locks::apply(&tx, &wallet_id, &removal).unwrap();
    tx.commit().unwrap();
    let tx = conn.transaction().unwrap();
    assert!(asset_locks::apply(&tx, &wallet_id, &upsert(AssetLockEntry::from(&conflict))).is_err());
    tx.rollback().unwrap();
    let mut replacement = recovered.clone();
    replacement.out_point = op(0xCC);
    replacement.operation_revision = 0;
    replacement.expected_operation_revision = None;
    let tx = conn.transaction().unwrap();
    assert!(asset_locks::apply(&tx, &wallet_id, &upsert(replacement)).is_err());
    tx.rollback().unwrap();
    drop(conn);
    drop(persister);
    let reopened = reopen(&path);
    let final_state = asset_locks::load_unconsumed(
        &reopened.lock_conn_for_test(),
        &wallet_id,
        &LoadCtx::strict(),
    )
    .unwrap();
    let lock = &final_state[&0][&outpoint];
    assert_eq!(lock.operation_id, Some([0xBD; 16]));
    assert_eq!(lock.operation_revision, 2);
    assert_eq!(lock.status, AssetLockStatus::RecoveredFromChain);
    assert_eq!(lock.proof, recovered.proof);
}

#[test]
fn should_migrate_conflict_sidecar_without_changing_existing_lifecycle_blobs() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    migrations::runner()
        .set_target(refinery::Target::Version(20))
        .run(&mut conn)
        .unwrap();
    let wallet_id = wid(0xC1);
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        [&wallet_id[..]],
    )
    .unwrap();
    let outpoint = op(0xC1);
    let original = entry(outpoint, AssetLockStatus::Broadcast);
    let blob = asset_locks::encode_entry_for_test(&original).unwrap();
    let op_bytes = blob::encode_outpoint(&outpoint).unwrap();
    conn.execute("INSERT INTO asset_locks (wallet_id,outpoint,status,account_index,identity_index,amount_duffs,lifecycle_blob,operation_id) VALUES (?1,?2,'broadcast',0,0,1000,?3,?4)", rusqlite::params![wallet_id.as_slice(), op_bytes, &blob, [0xC1u8;16].as_slice()]).unwrap();
    migrations::run(&mut conn).unwrap();
    let (restored_blob, revision): (Vec<u8>, i64) = conn
        .query_row(
            "SELECT lifecycle_blob,operation_revision FROM asset_locks",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(restored_blob, blob);
    assert_eq!(revision, 0);
    assert_eq!(
        asset_locks::load_unconsumed(
            &conn,
            &wallet_id,
            &platform_wallet_storage::LoadCtx::strict()
        )
        .unwrap()[&0][&outpoint]
            .operation_id,
        Some([0xC1; 16])
    );
}
