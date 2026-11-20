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

/// Create `dir` and its missing ancestors owner-only (`0o700`) on Unix,
/// whatever the umask. Components that already exist keep their mode: they
/// belong to the caller, not to this crate.
///
/// Components are created one `mkdir` at a time, outward from the nearest
/// existing ancestor, and each is restricted right after its own `mkdir`
/// succeeds, so its children can be created even under a umask that strips
/// owner bits. A component another process creates first (`AlreadyExists`)
/// is treated as the caller's and left alone.
pub(crate) fn create_owner_only_dirs(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        use std::path::{Component, PathBuf};

        let components: Vec<Component<'_>> = dir.components().collect();
        let existing = (1..=components.len())
            .rev()
            .find(|&len| components[..len].iter().collect::<PathBuf>().exists())
            .unwrap_or(0);
        let mut current: PathBuf = components[..existing].iter().collect();
        for component in &components[existing..] {
            current.push(component);
            // `..` names a directory that already exists; only real names
            // are created here.
            if !matches!(component, Component::Normal(_)) {
                continue;
            }
            match std::fs::DirBuilder::new().mode(0o700).create(&current) {
                Ok(()) => restrict_created_dir(&current)?,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
    }
    #[cfg(not(unix))]
    std::fs::create_dir_all(dir)?;
    Ok(())
}

/// Set a directory this process just created to `0o700` without following a
/// symlink planted in its place.
///
/// The `fchmod` goes through an `O_NOFOLLOW` handle. A umask that stripped
/// the owner's read bit makes that open fail with `EACCES`, so the fallback
/// is `fchmodat(AT_SYMLINK_NOFOLLOW)`, which needs no read access.
#[cfg(unix)]
fn restrict_created_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(handle) => handle.set_permissions(std::fs::Permissions::from_mode(0o700)),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => chmod_no_follow(path, 0o700),
        Err(e) => Err(e),
    }
}

/// `chmod` that refuses a symlink rather than changing its target.
///
/// Linux has no native no-follow `chmod` before `fchmodat2` (kernel 6.6);
/// libc emulates it there through `/proc` and fails with `EOPNOTSUPP` when it
/// cannot.
#[cfg(unix)]
#[expect(unsafe_code, reason = "libc fchmodat has no std equivalent")]
fn chmod_no_follow(path: &Path, mode: libc::mode_t) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: `c_path` is a live NUL-terminated string, read only for the
    // duration of the call; `AT_FDCWD` resolves it like any relative path.
    let result = unsafe {
        libc::fchmodat(
            libc::AT_FDCWD,
            c_path.as_ptr(),
            mode,
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
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

#[cfg(all(test, unix))]
mod tests {
    use super::{chmod_no_follow, create_owner_only_dirs, restrict_created_dir};
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    /// Set by the parent test for the re-executed child; `umask` is
    /// process-wide, so the restrictive mask is applied in a child process
    /// that runs this one test and nothing else.
    const UMASK_CHILD_DIR: &str = "PLATFORM_WALLET_STORAGE_UMASK_CHILD_DIR";

    #[test]
    #[expect(unsafe_code, reason = "libc umask requires an unsafe call")]
    fn should_create_owner_only_dirs_under_a_umask_that_strips_owner_bits() {
        if let Some(base) = std::env::var_os(UMASK_CHILD_DIR) {
            // SAFETY: umask only swaps the process file-mode mask; this
            // re-executed process runs this single test.
            let previous = unsafe { libc::umask(0o777) };
            let result = create_owner_only_dirs(&Path::new(&base).join("a").join("b").join("c"));
            // SAFETY: as above.
            unsafe { libc::umask(previous) };
            result.expect("create_owner_only_dirs under umask 0777");
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let module = module_path!().split_once("::").unwrap().1;
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                &format!(
                    "{module}::should_create_owner_only_dirs_under_a_umask_that_strips_owner_bits"
                ),
                "--test-threads=1",
            ])
            .env(UMASK_CHILD_DIR, tmp.path())
            .status()
            .unwrap();
        assert!(status.success(), "child process failed: {status}");

        let a = tmp.path().join("a");
        for created in [a.clone(), a.join("b"), a.join("b").join("c")] {
            assert_eq!(
                mode(&created),
                0o700,
                "{} must be owner-only",
                created.display()
            );
        }
    }

    #[test]
    fn should_leave_existing_components_alone_and_restrict_only_created_ones() {
        let tmp = tempfile::tempdir().unwrap();
        let existing = tmp.path().join("existing");
        std::fs::create_dir(&existing).unwrap();
        std::fs::set_permissions(&existing, std::fs::Permissions::from_mode(0o775)).unwrap();

        create_owner_only_dirs(&existing.join("new").join("leaf")).unwrap();

        assert_eq!(mode(&existing), 0o775);
        assert_eq!(mode(&existing.join("new")), 0o700);
        assert_eq!(mode(&existing.join("new").join("leaf")), 0o700);
    }

    /// The tighten step must never chmod through a symlink: a link planted
    /// in place of a just-created directory points at someone else's file.
    #[test]
    fn should_not_follow_a_symlink_when_restricting_a_created_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(restrict_created_dir(&link).is_err());
        assert_eq!(mode(&target), 0o755);
    }

    #[test]
    fn should_not_follow_a_symlink_in_the_no_follow_chmod_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(chmod_no_follow(&link, 0o700).is_err());
        assert_eq!(mode(&target), 0o755);
    }
}
