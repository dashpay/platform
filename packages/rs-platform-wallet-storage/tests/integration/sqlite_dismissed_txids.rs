#![allow(clippy::field_reassign_with_default)]

//! The transactions a wallet dropped as dead (`core_dismissed_txids`): each
//! change key-wallet reports is written with the removal it rides on, the
//! load hands swept losers and abandons back apart, each oldest first and
//! with a bound of its own, and a transaction mined or verified-locked since
//! is not handed back.

use crate::common;

use common::{
    ensure_wallet_meta, fresh_persister, store_and_flush, wid, SqlitePersister,
    SqlitePersisterConfig,
};

use dashcore::{Transaction, Txid};
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::transaction_record::{TransactionDirection, TransactionRecord};
use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
use platform_wallet::changeset::changeset::SweepBatch;
use platform_wallet::changeset::{CoreChangeSet, DismissalChange, PlatformWalletChangeSet};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::{blob, core_state};
use platform_wallet_storage::LoadCtx;
use rusqlite::params;

/// The migration that adds `core_dismissed_txids`.
const DISMISSED_TXIDS_VERSION: i64 = 26;

fn tx(n: u32) -> Transaction {
    Transaction {
        version: 3,
        lock_time: n,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    }
}

fn txid(n: u32) -> Txid {
    tx(n).txid()
}

fn record(n: u32, context: TransactionContext) -> TransactionRecord {
    TransactionRecord::new(
        tx(n),
        AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        },
        context,
        TransactionType::Standard,
        TransactionDirection::Outgoing,
        Vec::new(),
        Vec::new(),
        0,
    )
}

fn core(cs: CoreChangeSet) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(cs),
        ..Default::default()
    }
}

fn changes(changes: Vec<DismissalChange>) -> PlatformWalletChangeSet {
    core(CoreChangeSet {
        dismissal_changes: changes,
        ..Default::default()
    })
}

/// The record of `tx(n)`, mined.
fn mined(n: u32) -> PlatformWalletChangeSet {
    core(CoreChangeSet {
        records: vec![record(
            n,
            TransactionContext::InBlock(BlockInfo::new(
                42,
                dashcore::BlockHash::from_byte_array([3u8; 32]),
                1_735_689_600,
            )),
        )],
        ..Default::default()
    })
}

fn sweep_of(t: Txid) -> SweepBatch {
    SweepBatch {
        txids: vec![t],
        superseded_by: txid(0xFFFF),
        winner_mined_height: None,
        released_outpoints: vec![],
    }
}

fn loaded_both(persister: &SqlitePersister, w: &WalletId) -> (Vec<Txid>, Vec<Txid>) {
    let conn = persister.lock_conn_for_test();
    let core = core_state::load_state(&conn, w, dashcore::Network::Testnet, &LoadCtx::strict())
        .expect("load")
        .0;
    (core.dismissed_txids, core.abandoned_txids)
}

/// The swept losers the load hands back.
fn loaded(persister: &SqlitePersister, w: &WalletId) -> Vec<Txid> {
    loaded_both(persister, w).0
}

/// The abandons the load hands back.
fn abandoned(persister: &SqlitePersister, w: &WalletId) -> Vec<Txid> {
    loaded_both(persister, w).1
}

fn sweeps(txids: &[Txid]) -> PlatformWalletChangeSet {
    changes(
        txids
            .iter()
            .copied()
            .map(DismissalChange::Dismissed)
            .collect(),
    )
}

fn rows(persister: &SqlitePersister, w: &WalletId) -> Vec<Txid> {
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare("SELECT txid FROM core_dismissed_txids WHERE wallet_id = ?1 ORDER BY id")
        .unwrap();
    stmt.query_map(params![w.as_slice()], |r| r.get::<_, Vec<u8>>(0))
        .unwrap()
        .map(|b| Txid::from_slice(&b.unwrap()).unwrap())
        .collect()
}

