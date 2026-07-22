//! Online backup, restore, and retention helpers.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::backup::Backup;
use rusqlite::Connection;

use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::persister::{PruneReport, RetentionPolicy};
use crate::sqlite::util::permissions::apply_secure_permissions;

/// Fsync `path`'s parent dir on Unix so the rename's dentry update is
/// durable across power loss (`persist` only fsyncs the file inode; the
/// dentry is journalled separately). No-op on non-Unix.
#[cfg(unix)]
fn fsync_parent_dir(path: &Path) -> Result<(), WalletStorageError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let dir = std::fs::File::open(parent)?;
            dir.sync_all()?;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
fn fsync_parent_dir(_path: &Path) -> Result<(), WalletStorageError> {
    Ok(())
}

/// Normalize an `open_conn` failure on a candidate source/staged file
/// to the typed [`WalletStorageError::SourceOpenFailed`]. A raw rusqlite
/// open error keeps its `#[source]`; any other variant (e.g. a future
/// FK assertion on a RW open) passes through unchanged.
fn map_source_open_err(err: WalletStorageError) -> WalletStorageError {
    match err {
        WalletStorageError::Sqlite(source) => WalletStorageError::SourceOpenFailed { source },
        other => other,
    }
}

/// Distinguishes auto-backup filenames.
#[derive(Debug, Clone, Copy)]
pub enum BackupKind {
    PreMigration { from: i32, to: i32 },
    PreDelete { wallet_id: WalletId },
    PreRestore,
}

/// Filename for `backup_to(directory)`.
pub fn manual_backup_filename() -> String {
    format!("wallet-{}.db", utc_timestamp())
}

/// Filename for an auto-backup.
pub fn auto_backup_filename(kind: BackupKind) -> String {
    let ts = utc_timestamp();
    match kind {
        BackupKind::PreMigration { from, to } => format!("pre-migration-{from}-to-{to}-{ts}.db"),
        BackupKind::PreDelete { wallet_id } => {
            format!("pre-delete-{}-{ts}.db", hex::encode(wallet_id))
        }
        BackupKind::PreRestore => format!("pre-restore-{ts}.db"),
    }
}

/// Take an online backup of `src` to `dest` via the page-stepping
/// `Backup::run_to_completion` API so writers aren't blocked.
///
/// # Atomicity
///
/// The copy is staged in a `NamedTempFile` next to `dest` and
/// `persist_noclobber`-ed over `dest` only on success, so a failure never
/// materialises a partial `.db`. A pre-existing `dest` is rejected
/// atomically (no TOCTOU window), and the parent dir is fsynced afterward.
pub fn run_to(src: &Connection, dest: &Path) -> Result<(), WalletStorageError> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // Stage in an unguessable temp file in the same dir; the same-FS
    // guarantee makes `persist` an atomic rename.
    let parent = dest.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)?;
    // chmod 0o600 BEFORE persist so the destination inherits owner-only
    // mode via the rename; chmod after would leave an observable window.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    let mut backup_conn =
        crate::sqlite::conn::open_conn(tmp.path(), crate::sqlite::conn::Access::ReadWrite)?;
    {
        let backup = Backup::new(src, &mut backup_conn)?;
        // 100 pages × 4 KiB = 400 KiB per step on default SQLite page size.
        backup.run_to_completion(100, Duration::from_millis(5), None)?;
    }
    // Close before persisting so SQLite flushes its WAL/SHM siblings
    // against the temp path; the rename then sweeps them away.
    drop(backup_conn);

    // Atomic check-and-rename with no TOCTOU window; `AlreadyExists` maps
    // to the typed `BackupDestinationExists` overwrite-refusal contract.
    tmp.persist_noclobber(dest).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            WalletStorageError::BackupDestinationExists {
                path: dest.to_path_buf(),
            }
        } else {
            WalletStorageError::Io(e.error)
        }
    })?;
    fsync_parent_dir(dest)?;
    // Re-tighten for non-Unix builds; no-op on Unix where the temp
    // already landed at 0o600.
    apply_secure_permissions(dest)?;
    Ok(())
}

