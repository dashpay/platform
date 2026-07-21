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
    let tmp = common::secure_tempdir().unwrap();
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
    let tmp = common::secure_tempdir().unwrap();
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
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
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
    let tmp = common::secure_tempdir().unwrap();
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
            "SELECT COUNT(*) FROM wallets WHERE wallet_id = ?1",
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

/// Prune orders by the EMBEDDED filename timestamp, not mtime (proven by
/// giving older files newer mtimes). With `keep_last_n = 1` it evicts even
/// a pre-delete safety backup when that backup is not the newest by
/// embedded timestamp: the auto dir is not a protected vault, so operators
/// must size retention above the rollback horizon they care about.
#[test]
fn tc056_aggressive_prune_evicts_safety_backup_and_orders_by_embedded_ts() {
    let (persister, _tmp, _path) = fresh_persister();
    let dir = persister.config_for_test().auto_backup_dir.clone().unwrap();
    std::fs::create_dir_all(&dir).unwrap();

    let stamp = |hours_ago: i64| {
        chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::hours(hours_ago))
            .unwrap()
            .format("%Y%m%dT%H%M%SZ")
            .to_string()
    };

    // Newest by embedded timestamp: a manual backup taken AFTER the
    // delete. The pre-delete safety backup is older by embedded ts.
    let manual = dir.join(format!("wallet-{}.db", stamp(0)));
    let safety = dir.join(format!(
        "pre-delete-{}-{}.db",
        hex::encode([0x11u8; 32]),
        stamp(1)
    ));
    let old_manual = dir.join(format!("wallet-{}.db", stamp(48)));
    std::fs::write(&manual, b"m").unwrap();
    std::fs::write(&safety, b"s").unwrap();
    std::fs::write(&old_manual, b"o").unwrap();

    // Invert mtime vs embedded order: give the OLDEST-by-embedded-ts
    // file the NEWEST mtime. If prune (wrongly) sorted by mtime, it
    // would keep `old_manual`; sorting by the embedded token keeps
    // `manual`. This deterministically exercises the embedded-timestamp
    // path rather than the mtime fallback.
    let now = std::time::SystemTime::now();
    let hour = std::time::Duration::from_secs(3600);
    filetime::set_file_mtime(&old_manual, filetime::FileTime::from_system_time(now)).unwrap();
    filetime::set_file_mtime(&safety, filetime::FileTime::from_system_time(now - hour)).unwrap();
    filetime::set_file_mtime(
        &manual,
        filetime::FileTime::from_system_time(now - hour * 2),
    )
    .unwrap();

    let report = persister
        .prune_backups(
            &dir,
            platform_wallet_storage::RetentionPolicy {
                keep_last_n: Some(1),
                max_age: None,
            },
        )
        .unwrap();

    assert_eq!(report.kept, 1, "keep_last_n = 1 keeps exactly one file");
    assert_eq!(report.removed.len(), 2);
    // Embedded-ts ordering kept the newest-by-token file (`manual`),
    // NOT the newest-by-mtime file (`old_manual`).
    assert!(
        manual.exists(),
        "newest-by-embedded-timestamp file must survive keep_last_n = 1"
    );
    assert!(
        !old_manual.exists(),
        "an old file with a fresh mtime must NOT be treated as newest"
    );
    // The safety backup is NOT special-cased: aggressive retention
    // evicts it. Operators must size retention above the rollback
    // horizon they care about.
    assert!(
        !safety.exists(),
        "pre-delete safety backup is evicted by keep_last_n = 1 when not newest \
         (auto dir is not a protected vault)"
    );
}

/// `keep_last_n` is a FLOOR, not a ceiling: with both `keep_last_n` and
/// `max_age` set, a file beyond the N newest but still within `max_age` must
/// be KEPT (the union of the two policies), and only files failing BOTH are
/// evicted. Regression guard for the count-caps-the-age-window bug.
#[test]
fn keep_last_n_is_a_floor_not_a_ceiling_with_max_age() {
    let (persister, _tmp, _path) = fresh_persister();
    let dir = persister.config_for_test().auto_backup_dir.clone().unwrap();
    std::fs::create_dir_all(&dir).unwrap();

    let stamp = |hours_ago: i64| {
        chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::hours(hours_ago))
            .unwrap()
            .format("%Y%m%dT%H%M%SZ")
            .to_string()
    };
    // Newest (0h) kept by the floor; the 1h file is beyond the floor but
    // within the 2h window (kept by age); the 48h file fails both (evicted).
    let newest = dir.join(format!("wallet-{}.db", stamp(0)));
    let within_age = dir.join(format!("wallet-{}.db", stamp(1)));
    let too_old = dir.join(format!("wallet-{}.db", stamp(48)));
    for p in [&newest, &within_age, &too_old] {
        std::fs::write(p, b"x").unwrap();
    }

    let report = persister
        .prune_backups(
            &dir,
            platform_wallet_storage::RetentionPolicy {
                keep_last_n: Some(1),
                max_age: Some(std::time::Duration::from_secs(2 * 3600)),
            },
        )
        .unwrap();

    assert_eq!(
        report.kept, 2,
        "floor (1) + within-age (1) must both survive"
    );
    assert_eq!(report.removed.len(), 1);
    assert!(newest.exists(), "the newest file is kept by the floor");
    assert!(
        within_age.exists(),
        "a within-max_age file beyond the floor must NOT be evicted by the count"
    );
    assert!(!too_old.exists(), "a file failing both policies is evicted");
}