fn record_stored(persister: &SqlitePersister, w: &WalletId, t: &Txid) -> bool {
    let conn = persister.lock_conn_for_test();
    conn.query_row(
        "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
        params![w.as_slice(), AsRef::<[u8]>::as_ref(t)],
        |r| r.get::<_, i64>(0),
    )
    .unwrap()
        == 1
}

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

fn store_lock(persister: &SqlitePersister, w: &WalletId, t: Txid, verified: i64) {
    let lock = blob::encode(&dashcore::InstantLock {
        txid: t,
        ..Default::default()
    })
    .unwrap();
    persister
        .lock_conn_for_test()
        .execute(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob, verified) \
             VALUES (?1, ?2, ?3, ?4)",
            params![w.as_slice(), AsRef::<[u8]>::as_ref(&t), lock, verified],
        )
        .unwrap();
}

/// A dismissal and the sweep it rides on commit together: a write that
/// stops before its commit leaves the record and no row, and the committed
/// one leaves the row and no record.
#[test]
fn a_dismissal_commits_with_its_sweep_or_not_at_all() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD1);
    ensure_wallet_meta(&persister, &w);
    let loser = txid(1);
    store_and_flush(
        &persister,
        w,
        core(CoreChangeSet {
            records: vec![record(1, TransactionContext::Mempool)],
            ..Default::default()
        }),
    );
    let cs = CoreChangeSet {
        sweeps: vec![sweep_of(loser)],
        dismissal_changes: vec![DismissalChange::Dismissed(loser)],
        ..Default::default()
    };

    {
        let mut conn = persister.lock_conn_for_test();
        let tx = conn.transaction().unwrap();
        core_state::apply(&tx, &w, &cs).unwrap();
        drop(tx);
    }
    assert!(
        record_stored(&persister, &w, &loser),
        "the sweep applied without its commit"
    );
    assert!(
        rows(&persister, &w).is_empty(),
        "the dismissal stored without its commit"
    );

    store_and_flush(&persister, w, core(cs));
    assert!(
        !record_stored(&persister, &w, &loser),
        "the sweep did not apply"
    );
    assert_eq!(
        rows(&persister, &w),
        vec![loser],
        "the dismissal was not stored with it"
    );
}

/// A settle key-wallet reports (the dismissed transaction mined or locked)
/// removes its row, so the load hands it back no more.
#[test]
fn a_settled_dismissal_is_forgotten() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD2);
    ensure_wallet_meta(&persister, &w);
    let (a, b) = (txid(1), txid(2));
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Dismissed(a),
            DismissalChange::Dismissed(b),
        ]),
    );
    assert_eq!(loaded(&persister, &w), vec![a, b]);
    store_and_flush(&persister, w, changes(vec![DismissalChange::Forgotten(a)]));
    assert_eq!(loaded(&persister, &w), vec![b]);
}

/// Past the bound the wallet's oldest row goes, also when key-wallet's own
/// report of the eviction is missing, no other wallet's row goes, and the
/// load after a restart hands the rest back oldest first, the newest kept.
#[test]
fn past_the_bound_the_oldest_dismissal_goes_and_the_order_survives_a_restart() {
    let (persister, _tmp, path) = fresh_persister();
    let (w, other) = (wid(0xD3), wid(0xDC));
    ensure_wallet_meta(&persister, &w);
    ensure_wallet_meta(&persister, &other);
    let all: Vec<Txid> = (0..4_097).map(txid).collect();
    let dismiss = |txids: &[Txid]| {
        changes(
            txids
                .iter()
                .copied()
                .map(DismissalChange::Dismissed)
                .collect(),
        )
    };
    store_and_flush(&persister, w, dismiss(&all[..4_096]));
    store_and_flush(
        &persister,
        other,
        changes(vec![DismissalChange::Dismissed(txid(0xFFFE))]),
    );
    store_and_flush(&persister, w, dismiss(&all[4_096..]));
    drop(persister);

    let persister = reopen(&path);
    let kept = loaded(&persister, &w);
    assert_eq!(kept.len(), 4_096);
    assert_eq!(kept, all[1..], "not the newest 4,096, oldest first");
    assert_eq!(rows(&persister, &other), vec![txid(0xFFFE)]);
}