/// Restore a `.db` backup over `dest_db_path`. The caller must guarantee
/// the destination is not held open by this process and owns the
/// pre-restore auto-backup gate.
///
/// # Atomicity
///
/// Validation runs against the source and again against the STAGED bytes,
/// under a SQLite-native `BEGIN EXCLUSIVE` on `dest_db_path` that blocks
/// every other SQLite peer (which advisory flock could not). The
/// store-generation token is rotated INTO the staged temp before the swap,
/// so the single commit point brings in the restored bytes and the fresh
/// token together — a peer never observes restored content carrying the
/// source's stale token. The staged temp is `persist`-ed as an atomic rename
/// only after all gates pass, and that rename is the commit point: if it
/// fails, the live DB and its WAL/SHM siblings are left untouched, so a failed
/// restore never strands the old DB without its WAL-committed state. The
/// now-stale WAL/SHM siblings are unlinked only AFTER the swap succeeds (so a
/// leftover `-wal` can't shadow the restored DB); the parent dir is fsynced
/// afterward. See the numbered steps in the body for the per-phase rationale.
///
/// # Lock-release-before-rename trade-off
///
/// The EXCLUSIVE lock is dropped just BEFORE the rename: SQLite holds a
/// kernel handle on the old inode while the lock conn is alive, and
/// holding it across the rename would point it at the unlinked inode and
/// can make the rename fail on some filesystems. The cost is a microsecond
/// window where a peer could write into the old inode the rename then
/// unlinks — its own write is lost, nothing escalates. Correct file-handle
/// semantics across the rename outweigh absolute lock coverage.
pub fn restore_from(dest_db_path: &Path, src_backup: &Path) -> Result<(), WalletStorageError> {
    let parent = dest_db_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    crate::parent_permissions::check_parent_perms(parent).map_err(|error| match error {
        crate::parent_permissions::ParentPermissionsError::Io(source) => {
            WalletStorageError::Io(source)
        }
        crate::parent_permissions::ParentPermissionsError::Insecure { mode } => {
            WalletStorageError::InsecureParentDir { mode }
        }
    })?;

    // 1. Cheap early-out: sniff integrity + schema-history + version +
    //    wallet-identity against the source so an incompatible input fails
    //    before we stream the whole file. The authoritative, TOCTOU-safe
    //    gate re-runs on the STAGED bytes (step 4).
    let src = crate::sqlite::conn::open_conn(src_backup, crate::sqlite::conn::Access::ReadOnly)
        .map_err(map_source_open_err)?;
    run_integrity_check(&src, |report| WalletStorageError::IntegrityCheckFailed {
        report,
    })?;
    if !crate::sqlite::migrations::has_schema_history(&src)? {
        return Err(WalletStorageError::SchemaHistoryMissing);
    }
    crate::sqlite::migrations::assert_schema_version_supported(&src)?;
    crate::sqlite::conn::assert_wallet_application_id(&src)?;
    crate::sqlite::migrations::assert_schema_history_well_formed(&src)?;
    drop(src);

    // 2. SQLite-native exclusion: `BEGIN EXCLUSIVE` on a short-lived
    //    writer conn blocks every other SQLite peer until it drops (which
    //    advisory flock could not — it doesn't interlock with SQLite). The
    //    conn is dropped before `persist` (see lock-release trade-off).
    let mut dest_lock_conn: Option<rusqlite::Connection> = if dest_db_path.exists() {
        let conn =
            crate::sqlite::conn::open_conn(dest_db_path, crate::sqlite::conn::Access::ReadWrite)?;
        // The destination has no persister yet (the persister is the
        // caller), so apply our own busy_timeout for a backoff window.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // BUSY after busy_timeout becomes `RestoreDestinationLocked` so
        // callers keep their existing branch.
        match conn.execute_batch("BEGIN EXCLUSIVE") {
            Ok(()) => Some(conn),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if matches!(
                    err.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                ) =>
            {
                return Err(WalletStorageError::RestoreDestinationLocked);
            }
            Err(other) => return Err(WalletStorageError::Sqlite(other)),
        }
    } else {
        None
    };

    // 3. Stage the source into a NamedTempFile in the destination's parent
    //    dir (unguessable name, no symlink-plant TOCTOU).
    let parent = dest_db_path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    let mut src_file = std::fs::File::open(src_backup)?;
    std::io::copy(&mut src_file, tmp.as_file_mut())?;
    tmp.as_file().sync_all()?;

    // 4. Re-validate the STAGED bytes before persisting: a torn
    //    `io::copy` that escaped `sync_all` would otherwise persist a
    //    corrupt DB, and the recheck failing just drops the temp. Bound to
    //    the staged bytes (not the source handle) so a swap during the
    //    restore window can't slip a forward-version or foreign DB through.
    {
        let staged =
            crate::sqlite::conn::open_conn(tmp.path(), crate::sqlite::conn::Access::ReadOnly)
                .map_err(map_source_open_err)?;
        run_integrity_check(&staged, |report| WalletStorageError::IntegrityCheckFailed {
            report,
        })?;
        if !crate::sqlite::migrations::has_schema_history(&staged)? {
            return Err(WalletStorageError::SchemaHistoryMissing);
        }
        crate::sqlite::migrations::assert_schema_version_supported(&staged)?;
        crate::sqlite::conn::assert_wallet_application_id(&staged)?;
        crate::sqlite::migrations::assert_schema_history_well_formed(&staged)?;
    }

    // 5. Regenerate the store-generation token INTO the staged temp, before
    //    the atomic rename, so the single commit point (step 8) swaps in the
    //    restored bytes and the rotated token together — there is no window
    //    where restored content is observable with the source's stale token.
    //    The staged DB is switched to DELETE journaling first so the UPDATE
    //    lands in the main file with no `-wal` frames stranded outside the
    //    rename; the reopened destination is forced back to its configured
    //    journal mode on its next open. A pre-V003 backup has no generation
    //    table; `regenerate_generation` is a no-op there and the token is
    //    (re)seeded on its later migration to V003.
    {
        let conn =
            crate::sqlite::conn::open_conn(tmp.path(), crate::sqlite::conn::Access::ReadWrite)?;
        conn.pragma_update(None, "journal_mode", "DELETE")?;
        crate::sqlite::schema::versions::regenerate_generation(&conn)?;
        drop(conn);
        // Durably flush the regenerated token before the rename commits it.
        tmp.as_file().sync_all()?;
    }

    // 6. chmod 0o600 on the temp BEFORE persist so the destination
    //    inherits owner-only mode via the rename (post-persist chmod could
    //    fail with the new DB already live).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    // 7. Release the EXCLUSIVE lock before the rename/unlinks: on Windows /
    //    some FUSE mounts `remove_file` on a still-open file returns
    //    `PermissionDenied`, and the rename window wants a clean close (see
    //    lock-release trade-off above).
    if let Some(conn) = dest_lock_conn.take() {
        let _ = conn.execute_batch("ROLLBACK");
        drop(conn);
    }

    // 8. Persist the staged DB atomically over the destination. The atomic
    //    rename is the single commit point: it swaps in both the restored
    //    bytes and the rotated generation token together. If it fails (disk
    //    full, EXDEV, perms) the live DB and its WAL/SHM siblings are left
    //    untouched, so a failed restore can never strand the old DB without
    //    its WAL-committed state. Sibling cleanup (step 9) runs only once the
    //    swap has succeeded.
    tmp.persist(dest_db_path)
        .map_err(|e| WalletStorageError::Io(e.error))?;

    // 9. Clear the now-stale WAL/SHM siblings AFTER the swap so a leftover
    //    `-wal` can't shadow the restored DB on the next open. Sibling paths
    //    use `OsString::push` so non-UTF-8 bytes round-trip; `NotFound` is a
    //    silent no-op. The lock conn was dropped in step 7 for cross-platform
    //    unlink semantics.
    if let Some(file_name) = dest_db_path.file_name() {
        for ext in ["-wal", "-shm"] {
            let mut sibling_name = file_name.to_os_string();
            sibling_name.push(ext);
            let sibling = dest_db_path.with_file_name(sibling_name);
            match std::fs::remove_file(&sibling) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(WalletStorageError::Io(e)),
            }
        }
    }

    // 10. Make the rename + unlink dentry updates durable.
    fsync_parent_dir(dest_db_path)?;

    // 11. Re-tighten perms (idempotent; SQLite may re-materialise -wal/-shm).
    apply_secure_permissions(dest_db_path)?;
    Ok(())
}

