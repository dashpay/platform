//! Advisory ancestor-directory permission report for file-backed stores.
//!
//! Directories this crate creates are owner-only; the directories above them
//! belong to the user and the platform. An ancestor another local user could
//! replace is reported with a warning and never fails an operation.

use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

/// Why an ancestor directory of a wallet file or vault is reported.
///
/// The two causes need different remediations, and a mode alone cannot tell
/// them apart: an ancestor reported for its OWNER usually carries a perfectly
/// ordinary `0755`, so reporting that mode and asking for `chmod go-w` sends
/// the user to a command that cannot work.
///
/// Unix-only. `warn_on_insecure_ancestors` is a no-op on other targets —
/// Windows ACL inspection is tracked by dashpay/platform#3754.
#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InsecureAncestor {
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
    /// Both at once. The owner can undo a `chmod`, and the current user may
    /// not be able to run one, so the remediation names both.
    UntrustedOwnerAndWritable {
        /// The ancestor's owner uid.
        uid: u32,
        /// The process's effective uid.
        current_uid: u32,
        /// The offending POSIX mode bits.
        mode: u32,
    },
}

/// One actionable sentence naming the exact ancestor and the exact command.
///
/// `subject` is the artefact being protected ("database" / "vault").
#[cfg(unix)]
fn insecure_ancestor_message(subject: &str, ancestor: &Path, reason: &InsecureAncestor) -> String {
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
        InsecureAncestor::UntrustedOwnerAndWritable {
            uid,
            current_uid,
            mode,
        } => format!(
            "{subject} ancestor {path} is owned by uid {uid}, neither the current user \
             ({current_uid}) nor root, and is group/other-writable (mode {mode:04o}) and not \
             sticky; as root, run `chown {current_uid} {path}` and `chmod go-w {path}`"
        ),
    }
}

/// Platform identities trusted as ancestor owners and as the group of a
/// group-writable ancestor, beyond the current user and root.
///
/// Android creates every app's private storage beneath `/data`, `/data/user`,
/// `/data/user/0` and `/data/data`, all owned `system:system` (`AID_SYSTEM`,
/// 1000) and mostly `0771`. Without trusting that identity every app-private
/// path would be reported. `AID_SYSTEM` is the platform itself — it already
/// administers every app's data — and no third-party app can run as it or join
/// its group, so a `system`-group-writable ancestor is not replaceable by
/// another app.
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
///
/// The group is keyed on the uid, not `getegid()`. The two are equal in every
/// app process, but only the uid carries the guarantee: Android reserves the
/// gid numbered like an app's uid for that app alone, while the effective gid
/// is whatever group the process was started with, which outside an app
/// process can be one that other processes share.
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

/// Classify one ancestor; `None` when nobody outside the trusted set can
/// replace entries in it.
///
/// An ancestor that is both foreign-owned and writable is reported for both.
/// The filesystem root's owner is trusted as root for user-namespace
/// environments where uid 0 is mapped to another uid.
#[cfg(unix)]
fn classify_ancestor(
    uid: u32,
    mode: u32,
    writable_without_sticky: bool,
    current_uid: u32,
    root_uid: u32,
) -> Option<InsecureAncestor> {
    let untrusted_owner = !trusted_owner(uid, current_uid, root_uid, PLATFORM_IDS);
    match (untrusted_owner, writable_without_sticky) {
        (true, true) => Some(InsecureAncestor::UntrustedOwnerAndWritable {
            uid,
            current_uid,
            mode,
        }),
        (true, false) => Some(InsecureAncestor::UntrustedOwner { uid, current_uid }),
        (false, true) => Some(InsecureAncestor::WritableWithoutSticky { mode }),
        (false, false) => None,
    }
}

/// What one walk over a directory's ancestors found.
#[cfg(unix)]
#[derive(Default)]
struct AncestorReport {
    /// Ancestors another local user could replace.
    insecure: Vec<(PathBuf, InsecureAncestor)>,
    /// Ancestors the walk could not stat or resolve.
    uninspected: Vec<(PathBuf, std::io::Error)>,
}

/// Every ancestor of `parent`, through `/`, that another local user could
/// replace.
///
/// Both the lexical path and its canonical target are walked, so a symlink
/// cannot hide an unsafe target ancestor. An ancestor seen by both walks is
/// listed once. A component that cannot be inspected is recorded and the walk
/// goes on, so it never hides what the rest of the walk finds.
#[cfg(unix)]
fn insecure_ancestors(parent: &Path) -> AncestorReport {
    use std::os::unix::fs::MetadataExt;

    let mut report = AncestorReport::default();
    let current_uid = effective_uid();
    let trusted_groups = trusted_groups(current_uid);
    // An unreadable `/` is recorded by the lexical walk when it reaches it.
    let root_uid = std::fs::metadata("/").map_or(0, |meta| meta.uid());
    let absolute = if parent.is_absolute() {
        parent.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(parent),
            Err(error) => {
                report.uninspected.push((parent.to_path_buf(), error));
                return report;
            }
        }
    };

    for ancestor in absolute.ancestors() {
        let meta = match std::fs::symlink_metadata(ancestor) {
            Ok(meta) => meta,
            Err(error) => {
                report.uninspected.push((ancestor.to_path_buf(), error));
                continue;
            }
        };
        let mode = meta.mode() & 0o7777;
        let writable = !meta.file_type().is_symlink()
            && writable_without_sticky(mode, meta.gid(), &trusted_groups);
        if let Some(reason) = classify_ancestor(meta.uid(), mode, writable, current_uid, root_uid) {
            report.insecure.push((ancestor.to_path_buf(), reason));
        }
    }
    let canonical = match std::fs::canonicalize(&absolute) {
        Ok(canonical) => canonical,
        Err(error) => {
            report.uninspected.push((absolute, error));
            return report;
        }
    };
    for ancestor in canonical.ancestors() {
        let meta = match std::fs::metadata(ancestor) {
            Ok(meta) => meta,
            Err(error) => {
                report.uninspected.push((ancestor.to_path_buf(), error));
                continue;
            }
        };
        let mode = meta.mode() & 0o7777;
        let writable = writable_without_sticky(mode, meta.gid(), &trusted_groups);
        if let Some(reason) = classify_ancestor(meta.uid(), mode, writable, current_uid, root_uid) {
            if !report.insecure.iter().any(|(seen, _)| seen == ancestor) {
                report.insecure.push((ancestor.to_path_buf(), reason));
            }
        }
    }
    report
}

