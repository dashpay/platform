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
