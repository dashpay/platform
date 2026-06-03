//! Online backup, restore, and retention helpers.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::backup::Backup;
use rusqlite::Connection;

use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::persister::{PruneReport, RetentionPolicy};
use crate::sqlite::util::permissions::apply_secure_permissions;

/// Fsync the parent directory of `path` on Unix so the rename entry
/// that materialised `path` is durable across power loss.
/// `persist` only fsyncs the file inode; on most Unix filesystems the
/// dentry update is journalled separately and can be lost on crash
/// without this step. No-op on non-Unix platforms.
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

/// Take an online backup of `src` to `dest`. Uses the
/// `rusqlite::backup::Backup::run_to_completion` page-stepping API
/// so writers aren't blocked.
///
/// # Atomicity
///
/// The page-stepping copy runs against a `NamedTempFile` staged in
/// `dest`'s parent directory. The temp is `persist_noclobber`-ed over
/// `dest` only on success — any failure (open, chmod, backup-stream)
/// drops the temp without ever materialising a partial `.db` file at
/// the caller's path. A pre-existing `dest` is rejected atomically by
/// `persist_noclobber` (no TOCTOU window). On Unix, the parent
/// directory is `fsync`-ed after the rename so the dentry update
/// survives power loss; on non-Unix this fsync step is a no-op.
pub fn run_to(src: &Connection, dest: &Path) -> Result<(), WalletStorageError> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // Pre-existing-destination rejection happens at the
    // `persist_noclobber` site below — that's atomic against the rename
    // (no TOCTOU window between `dest.exists()` and persist). The
    // CLI's `backup_to(file_path)` still gets the typed
    // `BackupDestinationExists` error; auto-backup callers can't trip
    // it because the filename carries a unique timestamp suffix.

    // Stage the backup into an unguessable temp file in the same
    // directory. Same-FS guarantee makes `persist` an atomic rename.
    let parent = dest.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)?;
    // Tighten the temp's mode to 0o600 BEFORE persist so the
    // destination inherits owner-only permissions via the atomic
    // rename. Running chmod after persist would leave a brief
    // umask-default window where the destination is observable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    // Page-stepping copy against the temp. The dest Connection has to
    // own its own file handle; rusqlite opens it from a path.
    let mut backup_conn =
        crate::sqlite::conn::open_conn(tmp.path(), crate::sqlite::conn::Access::ReadWrite)?;
    {
        let backup = Backup::new(src, &mut backup_conn)?;
        // 100 pages × 4 KiB = 400 KiB per step on default SQLite page size.
        backup.run_to_completion(100, Duration::from_millis(5), None)?;
    }
    // Close the backup Connection before persisting so SQLite flushes
    // its own WAL/SHM siblings against the temp path — those go away
    // with the rename since `persist` atomically renames the temp file.
    drop(backup_conn);

    // `persist_noclobber` is the atomic check-and-rename — SQLite-free,
    // no TOCTOU window between an `exists()` probe and the rename.
    // `AlreadyExists` maps to the typed `BackupDestinationExists` for
    // the CLI's overwrite-refusal contract.
    tmp.persist_noclobber(dest).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            WalletStorageError::BackupDestinationExists {
                path: dest.to_path_buf(),
            }
        } else {
            WalletStorageError::Io(e.error)
        }
    })?;
    // Fsync the parent directory so the atomic rename's dentry update is
    // durable across power loss. On non-Unix this is a no-op.
    fsync_parent_dir(dest)?;
    // Re-tighten in case a non-Unix build (or a future platform-specific
    // tweak) needs to refresh sibling perms after SQLite materialised
    // them. No-op on Unix where the temp already landed at 0o600.
    apply_secure_permissions(dest)?;
    Ok(())
}

