#![allow(clippy::field_reassign_with_default)]

//! TC-031..TC-039 — online backup, restore source validation, retention.

mod common;

use std::fs;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{RetentionPolicy, SqlitePersister, WalletStorageError};

fn seed_one_row(persister: &SqlitePersister, w: &[u8; 32]) {
    ensure_wallet_meta(persister, w);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(5),
        last_processed_height: Some(5),
        ..Default::default()
    });
    persister.store(*w, cs).unwrap();
}

/// TC-031: backup_to(directory) produces a wallet-<ts>.db file.
#[test]
fn tc031_backup_directory_form() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xD0));
    let out_dir = tmp.path().join("backups");
    fs::create_dir(&out_dir).unwrap();
    let written = persister.backup_to(&out_dir).expect("backup_to");
    // `backup_to` canonicalizes its return; canonicalize the expected
    // dir too so the comparison is symmetric. On macOS the temp dir
    // lives under `/var` (a symlink to `/private/var`), so an
    // un-canonicalized `out_dir` would not prefix the canonical path.
    let expected_dir = out_dir.canonicalize().unwrap_or_else(|_| out_dir.clone());
    assert!(written.starts_with(&expected_dir));
    let name = written.file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.starts_with("wallet-") && name.ends_with(".db"));
    // Open the produced file and confirm it has the schema.
    let src =
        rusqlite::Connection::open_with_flags(&written, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let check: String = src
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(check, "ok");
}

/// TC-032: backup_to(explicit file path) writes to the exact path.
#[test]
fn tc032_backup_file_form() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xD1));
    let target = tmp.path().join("explicit-name.db");
    let written = persister.backup_to(&target).unwrap();
    assert_eq!(written, target.canonicalize().unwrap_or(target.clone()));
    assert!(target.exists());
    // Refuses overwrite.
    let err = persister.backup_to(&target);
    assert!(
        matches!(err, Err(WalletStorageError::BackupDestinationExists { .. })),
        "expected BackupDestinationExists, got {err:?}"
    );
}

/// TC-035 (subset): restore_from round-trips state via the on-disk backup.
#[test]
fn tc035_restore_roundtrip() {
    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xD2);
    seed_one_row(&persister, &w);
    // Take a backup.
    let backup_path = persister.backup_to(tmp.path()).unwrap();
    // Mutate the source — make synced_height a different value.
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(999),
        last_processed_height: Some(999),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    drop(persister);
    // Restore.
    // Tests pass through `restore_from_skip_backup` — simpler than
    // threading an auto_backup_dir through fixtures.
    SqlitePersister::restore_from_skip_backup(&path, &backup_path).expect("restore_from");
    // Reopen and check the synced height reverted to 5.
    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&path);
    let p2 = SqlitePersister::open(cfg).unwrap();
    let conn = p2.lock_conn_for_test();
    let h: i64 = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(h, 5);
}

/// TC-036: restore source missing schema_history is rejected.
#[test]
fn tc036_restore_missing_schema_history() {
    let tmp = tempfile::tempdir().unwrap();
    let fake_src = tmp.path().join("empty.db");
    rusqlite::Connection::open(&fake_src).unwrap();
    let dest = tmp.path().join("dest.db");
    fs::write(&dest, b"placeholder").unwrap();
    let err = SqlitePersister::restore_from_skip_backup(&dest, &fake_src);
    assert!(matches!(err, Err(WalletStorageError::SchemaHistoryMissing)));
}

/// TC-037: corrupt source rejected.
#[test]
fn tc037_restore_corrupt_source() {
    let tmp = tempfile::tempdir().unwrap();
    let corrupt = tmp.path().join("corrupt.db");
    fs::write(&corrupt, b"not a sqlite file ABCDEF").unwrap();
    let dest = tmp.path().join("dest.db");
    fs::write(&dest, b"placeholder").unwrap();
    let err = SqlitePersister::restore_from_skip_backup(&dest, &corrupt);
    assert!(
        matches!(
            err,
            Err(WalletStorageError::IntegrityCheckFailed { .. })
                | Err(WalletStorageError::IntegrityCheckRunFailed { .. })
                | Err(WalletStorageError::SourceOpenFailed { .. })
                | Err(WalletStorageError::Sqlite(_))
        ),
        "expected IntegrityCheckFailed / IntegrityCheckRunFailed / SourceOpenFailed / Sqlite, got {err:?}"
    );
}

/// ATOM-004 (A-1): a failure during `backup_to` must NOT leave a
/// partial/empty `.db` file at the caller-supplied destination. The
/// pre-A-1 code eagerly opened `dest`, so any later failure
/// (`apply_secure_permissions`, `Backup::new`, `run_to_completion`)
/// stranded an empty file at `dest`. We exercise the path that
/// already-rejects (pre-existing destination) — the new code's exists
/// check fires before any temp file gets created.
#[test]
fn atom_004_backup_to_failure_leaves_no_junk_at_dest() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xE7));
    // First backup succeeds.
    let target = tmp.path().join("first.db");
    persister.backup_to(&target).expect("first backup");
    // Second backup to same path fails fast — no auxiliary `.tmp*`
    // file should remain alongside the existing target.
    let err = persister.backup_to(&target);
    assert!(matches!(
        err,
        Err(WalletStorageError::BackupDestinationExists { .. })
    ));
    // Sanity: only the legitimate file is present, plus -journal/-wal
    // siblings SQLite may have created on the live persister DB —
    // those live in a different parent so this scan is clean.
    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    let leaked: Vec<_> = entries
        .iter()
        .filter(|n| n.starts_with(".tmp") || n.ends_with(".tmp"))
        .collect();
    assert!(
        leaked.is_empty(),
        "no .tmp* staging file should remain in {:?}; found {leaked:?}",
        tmp.path()
    );
}

/// TC-038: prune retention AND-semantics.
#[test]
fn tc038_prune_and_semantics() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    // Write 5 fake backup files with mtimes 1d/7d/14d/30d/60d ago.
    let day = std::time::Duration::from_secs(86_400);
    let now = std::time::SystemTime::now();
    let ages = [1u64, 7, 14, 30, 60];
    let mut files = Vec::new();
    for age in ages {
        let name = format!(
            "wallet-{}.db",
            chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::days(age as i64))
                .unwrap()
                .format("%Y%m%dT%H%M%SZ")
        );
        let path = dir.join(&name);
        fs::write(&path, b"x").unwrap();
        let mtime = now - day * age as u32;
        let _ = filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(mtime));
        files.push(path);
    }
    let (persister, _tmp_pers, _path) = fresh_persister();
    let report = persister
        .prune_backups(
            dir,
            RetentionPolicy {
                keep_last_n: Some(3),
                max_age: Some(day * 20),
            },
        )
        .unwrap();
    // Files with ages 30d and 60d (older than 20d) should be removed.
    assert_eq!(report.removed.len(), 2);
    assert_eq!(report.kept, 3);
}
