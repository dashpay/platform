//! Shared parent-directory permission check for file-backed stores.

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub(crate) enum ParentPermissionsError {
    Io(std::io::Error),
    Insecure {
        ancestor: PathBuf,
        reason: InsecureAncestor,
    },
}

/// Why an ancestor directory of a wallet file or vault was refused.
///
/// The two causes need different remediations, and a mode alone cannot tell
/// them apart: an ancestor rejected for its OWNER usually carries a perfectly
/// ordinary `0755`, so reporting that mode and asking for `chmod go-w` sends
/// the user to a command that cannot work.
///
/// Unix-only. `check_parent_perms` is a no-op on other targets — Windows ACL
/// inspection is tracked by dashpay/platform#3754 — so no value of this type is
/// ever produced there, and the POSIX remediation each arm names is always
/// correct where it can appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsecureAncestor {
    /// Group- or other-writable without the sticky bit: any local user can
    /// replace entries in it, and so replace the `0600` file below it.
    WritableWithoutSticky {
        /// The offending POSIX mode bits.
        mode: u32,
    },
    /// Owned by neither the effective user nor a root identity: its owner can
    /// grant itself write access whenever it likes, so the current mode is not
    /// a guarantee of anything.
    UntrustedOwner {
        /// The ancestor's owner uid.
        uid: u32,
        /// The process's effective uid.
        current_uid: u32,
    },
}

/// One actionable sentence naming the exact ancestor and the exact command.
///
/// `subject` is the artefact being protected ("database" / "vault"), so the two
/// error enums that carry this rejection phrase it identically.
pub(crate) fn insecure_ancestor_message(
    subject: &str,
    ancestor: &Path,
    reason: &InsecureAncestor,
) -> String {
    let path = ancestor.display();
    match reason {
        InsecureAncestor::WritableWithoutSticky { mode } => format!(
            "{subject} ancestor {path} is group/other-writable (mode {mode:04o}) and not sticky; \
             run `chmod go-w {path}` unless it is an intentional sticky shared directory"
        ),
        InsecureAncestor::UntrustedOwner { uid, current_uid } => format!(
            "{subject} ancestor {path} is owned by uid {uid}, neither the current user \
             ({current_uid}) nor root; run `chown {current_uid} {path}`"
        ),
    }
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
///
/// Both walks classify through here, so neither can report an ownership
/// rejection as a permission one. Write access is checked first: it is the
/// condition whose mode is worth printing.
#[cfg(unix)]
fn reject_ancestor(
    ancestor: &Path,
    uid: u32,
    mode: u32,
    writable_without_sticky: bool,
    current_uid: u32,
    root_uid: u32,
) -> Result<(), ParentPermissionsError> {
    let reason = if writable_without_sticky {
        InsecureAncestor::WritableWithoutSticky { mode }
    } else if !trusted_owner(uid, current_uid, root_uid) {
        InsecureAncestor::UntrustedOwner { uid, current_uid }
    } else {
        return Ok(());
    };
    Err(ParentPermissionsError::Insecure {
        ancestor: ancestor.to_path_buf(),
        reason,
    })
}

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
        reject_ancestor(
            ancestor,
            meta.uid(),
            mode,
            writable_without_sticky,
            current_uid,
            root_uid,
        )?;
    }
    for ancestor in canonical.ancestors() {
        let meta = std::fs::metadata(ancestor).map_err(ParentPermissionsError::Io)?;
        let mode = meta.mode() & 0o7777;
        let writable_without_sticky = mode & 0o022 != 0 && mode & 0o1000 == 0;
        reject_ancestor(
            ancestor,
            meta.uid(),
            mode,
            writable_without_sticky,
            current_uid,
            root_uid,
        )?;
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
