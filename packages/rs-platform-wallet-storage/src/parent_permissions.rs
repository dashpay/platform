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

/// Platform identities trusted as ancestor owners and as the group of a
/// group-writable ancestor, beyond the current user and root.
///
/// Android creates every app's private storage beneath `/data`, `/data/user`,
/// `/data/user/0` and `/data/data`, all owned `system:system` (`AID_SYSTEM`,
/// 1000) and mostly `0771`. No app-private path can pass the walk without
/// trusting that identity. `AID_SYSTEM` is the platform itself — it already
/// administers every app's data — and no third-party app can run as it or join
/// its group, so a `system`-group-writable ancestor is not replaceable by
/// anything the check defends against.
#[cfg(target_os = "android")]
const PLATFORM_IDS: &[u32] = &[1000];
#[cfg(all(unix, not(target_os = "android")))]
const PLATFORM_IDS: &[u32] = &[];

#[cfg(unix)]
fn trusted_owner(owner: u32, current_uid: u32, root_uid: u32, platform_ids: &[u32]) -> bool {
    owner == current_uid || owner == 0 || owner == root_uid || platform_ids.contains(&owner)
}

/// Groups whose write access to an ancestor is not a replacement risk.
///
/// On Android this is `PLATFORM_IDS` plus the app's own per-app group. Android
/// gives every app a gid equal to its uid, and `Context.getFilesDir()`,
/// `getDatabasesDir()` and friends create the app's own directories `0771`
/// in that group, so `<dataDir>/files` is group-writable by design. No other
/// app can be a member of that group. Everywhere else the set is empty: a
/// desktop user's primary group (macOS `staff`, for one) is commonly shared.
#[cfg(target_os = "android")]
fn trusted_groups(current_uid: u32) -> Vec<u32> {
    let mut groups = PLATFORM_IDS.to_vec();
    groups.push(current_uid);
    groups
}
#[cfg(all(unix, not(target_os = "android")))]
fn trusted_groups(_current_uid: u32) -> Vec<u32> {
    PLATFORM_IDS.to_vec()
}

/// Whether an identity outside the trusted set can replace entries in a
/// directory with this mode and group: other-writable, or group-writable for a
/// group outside `trusted_groups`, in both cases without the sticky bit.
#[cfg(unix)]
fn writable_without_sticky(mode: u32, gid: u32, trusted_groups: &[u32]) -> bool {
    if mode & 0o1000 != 0 {
        return false;
    }
    mode & 0o002 != 0 || (mode & 0o020 != 0 && !trusted_groups.contains(&gid))
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
    } else if !trusted_owner(uid, current_uid, root_uid, PLATFORM_IDS) {
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
    let trusted_groups = trusted_groups(current_uid);
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
        let writable_without_sticky = !meta.file_type().is_symlink()
            && writable_without_sticky(mode, meta.gid(), &trusted_groups);
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
        let writable_without_sticky = writable_without_sticky(mode, meta.gid(), &trusted_groups);
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
    use super::{trusted_owner, writable_without_sticky};

    const ANDROID_SYSTEM: &[u32] = &[1000];

    #[test]
    fn trusted_owner_accepts_current_and_root_identities_only() {
        assert!(trusted_owner(1000, 1000, 65_534, &[]));
        assert!(trusted_owner(0, 1000, 65_534, &[]));
        assert!(trusted_owner(65_534, 1000, 65_534, &[]));
        assert!(!trusted_owner(2000, 1000, 65_534, &[]));
    }

    #[test]
    fn should_trust_android_system_owner_only_when_it_is_a_platform_identity() {
        // App uid 10123 under /data/user/0 (owner system, 1000).
        assert!(!trusted_owner(1000, 10_123, 0, &[]));
        assert!(trusted_owner(1000, 10_123, 0, ANDROID_SYSTEM));
        // Another app's uid stays untrusted.
        assert!(!trusted_owner(10_456, 10_123, 0, ANDROID_SYSTEM));
    }

    #[test]
    fn should_reject_group_write_unless_group_is_a_platform_identity() {
        // `/data` on Android: system:system 0771.
        assert!(writable_without_sticky(0o771, 1000, &[]));
        assert!(!writable_without_sticky(0o771, 1000, ANDROID_SYSTEM));
        // Group-writable for any other group is still replaceable.
        assert!(writable_without_sticky(0o775, 1015, ANDROID_SYSTEM));
        assert!(writable_without_sticky(0o770, 0, ANDROID_SYSTEM));
    }

    #[test]
    fn should_accept_app_files_dir_group_writable_by_the_apps_own_group() {
        // `<dataDir>/files` on Android: app uid 10150, group 10150, 0771.
        // `trusted_groups(10_150)` on Android is `[1000, 10_150]`.
        let android_app: &[u32] = &[1000, 10_150];
        assert!(!writable_without_sticky(0o771, 10_150, android_app));
        // Another app's group, and the cache gid (20000 + app id), stay refused.
        assert!(writable_without_sticky(0o771, 10_456, android_app));
        assert!(writable_without_sticky(0o2771, 20_150, android_app));
        // Other-write is refused even in the app's own group.
        assert!(writable_without_sticky(0o777, 10_150, android_app));
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn should_trust_no_extra_groups_off_android() {
        assert!(super::trusted_groups(501).is_empty());
        // A user-owned, group-writable folder (macOS `staff` is gid 20) is
        // still refused on desktop.
        assert!(writable_without_sticky(
            0o775,
            20,
            &super::trusted_groups(501)
        ));
    }

    #[test]
    fn should_reject_other_write_regardless_of_group() {
        assert!(writable_without_sticky(0o757, 1000, ANDROID_SYSTEM));
        assert!(writable_without_sticky(0o777, 1000, ANDROID_SYSTEM));
    }

    #[test]
    fn should_accept_sticky_and_unwritable_directories() {
        assert!(!writable_without_sticky(0o1777, 2000, &[]));
        assert!(!writable_without_sticky(0o755, 2000, &[]));
        assert!(!writable_without_sticky(0o700, 2000, &[]));
    }
}