/// Report, in one warning, every ancestor of `parent` another local user could
/// replace — and with it the `0600` file `subject` names below it.
///
/// Advisory only: the directories above the ones this crate creates are not
/// this crate's to police, so neither a finding nor a failed walk ever fails
/// the caller. Ancestors that cannot be inspected (sandboxed or unreadable
/// path components are normal on mobile platforms) are logged at `debug`.
#[cfg(unix)]
pub(crate) fn warn_on_insecure_ancestors(subject: &str, parent: &Path) {
    let report = insecure_ancestors(parent);
    if !report.insecure.is_empty() {
        let findings = report
            .insecure
            .iter()
            .map(|(ancestor, reason)| insecure_ancestor_message(subject, ancestor, reason))
            .collect::<Vec<_>>()
            .join("; ");
        tracing::warn!(
            subject,
            directory = %parent.display(),
            findings = %findings,
            "insecure ancestor directory permissions: another local user may be able to \
             replace wallet files below them; continuing"
        );
    }
    if !report.uninspected.is_empty() {
        let skipped = report
            .uninspected
            .iter()
            .map(|(ancestor, error)| format!("{}: {error}", ancestor.display()))
            .collect::<Vec<_>>()
            .join("; ");
        tracing::debug!(
            subject,
            directory = %parent.display(),
            skipped = %skipped,
            "ancestor directory permission report is incomplete: some ancestors could not be inspected"
        );
    }
}

// Windows ACL checks require platform-specific security APIs; tracked by
// https://github.com/dashpay/platform/issues/3754.
#[cfg(not(unix))]
pub(crate) fn warn_on_insecure_ancestors(_subject: &str, _parent: &Path) {}

#[cfg(all(test, unix))]
mod tests {
    use super::{
        classify_ancestor, insecure_ancestor_message, trusted_owner, writable_without_sticky,
        InsecureAncestor,
    };
    use std::path::Path;

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

    #[test]
    fn should_name_the_ancestor_and_the_command_that_fixes_it() {
        let writable = insecure_ancestor_message(
            "vault",
            Path::new("/parent"),
            &InsecureAncestor::WritableWithoutSticky { mode: 0o777 },
        );
        assert!(writable.contains("0777"));
        assert!(writable.contains("chmod go-w /parent"));

        // An ownership finding must NOT hand out `chmod`: the mode is very
        // likely unremarkable and changing it would not help.
        let owned = insecure_ancestor_message(
            "vault",
            Path::new("/parent"),
            &InsecureAncestor::UntrustedOwner {
                uid: 999,
                current_uid: 1000,
            },
        );
        assert!(owned.contains("chown 1000 /parent"));
        assert!(!owned.contains("chmod"), "got {owned}");
    }

    /// A foreign-owned ancestor that is also writable needs both fixes: its
    /// owner can undo a `chmod`, and the current user may not be able to run one.
    #[test]
    fn should_report_both_owner_and_mode_of_a_foreign_owned_writable_ancestor() {
        let reason = classify_ancestor(2000, 0o777, true, 1000, 0)
            .expect("a foreign-owned writable ancestor must be reported");
        assert_eq!(
            reason,
            InsecureAncestor::UntrustedOwnerAndWritable {
                uid: 2000,
                current_uid: 1000,
                mode: 0o777,
            }
        );

        let message = insecure_ancestor_message("vault", Path::new("/parent"), &reason);
        assert!(message.contains("uid 2000"), "got {message}");
        assert!(message.contains("0777"), "got {message}");
        assert!(message.contains("chown 1000 /parent"), "got {message}");
        assert!(message.contains("chmod go-w /parent"), "got {message}");
    }

    #[test]
    fn should_classify_by_the_single_condition_present() {
        assert_eq!(
            classify_ancestor(1000, 0o777, true, 1000, 0),
            Some(InsecureAncestor::WritableWithoutSticky { mode: 0o777 })
        );
        assert_eq!(
            classify_ancestor(2000, 0o755, false, 1000, 0),
            Some(InsecureAncestor::UntrustedOwner {
                uid: 2000,
                current_uid: 1000,
            })
        );
        assert_eq!(classify_ancestor(0, 0o755, false, 1000, 0), None);
    }

    /// A component the walk cannot stat must not discard what it already
    /// found: a missing directory behind `..` fails every stat below it.
    #[test]
    #[tracing_test::traced_test]
    fn should_warn_about_findings_collected_around_an_uninspectable_ancestor() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let writable = dir.path().join("writable");
        std::fs::create_dir(&writable).unwrap();
        std::fs::set_permissions(&writable, std::fs::Permissions::from_mode(0o777)).unwrap();
        let parent = writable.join("missing").join("..").join("wallet");

        super::warn_on_insecure_ancestors("database", &parent);

        assert!(logs_contain("insecure ancestor directory permissions"));
        assert!(logs_contain(&format!("chmod go-w {}", writable.display())));
        assert!(logs_contain("could not be inspected"));
    }
}
