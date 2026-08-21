//! Shared parent-directory permission check for file-backed stores.

use std::path::Path;

#[derive(Debug)]
pub(crate) enum ParentPermissionsError {
    Io(std::io::Error),
    Insecure { mode: u32 },
}

#[cfg(unix)]
fn trusted_owner(owner: u32, current_uid: u32, root_uid: u32) -> bool {
    owner == current_uid || owner == 0 || owner == root_uid
}

#[cfg(unix)]
#[expect(unsafe_code, reason = "libc geteuid requires an unsafe call")]
fn effective_uid() -> u32 {
    // SAFETY: geteuid takes no arguments, has no failure mode, and reads only
    // the process credential maintained by the kernel.
    unsafe { libc::geteuid() }
}

/// Validate Unix ownership and replacement permissions up to the filesystem root.
///
/// Both the lexical path and its canonical target are walked through `/`, so a
/// symlink cannot hide unsafe target ancestors. A writable component is
/// accepted only with the sticky bit, and every component must be owned by the
/// effective user or a root identity. The filesystem root's owner is treated
/// as root for user-namespace environments where uid 0 is mapped to another uid.
#[cfg(unix)]
pub(crate) fn check_parent_perms(parent: &Path) -> Result<(), ParentPermissionsError> {
    use std::os::unix::fs::MetadataExt;

    let current_uid = effective_uid();
    let root_uid = std::fs::metadata("/")
        .map_err(ParentPermissionsError::Io)?
        .uid();
    let absolute = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(ParentPermissionsError::Io)?
            .join(parent)
    };
    let canonical = std::fs::canonicalize(&absolute).map_err(ParentPermissionsError::Io)?;

    for ancestor in absolute.ancestors() {
        let meta = std::fs::symlink_metadata(ancestor).map_err(ParentPermissionsError::Io)?;
        let mode = meta.mode() & 0o7777;
        let writable_without_sticky =
            !meta.file_type().is_symlink() && mode & 0o022 != 0 && mode & 0o1000 == 0;
        if writable_without_sticky || !trusted_owner(meta.uid(), current_uid, root_uid) {
            return Err(ParentPermissionsError::Insecure { mode });
        }
    }
    for ancestor in canonical.ancestors() {
        let meta = std::fs::metadata(ancestor).map_err(ParentPermissionsError::Io)?;
        let mode = meta.mode() & 0o7777;
        let writable_without_sticky = mode & 0o022 != 0 && mode & 0o1000 == 0;
        if writable_without_sticky || !trusted_owner(meta.uid(), current_uid, root_uid) {
            return Err(ParentPermissionsError::Insecure { mode });
        }
    }
    Ok(())
}

// Windows ACL checks require platform-specific security APIs; tracked by
// https://github.com/dashpay/platform/issues/3754.
#[cfg(not(unix))]
pub(crate) fn check_parent_perms(_parent: &Path) -> Result<(), ParentPermissionsError> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::trusted_owner;

    #[test]
    fn trusted_owner_accepts_current_and_root_identities_only() {
        assert!(trusted_owner(1000, 1000, 65_534));
        assert!(trusted_owner(0, 1000, 65_534));
        assert!(trusted_owner(65_534, 1000, 65_534));
        assert!(!trusted_owner(2000, 1000, 65_534));
    }
}
