#![allow(clippy::field_reassign_with_default)]

//! The status-filtered asset-lock reader excludes terminal `Consumed`
//! rows so a spent one-shot lock never resurrects as actionable on
//! rehydration, while the historical row stays on disk.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dashcore::hashes::Hash;
use dashcore::{OutPoint, Transaction, Txid};
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry, PlatformWalletPersistence};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet_storage::sqlite::schema::asset_locks;

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
        shielded_funding_attempts: Vec::new(),
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
    assert!(locks[&0][&outpoint].shielded_funding_attempts.is_empty());
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
fn should_restore_append_only_signed_attempts_and_reject_replacement() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xA9);
    ensure_wallet_meta(&persister, &w);
    let outpoint = op(0x33);
    let first = vec![18, 1, 2, 3];
    let second = vec![18, 4, 5, 6, 7];
    let store = |attempts: Vec<Vec<u8>>, status, id| {
        let mut row = entry(outpoint, status);
        row.operation_id = id;
        row.shielded_funding_attempts = attempts;
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
    store(
        vec![first.clone()],
        AssetLockStatus::Broadcast,
        Some([8; 16]),
    )
    .unwrap();
    store(
        vec![first.clone()],
        AssetLockStatus::Consumed,
        Some([8; 16]),
    )
    .unwrap();
    // A delayed evidence snapshot preserves terminal status while appending bytes.
    store(
        vec![first.clone(), second.clone()],
        AssetLockStatus::Broadcast,
        Some([8; 16]),
    )
    .unwrap();
    assert!(store(vec![vec![18, 9]], AssetLockStatus::Consumed, Some([8; 16])).is_err());
    // Legacy and stale prefixes cannot erase persisted attempts, even at terminal status.
    store(Vec::new(), AssetLockStatus::Consumed, None).unwrap();
    store(
        vec![first.clone()],
        AssetLockStatus::Consumed,
        Some([8; 16]),
    )
    .unwrap();
    persister.flush(w).unwrap();
    drop(persister);
    let reopened = reopen(&path);
    let conn = reopened.lock_conn_for_test();
    let locks =
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .unwrap();
    assert_eq!(
        locks[&0][&outpoint].shielded_funding_attempts,
        vec![first, second]
    );
    assert_eq!(locks[&0][&outpoint].status, AssetLockStatus::Consumed);
    conn.execute(
        "UPDATE asset_locks SET shielded_funding_attempts_blob = X'00'",
        [],
    )
    .unwrap();
    assert!(
        asset_locks::load_unconsumed(&conn, &w, &platform_wallet_storage::LoadCtx::strict())
            .is_err()
    );
}
