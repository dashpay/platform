//! Owner-only permission helpers for the SQLite DB files.
//!
//! Restricts the on-disk SQLite files (live DB, backup copies, restored
//! DB) to owner-only on Unix so the mode never depends on the calling
//! process's umask, mirroring the secrets-vault path. Windows has no
//! equivalent permission model here and these are no-ops there.

use std::path::Path;

use crate::sqlite::error::WalletStorageError;

/// Refuse `path` when it is a symbolic link.
///
/// `O_CREAT|O_EXCL` returns `EEXIST` for a symlink whether or not its
/// target exists, so `EEXIST` alone cannot tell a legitimate existing
/// database from a planted link — and every operation that follows (the
/// SQLite open, the `0o600` chmod) resolves the link. A missing path is
/// accepted: the caller creates it with `O_EXCL`, which cannot be
/// redirected.
///
/// # Errors
///
/// [`WalletStorageError::DatabasePathIsSymlink`] when `path` is a link;
/// [`WalletStorageError::Io`] when its metadata cannot be read.
pub fn reject_symlink(path: &Path) -> Result<(), WalletStorageError> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(WalletStorageError::DatabasePathIsSymlink {
                path: path.to_path_buf(),
            })
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(WalletStorageError::Io(e)),
    }
}

/// Pre-create the DB file at `path` with mode `0o600` before rusqlite
/// opens it, closing the umask window (the file is born owner-only, not
/// created at the umask default then chmod'd) and the final-component
/// symlink redirect.
///
/// A no-op when the file already exists (re-open of an existing DB) — the
/// live mode is then re-tightened by [`apply_secure_permissions`] after
/// open. No-op on non-Unix.
///
/// # Errors
///
/// [`WalletStorageError::DatabasePathIsSymlink`] when `path` is a symlink
/// rather than the database itself.
#[allow(unused_variables)]
pub fn precreate_secure(path: &Path) -> Result<(), WalletStorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
        {
            Ok(_file) => {}
            // Already present — re-open path; the real open + the
            // post-open chmod handle it. `O_EXCL` reports EEXIST for a
            // planted symlink too, so the redirect is ruled out here
            // rather than followed by the open and the chmod below.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => reject_symlink(path)?,
            Err(e) => return Err(WalletStorageError::Io(e)),
        }
    }
    Ok(())
}

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
        // WAL mode is the default for this crate, so recent committed
        // pages live in <path>-wal / <path>-shm. Without this sweep the
        // sidecars stay at the process umask default — a local-user info
        // leak on multi-user hosts.
        //
        // The sibling path is built via `OsString::push` so non-UTF-8
        // path bytes survive intact (no `to_string_lossy` corruption).
        // `set_permissions` runs unconditionally — a missing sibling
        // returns `ErrorKind::NotFound`, treated as a silent no-op
        // (closes the `exists()` TOCTOU gate).
        let Some(file_name) = path.file_name() else {
            return Ok(());
        };
        for ext in ["-wal", "-shm"] {
            let mut sibling_name = file_name.to_os_string();
            sibling_name.push(ext);
            let sibling = path.with_file_name(sibling_name);
            match std::fs::set_permissions(&sibling, perms.clone()) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(WalletStorageError::Io(e)),
            }
        }
    }
    Ok(())
}