/// A dismissed transaction the store holds mined, or with a lock stored as
/// verified, is not dead, whichever its kind: the load skips it and the
/// next writable open (or the wallet's next dismissal write) deletes its
/// row. A lock not stored as verified
/// settles nothing, on the load or at the open, and a Recovery open deletes
/// nothing.
#[test]
fn a_dismissal_of_a_mined_or_verified_locked_transaction_is_dropped() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD4);
    ensure_wallet_meta(&persister, &w);
    let (in_block, locked, unverified, dead) = (txid(1), txid(2), txid(3), txid(4));
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Dismissed(in_block),
            DismissalChange::Abandoned(locked),
            DismissalChange::Dismissed(unverified),
            DismissalChange::Abandoned(dead),
        ]),
    );
    store_and_flush(&persister, w, mined(1));
    store_lock(&persister, &w, locked, 1);
    store_lock(&persister, &w, unverified, 0);
    assert_eq!(loaded_both(&persister, &w), (vec![unverified], vec![dead]));
    assert_eq!(
        rows(&persister, &w).len(),
        4,
        "neither the load nor a write without dismissal changes deletes rows"
    );
    drop(persister);

    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(&path)
            .with_load_policy(platform_wallet_storage::LoadPolicy::Recovery),
    )
    .expect("recovery open");
    assert_eq!(rows(&recovery, &w).len(), 4, "a Recovery open deleted rows");
    drop(recovery);

    let persister = reopen(&path);
    assert_eq!(rows(&persister, &w), vec![unverified, dead]);
}

/// Deleting a wallet deletes its dismissals and no other wallet's.
#[test]
fn a_deleted_wallet_leaves_no_dismissals() {
    let (persister, _tmp, _path) = fresh_persister();
    let (w, other) = (wid(0xD5), wid(0xD6));
    ensure_wallet_meta(&persister, &w);
    ensure_wallet_meta(&persister, &other);
    store_and_flush(
        &persister,
        w,
        changes(vec![DismissalChange::Dismissed(txid(1))]),
    );
    store_and_flush(
        &persister,
        other,
        changes(vec![DismissalChange::Dismissed(txid(2))]),
    );
    persister.delete_wallet_skip_backup(w).expect("delete");
    assert!(rows(&persister, &w).is_empty());
    assert_eq!(rows(&persister, &other), vec![txid(2)]);
}

fn applied_versions(persister: &SqlitePersister) -> Vec<i64> {
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare("SELECT version FROM refinery_schema_history ORDER BY version")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// A database from before the table opens with it added, its data intact,
/// and keeps dismissals from then on.
#[test]
fn a_database_from_before_the_table_takes_it_on_open() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD7);
    ensure_wallet_meta(&persister, &w);
    let kept = txid(9);
    store_and_flush(
        &persister,
        w,
        core(CoreChangeSet {
            records: vec![record(9, TransactionContext::Mempool)],
            ..Default::default()
        }),
    );
    let all = applied_versions(&persister);
    assert!(all.contains(&DISMISSED_TXIDS_VERSION));
    drop(persister);
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "DROP TABLE core_dismissed_txids;
             DELETE FROM refinery_schema_history WHERE version = {DISMISSED_TXIDS_VERSION};"
        ))
        .unwrap();
    }

    let persister = reopen(&path);
    assert!(record_stored(&persister, &w, &kept));
    assert_eq!(applied_versions(&persister), all);
    store_and_flush(
        &persister,
        w,
        changes(vec![DismissalChange::Dismissed(txid(1))]),
    );
    assert_eq!(loaded(&persister, &w), vec![txid(1)]);
}