/// Run `PRAGMA integrity_check` and return `Ok(())` only on the single
/// row `"ok"`. Any other result becomes a typed `IntegrityCheckFailed` via
/// the caller-supplied builder; an underlying rusqlite error surfaces as
/// `IntegrityCheckRunFailed`. SQLite returns one row per detected problem
/// (default cap 100); all rows are `\n`-joined so the report carries every
/// diagnostic, not just the first.
pub(crate) fn run_integrity_check<F>(
    conn: &Connection,
    on_failure: F,
) -> Result<(), WalletStorageError>
where
    F: FnOnce(String) -> WalletStorageError,
{
    let mut stmt = conn
        .prepare("PRAGMA integrity_check")
        .map_err(|source| WalletStorageError::IntegrityCheckRunFailed { source })?;
    let mut rows: Vec<String> = Vec::new();
    let mut trailing_err: Option<rusqlite::Error> = None;
    let iter = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|source| WalletStorageError::IntegrityCheckRunFailed { source })?;
    for item in iter {
        match item {
            Ok(s) => rows.push(s),
            Err(e) => {
                // SQLite can surface a `DatabaseCorrupt` partway through
                // the stream; treat it as end-of-stream when we already
                // have diagnostic rows, else surface it below.
                trailing_err = Some(e);
                break;
            }
        }
    }
    if rows.is_empty() {
        if let Some(source) = trailing_err {
            return Err(WalletStorageError::IntegrityCheckRunFailed { source });
        }
        // Empty result with no error is unexpected but not "ok".
        return Err(on_failure(String::new()));
    }
    if rows.len() == 1 && rows[0] == "ok" && trailing_err.is_none() {
        Ok(())
    } else {
        let mut report = rows.join("\n");
        if let Some(e) = trailing_err {
            // Preserve the cut-off marker so operators see the stream
            // was truncated, not just under-reported.
            report.push_str(&format!("\n[integrity_check stream aborted: {e}]"));
        }
        Err(on_failure(report))
    }
}

