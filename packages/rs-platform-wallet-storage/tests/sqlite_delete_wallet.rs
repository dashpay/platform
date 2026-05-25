#![allow(clippy::field_reassign_with_default)]

//! CMT-002 — `delete_wallet` must restore the drained buffered
//! changeset on any pre-commit failure. Mirrors the take/restore
//! discipline `flush_inner` uses so the operator's pending writes
//! survive a failed delete.

mod common;

use std::path::PathBuf;

use common::{ensure_wallet_meta, wid};
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
    let perms = std::fs::Permissions::from(std::fs::metadata(tmp.path()).unwrap().permissions());
    let _ = perms; // keep clippy quiet
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
