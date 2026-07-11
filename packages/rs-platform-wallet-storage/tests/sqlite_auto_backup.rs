#![allow(clippy::field_reassign_with_default)]

//! TC-050..TC-055 — automatic backups.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet_storage::{
    AutoBackupOperation, SqlitePersister, SqlitePersisterConfig, WalletStorageError,
};

/// TC-050: brand-new DB does NOT produce a pre-migration backup.
#[test]
fn tc050_brand_new_db_skips_pre_migration_backup() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let cfg = SqlitePersisterConfig::new(&path);
    let dir = cfg.auto_backup_dir.clone().unwrap();
    let _p = SqlitePersister::open(cfg).unwrap();
    if dir.exists() {
        let leftover = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("pre-migration"))
            .count();
        assert_eq!(
            leftover, 0,
            "fresh DB should not produce pre-migration backups"
        );
    }
}

/// TC-051: delete_wallet writes a pre-delete backup before deleting.
#[test]
fn tc051_pre_delete_backup_taken() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xE0);
    ensure_wallet_meta(&persister, &w);
    let report = persister.delete_wallet(w).expect("delete_wallet");
    let backup_path = report.backup_path.expect("backup path present");
    assert!(backup_path.exists(), "backup file does not exist on disk");
    let name = backup_path.file_name().unwrap().to_string_lossy();
    assert!(
        name.starts_with("pre-delete-") && name.ends_with(".db"),
        "unexpected pre-delete filename: {name}"
    );
}

/// TC-052: delete_wallet with auto_backup_dir = None returns AutoBackupDisabled.
#[test]
fn tc052_delete_wallet_auto_backup_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let cfg = SqlitePersisterConfig::new(&path).with_auto_backup_dir(None);
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xE1);
    ensure_wallet_meta(&persister, &w);
    let err = persister.delete_wallet(w);
    assert!(
        matches!(
            err,
            Err(WalletStorageError::AutoBackupDisabled {
                operation: AutoBackupOperation::DeleteWallet
            })
        ),
        "expected AutoBackupDisabled, got {err:?}"
    );
    // Rows for `w` should still be present.
    let conn = persister.lock_conn_for_test();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM wallet_metadata WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

/// TC-054 (partial): unwritable auto-backup dir surfaces AutoBackupDirUnwritable.
///
/// The failure is forced through a path whose parent is a regular file
/// (`<file>/sub`), so `create_dir_all` fails with `ENOTDIR`. Unlike a
/// `chmod 0o500` directory — which UID 0 bypasses — this is rejected
/// for every UID, making the assertion deterministic in root-running
/// CI containers.
#[test]
fn tc054_unwritable_auto_backup_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("w.db");
    let blocker = tmp.path().join("not-a-dir");
    std::fs::write(&blocker, b"regular file").unwrap();
    let unwritable = blocker.join("sub");

    let cfg = SqlitePersisterConfig::new(&path).with_auto_backup_dir(Some(unwritable));
    let persister = SqlitePersister::open(cfg).unwrap();
    let w = wid(0xE2);
    ensure_wallet_meta(&persister, &w);
    let err = persister.delete_wallet(w);
    assert!(
        matches!(err, Err(WalletStorageError::AutoBackupDirUnwritable { .. })),
        "expected AutoBackupDirUnwritable, got {err:?}"
    );
    // Wallet still intact.
    let conn = persister.lock_conn_for_test();
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM wallet_metadata WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

/// TC-055: auto-backups respect the same retention as manual backups.
#[test]
fn tc055_auto_backups_subject_to_retention() {
    let (persister, _tmp, _path) = fresh_persister();
    let dir = persister.config_for_test().auto_backup_dir.clone().unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    // Drop in five `pre-delete-*` fixture files.
    for i in 0..5 {
        let name = format!(
            "pre-delete-{}-{}.db",
            hex::encode([i; 32]),
            chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::hours(i as i64))
                .unwrap()
                .format("%Y%m%dT%H%M%SZ")
        );
        std::fs::write(dir.join(name), b"x").unwrap();
    }
    let report = persister
        .prune_backups(
            &dir,
            platform_wallet_storage::RetentionPolicy {
                keep_last_n: Some(2),
                max_age: None,
            },
        )
        .unwrap();
    assert_eq!(report.kept, 2);
    assert_eq!(report.removed.len(), 3);
}
