#![allow(clippy::field_reassign_with_default)]

//! CMT-001 — `delete_wallet` must reconcile the in-memory buffer.
//!
//! `delete_wallet_inner` drains-and-discards the target wallet's
//! buffered changeset before the existence gate, and treats "buffered
//! OR persisted" as existence. These regression tests pin the three
//! historical failure modes: spurious `WalletNotFound` for a
//! buffered-only wallet, a pre-delete backup that excludes buffered
//! writes, and post-delete resurrection on the next flush.

mod common;

use common::{fresh_persister_with_mode, wid};
use key_wallet::Network;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{
    FlushMode, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};
use rusqlite::{ErrorCode, OptionalExtension};

/// A self-consistent changeset: includes `wallet_metadata` so a flush
/// would materialize a brand-new wallet (FK-valid) — modelling a
/// wallet whose only state is buffered.
fn full_changeset(synced: u32) -> PlatformWalletChangeSet {
    let mut cs = PlatformWalletChangeSet::default();
    cs.wallet_metadata = Some(WalletMetadataEntry {
        network: Network::Testnet,
        birth_height: 0,
    });
    cs.core = Some(CoreChangeSet {
        synced_height: Some(synced),
        last_processed_height: Some(synced),
        ..Default::default()
    });
    cs
}

fn busy_error() -> WalletStorageError {
    WalletStorageError::Sqlite(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error {
            code: ErrorCode::DatabaseBusy,
            extended_code: rusqlite::ffi::SQLITE_BUSY,
        },
        Some("database is busy".into()),
    ))
}

fn core_rows_for(persister: &SqlitePersister, w: &[u8; 32]) -> i64 {
    let conn = persister.lock_conn_for_test();
    conn.query_row(
        "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
        rusqlite::params![w.as_slice()],
        |row| row.get(0),
    )
    .unwrap()
}

/// Buffered-only wallet (no persisted row) deletes successfully and a
/// later `commit_writes` cannot resurrect its rows.
#[test]
fn buffered_only_delete_is_ok_and_no_resurrection() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Manual);
    let w = wid(0xB1);
    persister.store(w, full_changeset(5)).unwrap();

    persister
        .delete_wallet_skip_backup(w)
        .expect("buffered-only delete must be Ok, not WalletNotFound");

    persister.commit_writes().expect("commit_writes");
    assert_eq!(
        core_rows_for(&persister, &w),
        0,
        "buffered changeset must not resurrect the deleted wallet"
    );
}

/// The pre-delete backup excludes drained-and-discarded buffered
/// writes — the backup must not contain the wallet.
#[test]
fn pre_delete_backup_excludes_buffered_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let backup_dir = tmp.path().join("backups");
    let cfg = SqlitePersisterConfig::new(&path)
        .with_flush_mode(FlushMode::Manual)
        .with_auto_backup_dir(Some(backup_dir));
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xB2);
    persister.store(w, full_changeset(9)).unwrap();

    let report = persister.delete_wallet(w).expect("delete_wallet");
    let backup_path = report.backup_path.expect("pre-delete backup written");

    let backup = rusqlite::Connection::open_with_flags(
        &backup_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let in_backup: Option<i64> = backup
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(
        in_backup,
        Some(0),
        "pre-delete backup must not contain buffered-but-unflushed rows"
    );
}

/// A wallet that exists in neither the buffer nor the DB still returns
/// `WalletNotFound` — under both flush modes.
#[test]
fn delete_unknown_wallet_is_not_found() {
    for mode in [FlushMode::Manual, FlushMode::Immediate] {
        let (persister, _tmp, _path) = fresh_persister_with_mode(mode);
        let w = wid(0xB3);
        let err = persister.delete_wallet_skip_backup(w);
        assert!(
            matches!(err, Err(WalletStorageError::WalletNotFound { .. })),
            "expected WalletNotFound in {mode:?}, got {err:?}"
        );
    }
}

/// Immediate mode: a transient flush failure restores the changeset to
/// the buffer; a subsequent delete must drain it so no later
/// `commit_writes`/flush resurrects the wallet.
#[test]
fn immediate_after_failed_flush_delete_drains_buffer() {
    let (persister, _tmp, _path) = fresh_persister_with_mode(FlushMode::Immediate);
    let w = wid(0xB4);

    persister.force_next_flush_to_fail(busy_error());
    let _ = persister
        .store(w, full_changeset(7))
        .expect_err("immediate store surfaces the transient error");
    // The changeset is now restored to the buffer.

    persister
        .delete_wallet_skip_backup(w)
        .expect("delete must be Ok with a restored-after-failure buffer entry");

    persister.commit_writes().expect("commit_writes");
    persister.flush(w).expect("flush");
    assert_eq!(
        core_rows_for(&persister, &w),
        0,
        "restored buffered changeset must not resurrect the deleted wallet"
    );
}