/// Apply retention to a directory. Files that match the recognised
/// backup-name prefixes are eligible; others are ignored.
///
/// # Partial failures
///
/// Per-file `remove_file` failures are collected into
/// `PruneReport::failed_removals` rather than aborting the loop. The
/// happy path still removes every eligible file. Only catastrophic
/// errors (`read_dir` itself fails, an `entry?` returns Err) surface
/// as `Err(_)` — those affect every subsequent iteration too, so
/// continuing would just compound the failure.
pub fn prune(dir: &Path, policy: RetentionPolicy) -> Result<PruneReport, WalletStorageError> {
    let entries = std::fs::read_dir(dir)?;
    let mut files: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !is_backup_file(&path) {
            continue;
        }
        let ts = backup_timestamp(&path).unwrap_or_else(|| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        files.push((ts, path));
    }
    // Newest first.
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let now = SystemTime::now();
    let mut removed = Vec::new();
    let mut failed_removals: Vec<(PathBuf, std::io::Error)> = Vec::new();
    let mut kept = 0;
    for (idx, (ts, path)) in files.into_iter().enumerate() {
        // `keep_last_n` is a FLOOR: the N newest are always kept. `max_age` is
        // an independent age window. A file is kept if it satisfies EITHER
        // policy (the union), and removed only when it fails BOTH — so a
        // within-age file beyond the N newest is still kept (the bug fix: the
        // count must not cap the age window). With no policy set at all (both
        // `None`) every file is kept.
        let count_keep = matches!(policy.keep_last_n, Some(n) if idx < n);
        let age_keep = match policy.max_age {
            Some(max) => now.duration_since(ts).map(|d| d <= max).unwrap_or(true),
            None => false,
        };
        let no_policy = policy.keep_last_n.is_none() && policy.max_age.is_none();
        if no_policy || count_keep || age_keep {
            kept += 1;
        } else {
            match std::fs::remove_file(&path) {
                Ok(()) => removed.push(path),
                Err(e) => {
                    // A failed removal leaves the file on disk, so count it
                    // as kept to preserve `kept + removed == total`.
                    failed_removals.push((path, e));
                    kept += 1;
                }
            }
        }
    }
    // Sort `removed` oldest-first for deterministic output.
    removed.sort();
    failed_removals.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(PruneReport {
        removed,
        kept,
        failed_removals,
    })
}

fn is_backup_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    (name.starts_with("wallet-")
        || name.starts_with("pre-migration-")
        || name.starts_with("pre-delete-")
        || name.starts_with("pre-restore-"))
        && name.ends_with(".db")
}

