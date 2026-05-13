//! SEC-004 / SEC-011: chmod helpers for newly created DB files.
//!
//! Restricts the on-disk SQLite files (live DB, backup copies, restored
//! DB) to owner-only on Unix so the mode never depends on the calling
//! process's umask. Windows has no equivalent permission model here and
//! is a no-op.

use std::path::Path;

use crate::sqlite::error::WalletStorageError;

/// Apply owner-only (`0o600`) permissions to `path` on Unix.
/// No-op on non-Unix platforms.
#[allow(unused_variables)] // `path` is unused on non-Unix.
pub fn apply_secure_permissions(path: &Path) -> Result<(), WalletStorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }
    Ok(())
}
