#![allow(clippy::field_reassign_with_default)]

//! The status-filtered asset-lock reader excludes terminal `Consumed`
//! rows so a spent one-shot lock never resurrects as actionable on
//! rehydration, while the historical row stays on disk.

use crate::common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::{OutPoint, Transaction, Txid};
#[cfg(feature = "shielded")]
use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
#[cfg(feature = "shielded")]
use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::{
    AssetLockChangeSet, AssetLockEntry, PlatformWalletPersistence, ShieldedRecipientRaw,
    SHIELDED_RECIPIENT_SIZE,
};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet_storage::sqlite::error::WalletStorageError;
use platform_wallet_storage::sqlite::{
    migrations,
    schema::{asset_locks, blob},
};
#[cfg(feature = "shielded")]
use platform_wallet_storage::LoadCtx;

#[cfg(feature = "shielded")]
use dashcore::Network;
#[cfg(feature = "shielded")]
use key_wallet::account::StandardAccountType;
#[cfg(feature = "shielded")]
use platform_wallet::{
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
        shielded_recipient: None,
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

#[test]
fn should_preserve_shielded_recipient_when_later_snapshot_lacks_it_and_reject_a_change() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA9);
    ensure_wallet_meta(&persister, &w);
    let outpoint = op(0x35);
    let recipient = recipient_bytes(0x11);
    let store = |persister: &platform_wallet_storage::SqlitePersister, row: AssetLockEntry| {
        persister.store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(AssetLockChangeSet {
                    asset_locks: [(outpoint, row)].into_iter().collect(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
    };
    let mut recorded = entry(outpoint, AssetLockStatus::Built);
    recorded.shielded_recipient = Some(recipient);
    store(&persister, recorded).unwrap();
    // A later snapshot without the recipient (a status advance) keeps it.
    store(&persister, entry(outpoint, AssetLockStatus::Broadcast)).unwrap();
    // A snapshot naming another recipient is refused before any write.
    let mut redirected = entry(outpoint, AssetLockStatus::ChainLocked);
    redirected.shielded_recipient = Some(recipient_bytes(0x12));
    assert!(store(&persister, redirected).is_err());
    drop(persister);

    let persister = reopen(&path);
    let conn = persister.lock_conn_for_test();
    let locks =
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .unwrap();
    let restored = &locks[&0][&outpoint];
    assert_eq!(restored.shielded_recipient, Some(recipient));
    assert_eq!(restored.status, AssetLockStatus::Broadcast);
    // The column refuses any length but 43 and never changes once set.
    assert!(conn
        .execute(
            "UPDATE asset_locks SET shielded_recipient = ?1",
            [vec![1_u8; 42]]
        )
        .is_err());
    assert!(conn
        .execute(
            "UPDATE asset_locks SET shielded_recipient = ?1",
            [recipient_bytes(0x13).to_vec()]
        )
        .is_err());
}

#[test]
fn should_fail_the_load_when_a_stored_shielded_recipient_has_the_wrong_length() {
    let (persister, _tmp, _) = fresh_persister();
    let w = wid(0xAB);
    ensure_wallet_meta(&persister, &w);
    let outpoint = op(0x36);
    persister
        .store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(AssetLockChangeSet {
                    asset_locks: [(outpoint, entry(outpoint, AssetLockStatus::Built))]
                        .into_iter()
                        .collect(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let conn = persister.lock_conn_for_test();
    // The CHECK constraint stops this at the SQL layer; bypass it to prove
    // the reader fails closed on its own rather than yielding a row with
    // no recipient (which a resume would pay to whatever the caller names).
    conn.execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    conn.execute(
        "UPDATE asset_locks SET shielded_recipient = ?1",
        [vec![0xff_u8; SHIELDED_RECIPIENT_SIZE - 1]],
    )
    .unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = OFF")
        .unwrap();
    assert!(
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .is_err()
    );
}

#[test]
fn sqlite_advertises_asset_lock_shielded_recipients() {
    let (persister, _tmp, _) = fresh_persister();
    assert!(persister.persistence_capabilities().contains(
        platform_wallet::changeset::PersistenceCapabilities::ASSET_LOCK_SHIELDED_RECIPIENTS
    ));
}

fn recipient_bytes(seed_byte: u8) -> ShieldedRecipientRaw {
    [seed_byte; SHIELDED_RECIPIENT_SIZE]
}

/// The point read sees a `Consumed` row the rehydration feed drops, keeps
/// it across a reopen, returns a live row's status, and `None` for an
/// outpoint the wallet never stored or another wallet's row.
#[test]
fn should_read_an_asset_locks_status_by_outpoint_including_consumed_after_reopen() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA6);
    let other = wid(0xA7);
    ensure_wallet_meta(&persister, &w);
    ensure_wallet_meta(&persister, &other);
    let consumed = op(0x30);
    let live = op(0x31);
    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks
        .insert(consumed, entry(consumed, AssetLockStatus::Consumed));
    cs.asset_locks
        .insert(live, entry(live, AssetLockStatus::InstantSendLocked));
    persister
        .store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        persister.asset_lock_status(&w, &consumed).unwrap(),
        Some(AssetLockStatus::Consumed)
    );
    drop(persister);

    let p2 = reopen(&path);
    assert_eq!(
        p2.asset_lock_status(&w, &consumed).unwrap(),
        Some(AssetLockStatus::Consumed),
        "a consumed row reads back after a reopen"
    );
    assert_eq!(
        p2.asset_lock_status(&w, &live).unwrap(),
        Some(AssetLockStatus::InstantSendLocked)
    );
    assert_eq!(p2.asset_lock_status(&w, &op(0x32)).unwrap(), None);
    assert_eq!(
        p2.asset_lock_status(&other, &consumed).unwrap(),
        None,
        "another wallet's row is not this wallet's"
    );
}

/// Plant a row whose typed status says `consumed` while its lifecycle blob
/// says `Built`.
fn plant_consumed_column_over_built_blob(
    persister: &platform_wallet_storage::SqlitePersister,
    w: &platform_wallet::wallet::platform_wallet::WalletId,
    outpoint: OutPoint,
) {
    use platform_wallet_storage::sqlite::schema::blob;
    ensure_wallet_meta(persister, w);
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT INTO asset_locks \
            (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
         VALUES (?1, ?2, 'consumed', 0, 0, 1000, ?3)",
        rusqlite::params![
            w.as_slice(),
            blob::encode_outpoint(&outpoint).unwrap(),
            asset_locks::encode_entry_for_test(&entry(outpoint, AssetLockStatus::Built)).unwrap()
        ],
    )
    .expect("plant asset-lock status drift");
}

/// Status drift through the point read: Strict refuses it with the named
/// error; Recovery returns the sticky `Consumed` and adds one drift per
/// read to the degradation snapshot.
#[test]
fn should_follow_the_load_policy_when_a_point_read_meets_status_drift() {
    use platform_wallet_storage::LoadSite;
    let w = wid(0xA8);
    let drifted = op(0x40);

    let (strict, _tmp, _path) = fresh_persister();
    plant_consumed_column_over_built_blob(&strict, &w, drifted);
    assert!(matches!(
        strict.asset_lock_status(&w, &drifted),
        Err(WalletStorageError::AssetLockStatusMismatch { .. })
    ));
    assert!(!strict.last_load_degradation().degraded);

    let (recovery, _tmp, _path) = common::fresh_recovery_persister(|strict| {
        plant_consumed_column_over_built_blob(strict, &w, drifted);
    });
    for reads in 1..=2 {
        assert_eq!(
            recovery.asset_lock_status(&w, &drifted).unwrap(),
            Some(AssetLockStatus::Consumed),
            "consumption is sticky"
        );
        let degradation = recovery.last_load_degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation
                .by_site
                .get(&LoadSite::AssetLockStatusDrift)
                .copied(),
            Some(reads),
            "{:?}",
            degradation.by_site
        );
        assert_eq!(degradation.total, reads);
    }
}

/// A `DrainCompleted` lock reads back as exactly that, though its typed
/// column projects it to `consumed`.
#[test]
fn should_read_a_drain_completed_lock_as_drain_completed() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA9);
    ensure_wallet_meta(&persister, &w);
    let drained = op(0x41);
    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks
        .insert(drained, entry(drained, AssetLockStatus::DrainCompleted));
    persister
        .store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        persister.asset_lock_status(&w, &drained).unwrap(),
        Some(AssetLockStatus::DrainCompleted)
    );
    assert!(!persister.last_load_degradation().degraded);
}

