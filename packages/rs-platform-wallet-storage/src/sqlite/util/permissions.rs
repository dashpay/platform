//! SEC-004 / SEC-011: chmod helpers for newly created DB files.
//!
//! Restricts the on-disk SQLite files (live DB, backup copies, restored
//! DB) to owner-only on Unix so the mode never depends on the calling
//! process's umask. Windows has no equivalent permission model here and
//! is a no-op.

use std::path::Path;

use crate::sqlite::error::WalletStorageError;

/// Apply owner-only (`0o600`) permissions to `path` on Unix, plus its
/// `-wal` / `-shm` SQLite sidecars when present. Siblings that don't
/// exist are skipped silently — they're only created on demand by
/// SQLite's WAL journaling mode. No-op on non-Unix platforms.
#[allow(unused_variables)] // `path` is unused on non-Unix.
pub fn apply_secure_permissions(path: &Path) -> Result<(), WalletStorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms.clone())?;
        // SEC-004: WAL mode is the default for this crate, so recent
        // committed pages live in <path>-wal / <path>-shm. Without this
        // sweep, the sidecars stay at the process umask default — a
        // local-user info leak on multi-user hosts.
        for ext in ["-wal", "-shm"] {
            let sibling = path.with_file_name(format!(
                "{}{ext}",
                path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            ));
            if sibling.exists() {
                std::fs::set_permissions(&sibling, perms.clone())?;
            }
        }
    }
    Ok(())
}