/// Every row with its `id`, in `id` order.
fn id_rows(conn: &rusqlite::Connection) -> Vec<(i64, Vec<u8>)> {
    let mut stmt = conn
        .prepare("SELECT id, txid FROM core_dismissed_txids ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// The table and index shape SQLite reports, independent of DDL text.
fn dismissed_table_shape(conn: &rusqlite::Connection) -> Vec<String> {
    let mut shape = Vec::new();
    for pragma in [
        "pragma_table_info('core_dismissed_txids')",
        "pragma_foreign_key_list('core_dismissed_txids')",
        "pragma_index_list('core_dismissed_txids')",
    ] {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {pragma} ORDER BY 1, 2"))
            .unwrap();
        let n = stmt.column_count();
        let rows = stmt
            .query_map([], |r| {
                (0..n)
                    .map(|i| {
                        r.get::<_, rusqlite::types::Value>(i)
                            .map(|v| format!("{v:?}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap();
        for row in rows {
            shape.push(format!("{pragma}: {}", row.unwrap().join(", ")));
        }
    }
    shape
}

/// A database that already has the table, from before it had a kind, keeps
/// its rows: the table is rebuilt in a fresh file's shape, the rows keep
/// their ids and order and read as abandoned, so no later sweeps push them
/// out.
#[test]
fn a_database_that_already_has_the_table_adopts_it() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xD8);
    ensure_wallet_meta(&persister, &w);
    let fresh_shape = dismissed_table_shape(&persister.lock_conn_for_test());
    let all = applied_versions(&persister);
    assert!(all.contains(&DISMISSED_TXIDS_VERSION));
    drop(persister);
    let before: Vec<(i64, Vec<u8>)> = {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "DROP TABLE core_dismissed_txids;
             CREATE TABLE core_dismissed_txids (
                 id INTEGER PRIMARY KEY,
                 wallet_id BLOB NOT NULL,
                 txid BLOB NOT NULL,
                 UNIQUE (wallet_id, txid),
                 FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
             );",
        )
        .unwrap();
        for (id, n) in [(5_i64, 3_u32), (9, 1), (12, 2)] {
            conn.execute(
                "INSERT INTO core_dismissed_txids (id, wallet_id, txid) VALUES (?1, ?2, ?3)",
                params![id, w.as_slice(), AsRef::<[u8]>::as_ref(&txid(n))],
            )
            .unwrap();
        }
        conn.execute(
            "DELETE FROM refinery_schema_history WHERE version = ?1",
            [DISMISSED_TXIDS_VERSION],
        )
        .unwrap();
        id_rows(&conn)
    };

    let persister = reopen(&path);
    assert_eq!(applied_versions(&persister), all);
    {
        let conn = persister.lock_conn_for_test();
        assert_eq!(id_rows(&conn), before, "the adopted rows changed");
        assert_eq!(dismissed_table_shape(&conn), fresh_shape);
    }
    let old = vec![txid(3), txid(1), txid(2)];
    assert_eq!(loaded_both(&persister, &w), (Vec::new(), old.clone()));

    let later: Vec<Txid> = (0x100..0x100 + 4_097).map(txid).collect();
    store_and_flush(&persister, w, sweeps(&later));
    assert_eq!(
        abandoned(&persister, &w),
        old,
        "later sweeps pushed out an adopted row"
    );
    assert_eq!(loaded(&persister, &w), later[1..]);
}

/// Each kind has its own bound: past 4,096 of one kind the oldest of that
/// kind goes and none of the other.
#[test]
fn each_kind_keeps_its_own_newest_4096() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xDD);
    ensure_wallet_meta(&persister, &w);
    let user: Vec<Txid> = (0..4_097).map(txid).collect();
    let swept: Vec<Txid> = (0x10_000..0x10_000 + 4_097).map(txid).collect();
    store_and_flush(
        &persister,
        w,
        changes(
            user.iter()
                .copied()
                .map(DismissalChange::Abandoned)
                .collect(),
        ),
    );
    store_and_flush(&persister, w, sweeps(&swept));
    assert_eq!(abandoned(&persister, &w), user[1..]);
    assert_eq!(loaded(&persister, &w), swept[1..]);

    let other = wid(0xD8);
    ensure_wallet_meta(&persister, &other);
    store_and_flush(&persister, other, sweeps(&swept));
    store_and_flush(
        &persister,
        other,
        changes(
            user.iter()
                .copied()
                .map(DismissalChange::Abandoned)
                .collect(),
        ),
    );
    assert_eq!(abandoned(&persister, &other), user[1..]);
    assert_eq!(loaded(&persister, &other), swept[1..]);
}

/// A transaction is held as key-wallet holds it, in either set or both:
/// an abandon leaves its sweep in place and a sweep leaves its abandon,
/// each keeping a row of its kind already there in its place or adding one
/// as the newest, and a forgetting ends both.
#[test]
fn a_transaction_is_held_in_each_set_key_wallet_holds_it_in() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xDE);
    ensure_wallet_meta(&persister, &w);
    let (a, b, c, d) = (txid(1), txid(2), txid(3), txid(4));
    store_and_flush(&persister, w, sweeps(&[a, b]));
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Abandoned(c),
            DismissalChange::Abandoned(a),
        ]),
    );
    assert_eq!(loaded_both(&persister, &w), (vec![a, b], vec![c, a]));
    store_and_flush(&persister, w, changes(vec![DismissalChange::Abandoned(c)]));
    store_and_flush(&persister, w, sweeps(&[d]));
    assert_eq!(loaded_both(&persister, &w), (vec![a, b, d], vec![c, a]));
    store_and_flush(&persister, w, sweeps(&[c]));
    assert_eq!(loaded_both(&persister, &w), (vec![a, b, d, c], vec![c, a]));
    store_and_flush(&persister, w, sweeps(&[a]));
    assert_eq!(loaded_both(&persister, &w), (vec![a, b, d, c], vec![c, a]));
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Abandoned(b),
            DismissalChange::Forgotten(b),
            DismissalChange::Forgotten(c),
        ]),
    );
    assert_eq!(loaded_both(&persister, &w), (vec![a, d], vec![a]));
}

