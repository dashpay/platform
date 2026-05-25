//! Online backup, restore, and retention helpers.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::backup::Backup;
use rusqlite::{Connection, OptionalExtension};

use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::persister::{PruneReport, RetentionPolicy};
use crate::sqlite::util::permissions::apply_secure_permissions;

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
/// `dest`'s parent directory. The temp is `persist`-ed over `dest`
/// only on success — any failure (open, chmod, backup-stream) drops
/// the temp without ever materialising a partial `.db` file at the
/// caller's path.
pub fn run_to(src: &Connection, dest: &Path) -> Result<(), WalletStorageError> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // Reject pre-existing destinations BEFORE staging so the temp file
    // isn't created (and immediately dropped) on a duplicate path. The
    // CLI's `backup_to(file_path)` relies on this typed error; auto-
    // backup callers can't trip it because the filename carries a
    // unique timestamp suffix.
    if dest.exists() {
        return Err(WalletStorageError::BackupDestinationExists {
            path: dest.to_path_buf(),
        });
    }

    // Stage the backup into an unguessable temp file in the same
    // directory. Same-FS guarantee makes `persist` an atomic rename.
    let parent = dest.parent().unwrap_or(Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(parent)?;
    // SEC-011: tighten the temp's mode to 0o600 BEFORE persist so the
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

    tmp.persist(dest)
        .map_err(|e| WalletStorageError::Io(e.error))?;
    // SEC-011: re-tighten in case a non-Unix build (or a future
    // platform-specific tweak) needs to refresh sibling perms after
    // SQLite materialised them. No-op on Unix where the temp already
    // landed at 0o600.
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
/// The restore is staged in two phases bounded by an exclusive
/// advisory file lock on `dest_db_path` (kept across the entire body):
///
/// 1. Open the source read-only; run `PRAGMA integrity_check` +
///    schema-history + max-version sniffs. Any failure here aborts
///    before the live destination is touched.
/// 2. Stream the source into a `NamedTempFile` in `dest_db_path`'s
///    parent directory; re-run integrity + schema gates against the
///    STAGED bytes (catches a torn `io::copy`); unlink the existing
///    `-wal` / `-shm` siblings; chmod the temp to 0o600; then
///    `persist` over `dest_db_path` as an atomic rename.
///
/// Either both the main DB and its WAL/SHM siblings are replaced, or
/// — on any pre-persist failure — none of them are touched. The
/// exclusive lock prevents a racing opener from materialising new
/// WAL/SHM siblings against the about-to-be-replaced inode.
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
    let src_has_schema = src
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !src_has_schema {
        return Err(WalletStorageError::SchemaHistoryMissing);
    }
    crate::sqlite::migrations::assert_schema_version_supported(&src)?;
    drop(src);

    // 2. ATOM-005 (A-2): take an exclusive advisory lock on the
    //    destination and HOLD it across the entire restore body. The
    //    pre-A-2 code probed the lock, dropped the handle, then ran
    //    steps 3-7 unlocked — a concurrent process opening
    //    `dest_db_path` between the probe and `tmp.persist` would race
    //    the rename and end up holding a fd against the unlinked old
    //    inode while the new DB takes its place. Keeping the guard
    //    `_lock` alive in scope closes that window.
    //
    //    On filesystems without flock support (the previous silent-skip
    //    arm) we emit a structured warn so operators know the safety
    //    net is bypassed — still proceed because there's no alternative
    //    on such filesystems, but never silently.
    let _lock: Option<std::fs::File> = if dest_db_path.exists() {
        use fs2::FileExt;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(dest_db_path)?;
        match f.try_lock_exclusive() {
            Ok(()) => Some(f),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(WalletStorageError::RestoreDestinationLocked);
            }
            Err(_) => {
                tracing::warn!(
                    target: "platform_wallet_storage",
                    dest = %dest_db_path.display(),
                    "advisory lock unsupported on this filesystem; concurrent-writer race possible"
                );
                None
            }
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

    // 4. SEC-004: re-run integrity_check on the STAGED file before
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
        let has_schema = staged
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
                [],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !has_schema {
            return Err(WalletStorageError::SchemaHistoryMissing);
        }
        crate::sqlite::migrations::assert_schema_version_supported(&staged)?;
    }

    // 5. Atomicity gate: every staged-file validation has now passed,
    //    so it's safe to clear WAL/SHM siblings the replaced DB might
    //    have left behind. Doing this BEFORE persist ensures that
    //    either both the main DB and its siblings get replaced/cleared,
    //    or — if any earlier check failed — none of them are touched.
    for ext in ["-wal", "-shm"] {
        let sibling = dest_db_path.with_file_name(format!(
            "{}{ext}",
            dest_db_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        if sibling.exists() {
            std::fs::remove_file(&sibling)?;
        }
    }

    // 6. ATOM-010 (A-5): chmod 600 on the temp BEFORE persist so the
    //    destination inherits owner-only mode via the atomic rename.
    //    Pre-A-5 the chmod ran post-persist — a rare chmod failure
    //    returned Err while leaving the new DB live at the destination
    //    (caller thought restore rolled back, reality was mixed).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }

    // 7. Persist atomically over the destination.
    tmp.persist(dest_db_path)
        .map_err(|e| WalletStorageError::Io(e.error))?;

    // 8. Re-tighten siblings (SQLite may materialise -wal/-shm on next
    //    open; this is idempotent at restore-completion time).
    apply_secure_permissions(dest_db_path)?;
    // Lock guard is released by `_lock` going out of scope here.
    Ok(())
}

/// Run `PRAGMA integrity_check` and return `Ok(())` if SQLite returns
/// "ok". Any other returned text becomes a typed `IntegrityCheckFailed`
/// via the caller-supplied builder; an underlying rusqlite error
/// surfaces as `IntegrityCheckRunFailed`.
fn run_integrity_check<F>(conn: &Connection, on_failure: F) -> Result<(), WalletStorageError>
where
    F: FnOnce(String) -> WalletStorageError,
{
    let report: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|source| WalletStorageError::IntegrityCheckRunFailed { source })?;
    if report == "ok" {
        Ok(())
    } else {
        Err(on_failure(report))
    }
}

/// Apply retention to a directory. Files that match the recognised
/// backup-name prefixes are eligible; others are ignored.
///
// INTENTIONAL(CODE-007): prune fails-fast on the first I/O error
// rather than collecting per-file failures into PruneReport.
// Acceptable because the operator gets a typed error with the
// offending path; retrying prune is idempotent.
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
            std::fs::remove_file(&path)?;
            removed.push(path);
        }
    }
    // Sort `removed` oldest-first for deterministic output.
    removed.sort();
    Ok(PruneReport { removed, kept })
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
