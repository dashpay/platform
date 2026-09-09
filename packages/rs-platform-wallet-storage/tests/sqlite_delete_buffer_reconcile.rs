#![allow(clippy::field_reassign_with_default)]

//! `delete_wallet` must reconcile the in-memory buffer AND fold
//! buffered writes into the pre-delete backup.
//!
//! `delete_wallet_inner` drains the target wallet's buffered
//! changeset, flushes it to disk, snapshots the backup, then runs
//! the cascade. These regression tests pin the failure modes: a
//! buffered-only wallet must delete cleanly without spurious
//! `WalletNotFound`; the pre-delete backup must contain buffered-but-
//! unflushed rows; a transient pre-flush failure must restore the
//! buffer and abort the delete without producing a backup; the
//! flush must not resurrect a deleted wallet.

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
        wallet_group_id: [0u8; 32],
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

/// The pre-delete backup MUST include buffered writes flushed during
/// `delete_wallet`'s pre-flush phase. Without
/// the pre-flush, rollback-from-backup couldn't recover a wallet
/// whose only state lived in the buffer.
#[test]
fn pre_delete_backup_includes_buffered_writes() {
    let tmp = common::secure_tempdir().unwrap();
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
    let in_backup_core: Option<i64> = backup
        .query_row(
            "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    let in_backup_meta: Option<i64> = backup
        .query_row(
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(
        in_backup_core,
        Some(1),
        "pre-delete backup must contain the flushed buffered core_sync_state row"
    );
    assert_eq!(
        in_backup_meta,
        Some(1),
        "pre-delete backup must contain the flushed buffered wallets row"
    );
}

/// When the pre-flush fails, the buffer is restored, no backup is
/// produced, the wallet stays in the live DB, and
/// `delete_wallet` surfaces the original error.
#[test]
fn pre_flush_failure_preserves_buffer_and_skips_backup() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let backup_dir = tmp.path().join("backups");
    let cfg = SqlitePersisterConfig::new(&path)
        .with_flush_mode(FlushMode::Manual)
        .with_auto_backup_dir(Some(backup_dir.clone()));
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xC1);

    // Seed wallets so the wallet exists in the live DB.
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
    }

    // Buffer a changeset so `delete_wallet` enters the pre-flush
    // branch, then prime the pre-flush injector to fail.
    persister.store(w, full_changeset(11)).unwrap();
    persister.force_next_pre_flush_to_fail(busy_error());

    let err = persister
        .delete_wallet(w)
        .expect_err("pre-flush failure must propagate as Err");
    assert!(
        matches!(err, WalletStorageError::Sqlite(_)),
        "expected Sqlite error from primed pre-flush failure, got {err:?}"
    );

    // Backup dir holds no PreDelete file (dir may not even exist if
    // `run_auto_backup` never ran — both are acceptable).
    let entries: Vec<_> = std::fs::read_dir(&backup_dir)
        .map(|it| it.filter_map(Result::ok).collect())
        .unwrap_or_default();
    assert!(
        entries.is_empty(),
        "pre-flush failure must not leave a backup behind: {entries:?}"
    );

    // Wallet still in the live DB, buffer still holds the changeset.
    let meta_rows: i64 = {
        let conn = persister.lock_conn_for_test();
        conn.query_row(
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(meta_rows, 1, "wallet must remain in the live DB");
    assert!(
        persister.buffer_has_changeset_for_test(&w),
        "buffer must still hold the changeset after a failed pre-flush"
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
