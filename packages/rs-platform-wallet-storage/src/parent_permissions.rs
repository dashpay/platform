//! Shared parent-directory permission check for file-backed stores.

use std::path::Path;

#[derive(Debug)]
pub(crate) enum ParentPermissionsError {
    Io(std::io::Error),
    Insecure { mode: u32 },
}

/// Refuse a group/other-writable parent directory on Unix.
#[cfg(unix)]
pub(crate) fn check_parent_perms(parent: &Path) -> Result<(), ParentPermissionsError> {
    use std::os::unix::fs::MetadataExt;

    let meta = std::fs::metadata(parent).map_err(ParentPermissionsError::Io)?;
    let mode = meta.mode() & 0o777;
    if mode & 0o022 != 0 {
        return Err(ParentPermissionsError::Insecure { mode });
    }
    Ok(())
}

// Windows ACL checks require platform-specific security APIs; tracked by
// https://github.com/dashpay/platform/issues/3754.
#[cfg(not(unix))]
pub(crate) fn check_parent_perms(_parent: &Path) -> Result<(), ParentPermissionsError> {
    Ok(())
}