/// Restore a `.db` backup over `dest_db_path`. Associated function;
/// caller must guarantee the destination is not held open by this
/// process. The caller (the persister's `restore_from_inner`) handles
/// the pre-restore auto-backup gate.
///
/// # Atomicity
///
/// The restore is staged in two phases bounded by a SQLite-native
/// `BEGIN EXCLUSIVE` transaction on `dest_db_path` (kept across the
/// entire restore body):
///
/// 1. Open the source read-only; run `PRAGMA integrity_check` +
///    schema-history + max-version sniffs. Any failure here aborts
///    before the live destination is touched.
/// 2. Open a short-lived writer connection on the destination and
///    `BEGIN EXCLUSIVE`. This blocks every other SQLite peer
///    (other `SqlitePersister` handles in this or sibling processes,
///    bare `rusqlite::Connection`s, the CLI) from writing the file
///    until restore completes. Peers waiting for the lock back off
///    via SQLite's own busy_timeout. The lock conn is DROPPED right
///    before `persist` so SQLite releases its file handle on the old
///    inode before the atomic rename takes its place.
/// 3. Stream the source into a `NamedTempFile` in `dest_db_path`'s
///    parent directory; re-run integrity + schema gates against the
///    STAGED bytes (catches a torn `io::copy`); unlink the existing
///    `-wal` / `-shm` siblings; chmod the temp to 0o600; then
///    `persist` over `dest_db_path` as an atomic rename.
///
/// Either both the main DB and its WAL/SHM siblings are replaced, or
/// — on any pre-persist failure — none of them are touched. The
/// SQLite-native lock prevents a racing peer from committing rows
/// between the staged validation and the rename, which the prior
/// flock-based approach could not do (flock doesn't see SQLite peers).
///
/// On Unix, the parent directory is `fsync`-ed after the rename so the
/// dentry update is durable across power loss; on non-Unix this is a
/// no-op.
///
/// # Lock-release-before-rename trade-off
///
/// The EXCLUSIVE lock is released BEFORE the atomic rename, on
/// purpose. SQLite keeps a kernel file handle on the destination's
/// (old) inode for as long as the lock conn is alive; holding that
/// handle across the rename would leave it pointing at the unlinked
/// old inode while peers opening the new path would race the rename
/// itself (on some filesystems the rename can outright fail).
/// Releasing the lock first lets SQLite drop its old-inode handle
/// before the rename swaps it.
///
/// The trade-off: a microsecond window opens between lock release and
/// rename in which a peer can acquire its own SQLite lock on the
/// destination's old inode. Any writes it makes within that window
/// land in the old inode, which the rename immediately unlinks — the
/// peer's writes are effectively dropped on the floor (the peer keeps
/// a handle on an inode that no longer has any directory entry; once
/// it closes, the bytes are reclaimed). That is acceptable for the
/// restore contract: callers serialize their own restore intent at
/// the application layer; the window is too short for a non-malicious
/// peer to land more than a transient miss, and a malicious peer
/// cannot escalate beyond losing its own write. Correct file-handle
/// semantics across the rename matter more than absolute lock
/// coverage.
pub fn restore_from(dest_db_path: &Path, src_backup: &Path) -> Result<(), WalletStorageError> {
    // 1. Confirm the source is openable, then run cheap pre-staging
    //    integrity + schema-history + max-version sniffs against the
    //    source itself so an obviously-incompatible input fails before
    //    we stream the whole file into the destination's partition.
    //    The authoritative schema-history / version gate still re-runs
    //    on the STAGED copy (step 4) — that's the TOCTOU-safe check
    //    bound to the exact bytes about to be persisted.
    let src = crate::sqlite::conn::open_conn(src_backup, crate::sqlite::conn::Access::ReadOnly)
        .map_err(map_source_open_err)?;
    run_integrity_check(&src, |report| WalletStorageError::IntegrityCheckFailed {
        report,
    })?;
    if !crate::sqlite::migrations::has_schema_history(&src)? {
        return Err(WalletStorageError::SchemaHistoryMissing);
    }
    crate::sqlite::migrations::assert_schema_version_supported(&src)?;
    drop(src);

    // 2. SQLite-native exclusion. `BEGIN EXCLUSIVE` against a short-
    //    lived writer connection on the destination blocks every other
    //    SQLite peer (rusqlite Connection, sibling `SqlitePersister`)
    //    until the tx is committed/rolled-back or the conn drops. The
    //    prior flock approach was a false promise: advisory locks
    //    don't interlock with SQLite's own locking, so a peer mid-write
    //    could race the swap. The lock conn is dropped (`take()` + end
    //    of scope) BEFORE `tmp.persist` so SQLite releases its file
    //    handle on the old inode before the atomic rename — otherwise
    //    we'd leave a dangling handle on the unlinked inode.
    let mut dest_lock_conn: Option<rusqlite::Connection> = if dest_db_path.exists() {
        let conn =
            crate::sqlite::conn::open_conn(dest_db_path, crate::sqlite::conn::Access::ReadWrite)?;
        // Reuse a sensible busy_timeout so peers don't immediately
        // surface BUSY without a backoff window. The destination DB
        // may not have a persister attached yet (the persister is the
        // CALLER), so this conn applies its own.
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // Take EXCLUSIVE up-front by promoting an immediate tx. If a
        // peer holds the DB, SQLite waits for busy_timeout then
        // returns BUSY — we surface that as `RestoreDestinationLocked`
        // so callers keep their existing branch.
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

    // 3. Stage the source into a NamedTempFile in the destination's
    //    parent dir (unguessable name, no symlink-plant TOCTOU).
    let parent = dest_db_path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    let mut src_file = std::fs::File::open(src_backup)?;
    std::io::copy(&mut src_file, tmp.as_file_mut())?;
    tmp.as_file().sync_all()?;

    // 4. Re-run integrity_check on the STAGED file before
    //    persisting. A torn `std::io::copy` or transient FS error
    //    that escaped `sync_all`'s notice would otherwise persist a
    //    corrupted database. If the recheck fails, the temp file
    //    drops naturally and the live destination stays untouched.
    {
        let staged =
            crate::sqlite::conn::open_conn(tmp.path(), crate::sqlite::conn::Access::ReadOnly)
                .map_err(map_source_open_err)?;
        run_integrity_check(&staged, |report| WalletStorageError::IntegrityCheckFailed {
            report,
        })?;
        // Schema-history presence + max-version gate, bound to the
        // staged bytes (not the first source handle) so a swap during
        // the restore window can't slip a forward-version DB through.
        if !crate::sqlite::migrations::has_schema_history(&staged)? {
            return Err(WalletStorageError::SchemaHistoryMissing);
        }
        crate::sqlite::migrations::assert_schema_version_supported(&staged)?;
    }

    // 5. chmod 600 on the temp BEFORE persist so the destination
    //    inherits owner-only mode via the atomic rename. Chmodding
    //    post-persist would leave the new DB live at the destination on
    //    a chmod failure, contradicting the rolled-back error.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    // 6. Release the SQLite-native EXCLUSIVE lock BEFORE touching the
    //    on-disk WAL/SHM siblings or running the rename. On Windows /
    //    some FUSE / AV-scanned mounts, `remove_file` against a file
    //    still held open by another handle on the same process returns
    //    `PermissionDenied`; on Unix the unlinked inodes remain
    //    reachable through the open fd but the rename window still
    //    benefits from a clean close.
    if let Some(conn) = dest_lock_conn.take() {
        // Best-effort rollback of the empty EXCLUSIVE tx; an error here
        // means SQLite is already in trouble and `drop(conn)` covers
        // the rest. Silent because the conn is about to drop anyway.
        let _ = conn.execute_batch("ROLLBACK");
        drop(conn);
    }

    // 7. Atomicity gate: every staged-file validation has now passed
    //    and our writer handle is closed, so it's safe to clear WAL/SHM
    //    siblings the replaced DB might have left behind. Doing this
    //    BEFORE persist ensures that either both the main DB and its
    //    siblings get replaced/cleared, or — if any earlier check
    //    failed — none of them are touched.
    //
    // Build sibling paths via `OsString::push` so non-UTF-8 bytes
    // round-trip intact; `remove_file` runs unconditionally and
    // `ErrorKind::NotFound` is a silent no-op (closes the `exists()`
    // TOCTOU gate). Ordering requires the dest lock conn to be dropped
    // first so cross-platform unlink semantics hold.
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

    // 8. Persist atomically over the destination.
    tmp.persist(dest_db_path)
        .map_err(|e| WalletStorageError::Io(e.error))?;

    // 9. Fsync the destination's parent directory so the atomic rename's
    //    dentry update is durable across power loss (no-op on non-Unix).
    fsync_parent_dir(dest_db_path)?;

    // 10. Re-tighten siblings (SQLite may materialise -wal/-shm on next
    //     open; this is idempotent at restore-completion time).
    apply_secure_permissions(dest_db_path)?;
    Ok(())
}

/// Run `PRAGMA integrity_check` and return `Ok(())` when SQLite reports
/// the single row `"ok"`. Any other result becomes a typed
/// `IntegrityCheckFailed` via the caller-supplied builder; an
/// underlying rusqlite error surfaces as `IntegrityCheckRunFailed`.
///
/// SQLite returns one row per detected problem (capped at
/// `PRAGMA integrity_check(N)`; default 100). All rows are collected
/// and joined with `\n` so the typed report carries every diagnostic
/// instead of just the first line.
///
/// `pub(crate)` so the persister's open-time A-8 probe shares the
/// same helper rather than reimplementing the report-rendering rule.
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
                // Severe corruption can cause SQLite to surface a
                // `DatabaseCorrupt` SqliteFailure partway through the
                // integrity_check stream. Treat it as end-of-stream
                // when we already have diagnostics (the rows we have
                // are still valid); if we have NOTHING, surface the
                // typed `IntegrityCheckRunFailed`.
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
        let pass_count = match policy.keep_last_n {
            Some(n) => idx < n,
            None => true,
        };
        let pass_age = match policy.max_age {
            Some(max) => now.duration_since(ts).map(|d| d <= max).unwrap_or(true),
            None => true,
        };
        if pass_count && pass_age {
            kept += 1;
        } else {
            match std::fs::remove_file(&path) {
                Ok(()) => removed.push(path),
                Err(e) => {
                    // A failed `remove_file` leaves the file on disk, so
                    // it MUST be counted in `kept`. The invariant
                    // `kept + removed.len() == total` then holds and
                    // `failed_removals` is a subset of `kept`.
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