/// Under `FlushMode::Manual` a stored lock is in the write buffer, not the
/// committed table: the point read sees none until the flush.
#[test]
fn should_not_see_a_buffered_lock_until_its_flush() {
    let (persister, _tmp, _path) = common::fresh_persister_with_mode(common::FlushMode::Manual);
    let w = wid(0xAA);
    ensure_wallet_meta(&persister, &w);
    let buffered = op(0x42);
    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks.insert(
        buffered,
        entry(buffered, AssetLockStatus::InstantSendLocked),
    );
    persister
        .store(
            w,
            platform_wallet::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(persister.asset_lock_status(&w, &buffered).unwrap(), None);
    persister.flush(w).unwrap();
    assert_eq!(
        persister.asset_lock_status(&w, &buffered).unwrap(),
        Some(AssetLockStatus::InstantSendLocked)
    );
}

/// The snapshot the wallet's conflict cleanup
/// (`remove_tracked_asset_locks_for_swept`) persists for a UUID-bound lock
/// whose funding transaction lost a conflict.
#[cfg(feature = "shielded")]
fn conflict_snapshot(lock: &AssetLockEntry) -> AssetLockChangeSet {
    let mut conflict = lock.clone();
    conflict.status = AssetLockStatus::Conflicted;
    conflict.proof = None;
    conflict.advance_operation_revision();
    AssetLockChangeSet {
        asset_locks: [(conflict.out_point, conflict)].into_iter().collect(),
        ..Default::default()
    }
}

/// A real restored manager reports durable conflict and rejects the same UUID
/// before signing, broadcasting, or mutating storage after restart.
#[cfg(feature = "shielded")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_report_conflict_and_refuse_same_uuid_drain_after_conflict_and_reopen() {
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
                    asset_locks: [(loser, lock.clone())].into_iter().collect(),
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
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                asset_locks: Some(conflict_snapshot(&lock)),
                ..Default::default()
            },
        )
        .unwrap();
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
fn should_retain_operation_binding_through_removal_across_restart() {
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
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                asset_locks: Some(AssetLockChangeSet {
                    removed: [outpoint].into_iter().collect(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
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
        "a removal must retain the original UUID binding"
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
    asset_locks::apply(&tx, &wallet_id, &conflict_snapshot(&original)).unwrap();
    tx.rollback().unwrap();
    let before = asset_locks::load_unconsumed(&conn, &wallet_id, &LoadCtx::strict()).unwrap();
    assert_eq!(before[&0][&outpoint].status, AssetLockStatus::Broadcast);
    assert_eq!(before[&0][&outpoint].operation_revision, 0);
    let tx = conn.transaction().unwrap();
    asset_locks::apply(&tx, &wallet_id, &conflict_snapshot(&original)).unwrap();
    tx.commit().unwrap();
    let conflict = asset_locks::load_unconsumed(&conn, &wallet_id, &LoadCtx::strict()).unwrap()[&0]
        [&outpoint]
        .clone();
    assert_eq!(conflict.status, AssetLockStatus::Conflicted);
    assert_eq!(conflict.operation_revision, 1);
    assert!(conflict.proof.is_none());
    assert_eq!(
        asset_locks::status_of(&conn, &wallet_id, &outpoint, &LoadCtx::strict()).unwrap(),
        Some(AssetLockStatus::Conflicted)
    );
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
        .set_target(refinery::Target::Version(22))
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

/// A compare-and-set re-delivered over a row that stores a shielded recipient
/// is accepted when it is the exact committed snapshot, and refused when it
/// names a different recipient; the stored recipient never changes.
#[test]
fn should_accept_exact_cas_redelivery_and_refuse_a_changed_recipient() {
    let (persister, _tmp, _path) = fresh_persister();
    let wallet_id = wid(0xC2);
    ensure_wallet_meta(&persister, &wallet_id);
    let outpoint = op(0xC2);
    let recipient = recipient_bytes(0x21);
    let mut lock = entry(outpoint, AssetLockStatus::Broadcast);
    lock.operation_id = Some([0xC2; 16]);
    lock.shielded_recipient = Some(recipient);
    let upsert = |entry: AssetLockEntry| AssetLockChangeSet {
        asset_locks: [(entry.out_point, entry)].into_iter().collect(),
        ..Default::default()
    };
    let mut conflict = lock.clone();
    conflict.status = AssetLockStatus::Conflicted;
    conflict.advance_operation_revision();

    let mut conn = persister.lock_conn_for_test();
    for (row, accepted) in [
        (lock, true),
        (conflict.clone(), true),
        // Lost commit acknowledgement: the identical CAS is re-delivered.
        (conflict.clone(), true),
        (
            AssetLockEntry {
                shielded_recipient: Some(recipient_bytes(0x22)),
                ..conflict.clone()
            },
            false,
        ),
    ] {
        let tx = conn.transaction().unwrap();
        let result = asset_locks::apply(&tx, &wallet_id, &upsert(row));
        assert_eq!(result.is_ok(), accepted, "{result:?}");
        if accepted {
            tx.commit().unwrap();
        }
    }
    let (stored_recipient, revision): (Vec<u8>, i64) = conn
        .query_row(
            "SELECT shielded_recipient, operation_revision FROM asset_locks WHERE wallet_id = ?1",
            [wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored_recipient, recipient.to_vec());
    assert_eq!(revision, 1);
}