fn backup_timestamp(path: &Path) -> Option<SystemTime> {
    let name = path.file_name()?.to_str()?;
    // Find the last `YYYYMMDDTHHMMSSZ` token before `.db`.
    let stem = name.strip_suffix(".db")?;
    let token = stem.rsplit('-').next()?;
    parse_compact_timestamp(token)
}

fn parse_compact_timestamp(s: &str) -> Option<SystemTime> {
    // Expect 16 chars: `YYYYMMDDTHHMMSSZ`.
    if s.len() != 16 {
        return None;
    }
    let year: i32 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(4..6)?.parse().ok()?;
    let day: u32 = s.get(6..8)?.parse().ok()?;
    if s.as_bytes().get(8) != Some(&b'T') {
        return None;
    }
    let hour: u32 = s.get(9..11)?.parse().ok()?;
    let minute: u32 = s.get(11..13)?.parse().ok()?;
    let second: u32 = s.get(13..15)?.parse().ok()?;
    if s.as_bytes().get(15) != Some(&b'Z') {
        return None;
    }
    use chrono::{TimeZone, Utc};
    let dt = Utc
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(dt.timestamp().max(0) as u64))
}

fn utc_timestamp() -> String {
    chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_backup_filename_matches_regex() {
        let n = manual_backup_filename();
        assert!(n.starts_with("wallet-"));
        assert!(n.ends_with(".db"));
        assert_eq!(n.len(), "wallet-YYYYMMDDTHHMMSSZ.db".len());
    }

    #[test]
    fn timestamp_roundtrip() {
        let ts = parse_compact_timestamp("20260101T000000Z").unwrap();
        let secs = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
        // 2026-01-01 00:00:00 UTC = 1767225600
        assert_eq!(secs, 1767225600);
    }

    /// `backup_timestamp` must extract the embedded timestamp (not fall
    /// back to mtime) for every `BackupKind` shape, including ones with
    /// inner `-`. Guards the `rsplit('-')` coupling against a future label
    /// that shifts the trailing token.
    #[test]
    fn backup_timestamp_extracts_embedded_token_for_all_kinds() {
        let want = parse_compact_timestamp("20260101T000000Z").unwrap();
        let real_wallet_id = hex::encode([0xABu8; 32]);
        let names = [
            "wallet-20260101T000000Z.db".to_string(),
            // Multiple `-` from the from/to version segments.
            "pre-migration-1-to-2-20260101T000000Z.db".to_string(),
            // 64 lowercase hex chars: hex::encode never emits `-`, so the
            // timestamp stays the last `-`-delimited token.
            format!("pre-delete-{real_wallet_id}-20260101T000000Z.db"),
            "pre-restore-20260101T000000Z.db".to_string(),
        ];
        for name in names {
            let got = backup_timestamp(Path::new(&name));
            assert_eq!(
                got,
                Some(want),
                "backup_timestamp must parse the embedded token, not fall back to mtime, for {name}"
            );
        }
    }

    /// A label with a trailing non-timestamp segment must return `None`
    /// (prune falls back to mtime) rather than misread a wrong token as a
    /// valid time — a detectable regression if a future `BackupKind`
    /// appends a `-`-bearing suffix after the timestamp.
    #[test]
    fn backup_timestamp_rejects_trailing_non_timestamp_segment() {
        assert_eq!(
            backup_timestamp(Path::new("pre-delete-20260101T000000Z-label.db")),
            None,
            "a trailing non-timestamp segment must not parse as a timestamp"
        );
    }

    #[test]
    fn is_backup_file_recognises_prefixes() {
        assert!(is_backup_file(Path::new("/tmp/wallet-20260101T000000Z.db")));
        assert!(is_backup_file(Path::new(
            "/tmp/pre-migration-1-to-2-20260101T000000Z.db"
        )));
        assert!(is_backup_file(Path::new(
            "/tmp/pre-delete-abcd-20260101T000000Z.db"
        )));
        assert!(is_backup_file(Path::new(
            "/tmp/pre-restore-20260101T000000Z.db"
        )));
        assert!(!is_backup_file(Path::new("/tmp/notes.txt")));
        assert!(!is_backup_file(Path::new("/tmp/wallet.db")));
    }
}
