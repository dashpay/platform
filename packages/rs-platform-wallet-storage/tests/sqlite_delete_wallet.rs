#![allow(clippy::field_reassign_with_default)]

//! CMT-002 — `delete_wallet` must restore the drained buffered
//! changeset on any pre-commit failure. Mirrors the take/restore
//! discipline `flush_inner` uses so the operator's pending writes
//! survive a failed delete.

mod common;

use std::path::PathBuf;
use std::sync::Arc;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{
    FlushMode, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};

/// Open a persister with Manual flush mode + an explicitly bad
/// auto-backup directory so `run_auto_backup` reliably fails.
fn persister_with_bad_backup_dir() -> (SqlitePersister, tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("wallet.db");
    let bad_dir = tmp.path().join("does-not-exist").join("nested");
    // Point auto_backup_dir at a path whose grand-parent doesn't exist
    // — write attempts fail without create-mode magic.
    std::fs::create_dir(tmp.path().join("does-not-exist")).unwrap();
    let cfg = SqlitePersisterConfig::new(&db)
        .with_flush_mode(FlushMode::Manual)
        .with_auto_backup_dir(Some(bad_dir));
    // Take the parent away so create_dir_all fails.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            tmp.path().join("does-not-exist"),
            std::fs::Permissions::from_mode(0o500),
        )
        .unwrap();
    }
    let persister = SqlitePersister::open(cfg).expect("open");
    (persister, tmp, db)
}

#[test]
fn delete_wallet_restores_buffer_on_backup_failure() {
    let (persister, _tmp, _path) = persister_with_bad_backup_dir();
    let w = wid(0x42);
    ensure_wallet_meta(&persister, &w);

    // Stage a buffered changeset on top of the persisted parent.
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(99),
        last_processed_height: Some(99),
        ..Default::default()
    });
    persister.store(w, cs).expect("store");

    // Delete fails because the auto-backup dir is unwritable.
    let err = persister
        .delete_wallet(w)
        .expect_err("delete must fail when auto-backup is unwritable");
    assert!(
        matches!(err, WalletStorageError::AutoBackupDirUnwritable { .. }),
        "expected AutoBackupDirUnwritable, got {err:?}"
    );

    // The buffer MUST still hold the staged changeset — flush it and
    // observe the height landed in core_sync_state.
    persister.flush(w).expect("flush after failed delete");

    let conn = persister.lock_conn_for_test();
    let h: i64 = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .expect("core_sync_state row should exist post-flush");
    assert_eq!(
        h, 99,
        "buffered changeset must survive a failed delete and flush cleanly"
    );
}

/// CMT-008: a concurrent `store()` racing against `delete_wallet` must
/// not leave persisted rows behind. We spawn a worker hammering
/// `store(wallet_id, cs)` while the main thread calls `delete_wallet`,
/// then commit any remaining buffered writes and assert every
/// per-wallet table holds zero rows for that wallet_id.
#[test]
fn concurrent_store_does_not_resurrect_deleted_wallet() {
    use platform_wallet_storage::sqlite::schema::{count_rows_for_wallet_sql, PER_WALLET_TABLES};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;

    let (persister, _tmp, _path) = fresh_persister();
    let persister = Arc::new(persister);
    let w = wid(0xCC);
    ensure_wallet_meta(&persister, &w);

    // Seed at least one persisted child row.
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(1),
        last_processed_height: Some(1),
        ..Default::default()
    });
    persister.store(w, cs).expect("seed store");

    let stop = Arc::new(AtomicBool::new(false));
    let worker = {
        let persister = Arc::clone(&persister);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut i = 2u32;
            while !stop.load(Ordering::Relaxed) {
                let mut cs = PlatformWalletChangeSet::default();
                cs.core = Some(CoreChangeSet {
                    synced_height: Some(i),
                    last_processed_height: Some(i),
                    ..Default::default()
                });
                // Race against delete — either succeeds or hits a
                // wallet-gone state. Both are acceptable: we only care
                // that no row survives the delete commit.
                let _ = persister.store(w, cs);
                i = i.wrapping_add(1);
                std::thread::yield_now();
            }
        })
    };

    // Give the worker a moment to land some racing stores.
    std::thread::sleep(std::time::Duration::from_millis(20));
    persister
        .delete_wallet(w)
        .expect("delete_wallet should succeed in concurrent-store regression");
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();

    // Drain any remaining buffered writes — these MUST also leave the
    // wallet at zero rows because delete_wallet wiped the buffer post-
    // commit (CMT-008 post-commit re-drain).
    let _ = persister.commit_writes();

    let conn = persister.lock_conn_for_test();
    for (table, scope) in PER_WALLET_TABLES {
        let n: i64 = conn
            .query_row(
                &count_rows_for_wallet_sql(table, *scope),
                rusqlite::params![w.as_slice()],
                |row| row.get(0),
            )
            .unwrap_or_else(|e| panic!("COUNT(*) query failed for table `{table}`: {e}"));
        assert_eq!(
            n, 0,
            "table `{table}` still has rows for deleted wallet — concurrent-store race regression"
        );
    }
}
