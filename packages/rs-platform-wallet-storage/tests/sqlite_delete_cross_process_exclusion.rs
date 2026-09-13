#![allow(clippy::field_reassign_with_default)]

//! `delete_wallet` must hold a SQLite-native EXCLUSIVE across the
//! (backup + cascade-delete) window so a peer rusqlite
//! Connection (a different process equivalent) can't commit rows
//! between the backup snapshot and the cascade.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use rusqlite::{OptionalExtension as _, TransactionBehavior};

/// When a peer holds EXCLUSIVE on the destination, `delete_wallet`
/// must block / fail on busy rather than proceeding through an
/// in-process-mutex-only path that ignores the peer.
#[test]
fn delete_wallet_blocks_when_peer_holds_exclusive() {
    let (persister, _tmp, db_path) = fresh_persister();
    let w = wid(0x77);
    ensure_wallet_meta(&persister, &w);

    let backup_dir = common::secure_tempdir().expect("backup dir");
    // Re-open with auto-backup wired so delete_wallet exercises the
    // backup + cascade path (the canonical path under test).
    drop(persister);
    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&db_path)
        .with_auto_backup_dir(Some(backup_dir.path().to_path_buf()));
    let persister = platform_wallet_storage::SqlitePersister::open(cfg).expect("re-open");
    ensure_wallet_meta(&persister, &w);

    // Peer opens a writer conn against the same DB file and takes
    // EXCLUSIVE — represents "another process holds the DB busy".
    let mut peer = rusqlite::Connection::open(&db_path).expect("peer open");
    peer.pragma_update(None, "busy_timeout", 50i64).unwrap();
    let tx = peer
        .transaction_with_behavior(TransactionBehavior::Exclusive)
        .expect("peer EXCLUSIVE");

    // Set the persister's busy-timeout low so the test doesn't hang.
    {
        let conn = persister.lock_conn_for_test();
        conn.pragma_update(None, "busy_timeout", 50i64).unwrap();
    }

    let err = persister
        .delete_wallet(w)
        .expect_err("delete must conflict with peer EXCLUSIVE");
    // Detect the busy/locked condition either through the SqliteFailure
    // error code or the source rusqlite error string (which renders
    // "database is locked" etc.). Falls back to a substring check on
    // the Display form so the assertion is robust across rusqlite
    // versions and the exact path that surfaced the conflict (open vs.
    // begin vs. cascade).
    let busy = matches!(
        &err,
        platform_wallet_storage::WalletStorageError::Sqlite(
            rusqlite::Error::SqliteFailure(e, _)
        ) if matches!(
            e.code,
            rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
        )
    );
    let dbg = format!("{err:?}");
    assert!(
        busy || dbg.contains("Busy")
            || dbg.contains("Locked")
            || dbg.contains("database is locked"),
        "expected busy/locked SQLite error, got: {err} | dbg: {dbg}"
    );

    drop(tx);
    drop(peer);
}

/// Single-process load must still pass under the EXCLUSIVE-locked
/// delete path.
#[test]
fn delete_wallet_single_process_still_works() {
    let (persister, _tmp, db_path) = fresh_persister();
    let backup_dir = common::secure_tempdir().expect("backup dir");
    drop(persister);
    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&db_path)
        .with_auto_backup_dir(Some(backup_dir.path().to_path_buf()));
    let persister = platform_wallet_storage::SqlitePersister::open(cfg).expect("re-open");

    let w = wid(0x88);
    ensure_wallet_meta(&persister, &w);

    let report = persister.delete_wallet(w).expect("delete succeeds");
    assert!(report.backup_path.is_some(), "auto-backup should fire");
    // wallets row should be gone.
    let conn = persister.lock_conn_for_test();
    let row: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM wallets WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .optional()
        .expect("wallets query must not fail — only absence is expected");
    assert!(row.is_none(), "wallets row must be gone");
}
