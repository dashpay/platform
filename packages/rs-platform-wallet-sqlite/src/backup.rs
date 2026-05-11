//! Online backup, restore, and retention helpers.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::backup::Backup;
use rusqlite::Connection;

use platform_wallet::wallet::platform_wallet::WalletId;

use crate::error::SqlitePersisterError;
use crate::persister::{PruneReport, RetentionPolicy};

/// Distinguishes auto-backup filenames.
#[derive(Debug, Clone, Copy)]
pub enum BackupKind {
    PreMigration { from: i32, to: i32 },
    PreDelete { wallet_id: WalletId },
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
    }
}

/// Take an online backup of `src` to `dest`. Uses the
/// `rusqlite::backup::Backup::run_to_completion` page-stepping API
/// (250 ms steps, 5 ms inter-step pause) so writers aren't blocked.
pub fn run_to(src: &Connection, dest: &Path) -> Result<(), SqlitePersisterError> {
    if let Some(parent) = dest.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut backup_conn = Connection::open(dest)?;
    let backup = Backup::new(src, &mut backup_conn)?;
    // Pages per step. The plan's `Duration::from_millis(250)`
    // figure is the *step duration*, not a page count; in rusqlite
    // 0.38 the API takes a page count + pause + optional progress
    // callback. 100 pages × 4 KiB = 400 KiB per step, which on a
    // typical SSD takes well under 250 ms.
    backup.run_to_completion(100, Duration::from_millis(5), None)?;
    Ok(())
}

/// Restore a `.db` backup over `dest_db_path`. Associated function;
/// caller must guarantee the destination is not held open by this
/// process.
pub fn restore_from(dest_db_path: &Path, src_backup: &Path) -> Result<(), SqlitePersisterError> {
    // 1. Validate source — opens read-only, runs PRAGMA integrity_check,
    //    requires `refinery_schema_history`, and checks the schema
    //    version is within the supported range (D-04).
    let src = match Connection::open_with_flags(
        src_backup,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) {
        Ok(c) => c,
        Err(e) => {
            return Err(SqlitePersisterError::IntegrityCheckFailed {
                check_output: format!("cannot open source: {e}"),
            });
        }
    };
    let check: String = src
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| SqlitePersisterError::IntegrityCheckFailed {
            check_output: format!("integrity_check error: {e}"),
        })?;
    if check != "ok" {
        return Err(SqlitePersisterError::IntegrityCheckFailed {
            check_output: check,
        });
    }
    let has_schema: bool = src
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !has_schema {
        return Err(SqlitePersisterError::SchemaHistoryMissing);
    }
    let max_supported = crate::migrations::embedded_migrations()
        .iter()
        .map(|(v, _)| *v as i64)
        .max()
        .unwrap_or(0);
    let source_version: Option<i64> = src
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get(0),
        )
        .ok()
        .flatten();
    if let Some(v) = source_version {
        if v > max_supported {
            return Err(SqlitePersisterError::SchemaVersionUnsupported {
                found: v,
                expected_range: format!("0..={max_supported}"),
            });
        }
    }
    drop(src);

    // 2. Try-lock the destination so we don't replace a DB that another
    //    process still holds open. `fs2::FileExt::try_lock_exclusive`
    //    is non-blocking; if the file is held we surface
    //    `RestoreDestinationLocked` (D-03). On platforms where flock
    //    fails for unrelated reasons (e.g. tmpfs without advisory
    //    locking) the error path falls through to the generic Io
    //    variant.
    if dest_db_path.exists() {
        use fs2::FileExt;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(dest_db_path)
            .map_err(SqlitePersisterError::Io)?;
        match f.try_lock_exclusive() {
            Ok(()) => {
                let _ = FileExt::unlock(&f);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Err(SqlitePersisterError::RestoreDestinationLocked);
            }
            Err(_) => {
                // Advisory locks unsupported on this FS — proceed.
            }
        }
    }

    // 3. Remove any WAL / SHM siblings of the destination so SQLite
    //    can't open the live wallet's stale auxiliary state by mistake.
    for ext in ["-wal", "-shm"] {
        let sibling = dest_db_path.with_file_name(format!(
            "{}{ext}",
            dest_db_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        if sibling.exists() {
            std::fs::remove_file(&sibling).map_err(SqlitePersisterError::Io)?;
        }
    }

    // 4. Stage the source into a `NamedTempFile` in the destination's
    //    parent dir, then atomically `persist` over the destination
    //    (SEC-001: the temp filename is unguessable, eliminating a
    //    symlink-plant TOCTOU window on the predictable
    //    `<dest>.db.restore-tmp` path).
    let parent = dest_db_path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(SqlitePersisterError::Io)?;
    let mut src_file = std::fs::File::open(src_backup).map_err(SqlitePersisterError::Io)?;
    std::io::copy(&mut src_file, tmp.as_file_mut()).map_err(SqlitePersisterError::Io)?;
    tmp.as_file().sync_all().map_err(SqlitePersisterError::Io)?;
    tmp.persist(dest_db_path)
        .map_err(|e| SqlitePersisterError::Io(e.error))?;
    Ok(())
}

/// Apply retention to a directory. Files that match the recognised
/// backup-name prefixes are eligible; others are ignored.
pub fn prune(dir: &Path, policy: RetentionPolicy) -> Result<PruneReport, SqlitePersisterError> {
    let entries = std::fs::read_dir(dir).map_err(SqlitePersisterError::Io)?;
    let mut files: Vec<(SystemTime, PathBuf)> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(SqlitePersisterError::Io)?;
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
            std::fs::remove_file(&path).map_err(SqlitePersisterError::Io)?;
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
        || name.starts_with("pre-delete-"))
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
        assert!(!is_backup_file(Path::new("/tmp/notes.txt")));
        assert!(!is_backup_file(Path::new("/tmp/wallet.db")));
    }
}