/// A sweep that pushes out of a full swept set a transaction still abandoned
/// is no report, and a forgetting later in the same write must not leave
/// that row in place of the one key-wallet dropped.
#[test]
fn a_silent_push_out_and_a_forgetting_in_one_write_keep_the_cap() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xDB);
    ensure_wallet_meta(&persister, &w);
    let full: Vec<Txid> = (0x100..0x100 + 4_096).map(txid).collect();
    let (f, gone, y) = (full[0], full[5], txid(0x9000));
    store_and_flush(&persister, w, sweeps(&full));
    store_and_flush(&persister, w, changes(vec![DismissalChange::Abandoned(f)]));
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Dismissed(y),
            DismissalChange::Forgotten(gone),
        ]),
    );
    let swept: Vec<Txid> = full[1..]
        .iter()
        .copied()
        .filter(|t| *t != gone)
        .chain([y])
        .collect();
    assert_eq!(loaded_both(&persister, &w), (swept, vec![f]));
}

/// A dismissal settled without its forgetting is not held by key-wallet
/// after a load, so it takes no place in the bound of a later write.
#[test]
fn a_settled_dismissal_takes_no_place_in_a_later_write() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xDC);
    ensure_wallet_meta(&persister, &w);
    let full: Vec<Txid> = (0x100..0x100 + 4_096).map(txid).collect();
    let (settled, y) = (full[5], txid(0x9000));
    store_and_flush(&persister, w, sweeps(&full));
    store_and_flush(&persister, w, mined(0x105));
    store_and_flush(&persister, w, sweeps(&[y]));
    let swept: Vec<Txid> = full
        .iter()
        .copied()
        .filter(|t| *t != settled)
        .chain([y])
        .collect();
    assert_eq!(loaded_both(&persister, &w), (swept, Vec::new()));
}

