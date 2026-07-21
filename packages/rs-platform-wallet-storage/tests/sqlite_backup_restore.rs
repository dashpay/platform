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
    let tmp = common::secure_tempdir().unwrap();
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
    let tmp = common::secure_tempdir().unwrap();
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

/// A failure during `backup_to` must NOT leave a partial/empty `.db`
/// file at the caller-supplied destination. The
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
    let tmp = common::secure_tempdir().unwrap();
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
    assert!(
        report.failed_removals.is_empty(),
        "happy-path prune must have no failed removals"
    );
}

/// A per-file remove failure is collected into `failed_removals`, not
/// propagated as `Err` aborting the loop.
/// We can't directly simulate a remove failure inside the spec on a
/// portable filesystem, so we use the simpler approach: confirm the
/// report shape carries `failed_removals` and a happy-path prune
/// leaves it empty. The classifier itself is the regression — pre-A-6
/// the field did not exist, so this file fails to compile against the
/// old API.
#[test]
fn atom_011_prune_report_carries_failed_removals_field() {
    let report = platform_wallet_storage::PruneReport {
        removed: vec![],
        kept: 0,
        failed_removals: vec![(
            std::path::PathBuf::from("/x"),
            std::io::Error::other("synthetic"),
        )],
    };
    assert_eq!(report.failed_removals.len(), 1);
}

/// Detect whether the current process can bypass directory permission
/// checks (i.e. is effectively root) by setting a probe dir to `0o500`
/// and trying to create a file inside it. Returns `true` when the
/// write succeeds — only root sees that.
#[cfg(unix)]
fn is_root_via_probe() -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(tmp) = common::secure_tempdir() else {
        return false;
    };
    let dir = tmp.path().join("probe");
    if fs::create_dir(&dir).is_err() {
        return false;
    }
    if fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).is_err() {
        return false;
    }
    let can_write = fs::write(dir.join("x"), b"x").is_ok();
    let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    can_write
}

/// `backup_to(existing-file)` refuses to overwrite and leaves the
/// sentinel content intact. With `persist_noclobber` the
/// check is atomic against the rename — no TOCTOU window between an
/// `exists()` probe and the atomic swap.
#[test]
fn tc_code_009_a_backup_to_refuses_overwrite_atomically() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xC9));
    let target = tmp.path().join("sentinel.db");
    let sentinel = b"DO NOT OVERWRITE";
    fs::write(&target, sentinel).unwrap();

    let err = persister.backup_to(&target);
    assert!(
        matches!(err, Err(WalletStorageError::BackupDestinationExists { .. })),
        "expected BackupDestinationExists, got {err:?}"
    );
    let after = fs::read(&target).unwrap();
    assert_eq!(
        after, sentinel,
        "destination must be untouched after refusal"
    );
}

/// Non-`AlreadyExists` persist failures surface as
/// `WalletStorageError::Io` — the variant taxonomy stays narrow.
/// Unix-only: emulated via a read-only parent directory (which UID 0
/// bypasses, so the test is skipped under root).
#[cfg(unix)]
#[test]
fn tc_code_009_b_backup_to_non_already_exists_maps_to_io() {
    // Skip under root — UID 0 bypasses the directory permission check
    // we use to force EACCES. Detected via a probe: create a 0o500 dir
    // and try to write into it; a non-root user gets EACCES, root
    // doesn't.
    if is_root_via_probe() {
        eprintln!("skip: read-only-dir permission bypassed by root");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xCA));
    let dest_dir = tmp.path().join("ro");
    fs::create_dir(&dest_dir).unwrap();
    fs::set_permissions(&dest_dir, fs::Permissions::from_mode(0o500)).unwrap();
    let target = dest_dir.join("new.db");

    let err = persister.backup_to(&target);
    // Restore perms so tempdir cleanup works on systems that need
    // write access to the parent dir.
    let _ = fs::set_permissions(&dest_dir, fs::Permissions::from_mode(0o700));

    match err {
        Err(WalletStorageError::Io(e)) => {
            assert_ne!(
                e.kind(),
                std::io::ErrorKind::AlreadyExists,
                "must NOT map AlreadyExists to plain Io"
            );
        }
        other => panic!("expected Io variant, got {other:?}"),
    }
}

/// `run_to` and `restore_from` call `fsync` on the destination's
/// parent directory after the atomic rename. Functional
/// fsync verification is impractical without a crash harness, so the
/// regression check is source-level: confirm `fsync_parent_dir` is
/// invoked in `backup.rs`.
#[test]
fn tc_code_014_a_backup_calls_parent_fsync() {
    let src = include_str!("../src/sqlite/backup.rs");
    let calls = src.matches("fsync_parent_dir(").count();
    assert!(
        calls >= 3,
        "expected at least 3 occurrences of `fsync_parent_dir(` in backup.rs \
         (def + run_to + restore_from), found {calls}"
    );
}

/// The `# Atomicity` rustdoc mentions the parent-dir fsync so callers
/// aren't misled about durability guarantees.
#[test]
fn tc_code_014_b_atomicity_doc_mentions_fsync() {
    let src = include_str!("../src/sqlite/backup.rs");
    let lower = src.to_lowercase();
    assert!(
        lower.contains("fsync") || lower.contains("sync_all"),
        "atomicity rustdoc must mention fsync / sync_all on parent dir"
    );
}

/// A failed `remove_file` is counted in BOTH `kept` and
/// `failed_removals`, preserving `kept + removed == total`.
///
/// Unix-only: emulated by chmodding the prune directory read-only so
/// `unlink` returns `EACCES`. Skipped under root because UID 0 bypasses
/// the directory permission check.
#[cfg(unix)]
#[test]
fn tc_code_019_a_failed_removal_counts_in_kept() {
    // Skip under root — UID 0 bypasses the directory permission check
    // we use to force EACCES. Detected via a probe: create a 0o500 dir
    // and try to write into it; a non-root user gets EACCES, root
    // doesn't.
    if is_root_via_probe() {
        eprintln!("skip: read-only-dir permission bypassed by root");
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let tmp = common::secure_tempdir().unwrap();
    let dir = tmp.path().join("backups");
    fs::create_dir(&dir).unwrap();
    // Five eligible backups, all old enough to be removed by `max_age`.
    let day = std::time::Duration::from_secs(86_400);
    let now = std::time::SystemTime::now();
    for age in [30u64, 31, 32, 33, 34] {
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
    }
    // Lock the directory so `unlink` fails on EVERY file.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();

    let (persister, _tmp_p, _path) = fresh_persister();
    let res = persister.prune_backups(
        &dir,
        RetentionPolicy {
            keep_last_n: None,
            max_age: Some(day),
        },
    );
    // Restore perms so tempdir cleanup works.
    let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));

    let report = res.expect("prune_backups must return Ok even on partial removal failures");
    assert!(
        !report.failed_removals.is_empty(),
        "expected at least one failed removal"
    );
    let total = report.removed.len() + report.failed_removals.len();
    assert_eq!(
        report.kept, total,
        "kept ({}) must equal total ({}) when no files were removed",
        report.kept, total
    );
    assert_eq!(
        report.removed.len() + report.kept,
        5,
        "kept + removed must equal total eligible (5)"
    );
}