/// An abandon pushed out of a full abandoned set without a report, then
/// abandoned again in the same write, is the newest abandon, and the one
/// behind it goes while its sweep stays.
#[test]
fn an_abandon_pushed_out_and_abandoned_again_in_one_write_is_the_newest() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xDA);
    ensure_wallet_meta(&persister, &w);
    let full: Vec<Txid> = (0x100..0x100 + 4_096).map(txid).collect();
    let (f, second, n) = (full[0], full[1], txid(0x9000));
    store_and_flush(&persister, w, sweeps(&[f, second]));
    store_and_flush(
        &persister,
        w,
        changes(
            full.iter()
                .copied()
                .map(DismissalChange::Abandoned)
                .collect(),
        ),
    );
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Abandoned(n),
            DismissalChange::Abandoned(f),
        ]),
    );
    let abandoned: Vec<Txid> = full[2..].iter().copied().chain([n, f]).collect();
    assert_eq!(loaded_both(&persister, &w), (vec![f, second], abandoned));
}

/// The changes of one write apply in the order key-wallet made them: a
/// transaction dismissed again after it was forgotten is the newest, and
/// one dismissed again while still held keeps its place.
#[test]
fn the_changes_of_one_write_apply_in_order() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xD9);
    ensure_wallet_meta(&persister, &w);
    let (a, b, c) = (txid(1), txid(2), txid(3));
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Dismissed(a),
            DismissalChange::Dismissed(b),
            DismissalChange::Dismissed(c),
            DismissalChange::Forgotten(c),
        ]),
    );
    assert_eq!(rows(&persister, &w), vec![a, b]);
    store_and_flush(
        &persister,
        w,
        changes(vec![
            DismissalChange::Forgotten(a),
            DismissalChange::Dismissed(a),
        ]),
    );
    assert_eq!(rows(&persister, &w), vec![b, a]);
    store_and_flush(&persister, w, changes(vec![DismissalChange::Dismissed(b)]));
    assert_eq!(rows(&persister, &w), vec![b, a]);
}

/// A transaction one wallet holds mined or verified-locked settles that
/// wallet's dismissal of it and no other wallet's, on the load and at the
/// open.
#[test]
fn a_transaction_settled_in_one_wallet_settles_no_other_wallets_dismissal() {
    let (persister, _tmp, path) = fresh_persister();
    let (w, other) = (wid(0xDA), wid(0xDB));
    ensure_wallet_meta(&persister, &w);
    ensure_wallet_meta(&persister, &other);
    let (in_block, locked) = (txid(1), txid(2));
    store_and_flush(&persister, w, mined(1));
    store_lock(&persister, &w, locked, 1);
    for wallet in [w, other] {
        store_and_flush(
            &persister,
            wallet,
            changes(vec![
                DismissalChange::Dismissed(in_block),
                DismissalChange::Dismissed(locked),
            ]),
        );
    }
    assert!(
        loaded(&persister, &w).is_empty(),
        "the mined record or the verified lock was not stored"
    );
    assert_eq!(loaded(&persister, &other), vec![in_block, locked]);
    drop(persister);

    let persister = reopen(&path);
    assert!(rows(&persister, &w).is_empty());
    assert_eq!(rows(&persister, &other), vec![in_block, locked]);
}

/// A transaction pushed out of the swept set while still abandoned stays
/// abandoned only: the sweep that pushed it out trims its swept row.
#[test]
fn a_sweep_pushed_out_while_abandoned_leaves_the_abandon() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xDF);
    ensure_wallet_meta(&persister, &w);
    let t = txid(1);
    store_and_flush(&persister, w, sweeps(&[t]));
    store_and_flush(&persister, w, changes(vec![DismissalChange::Abandoned(t)]));
    assert_eq!(loaded_both(&persister, &w), (vec![t], vec![t]));
    let later: Vec<Txid> = (0x100..0x100 + 4_096).map(txid).collect();
    store_and_flush(&persister, w, sweeps(&later));
    assert_eq!(loaded_both(&persister, &w), (later, vec![t]));
}
