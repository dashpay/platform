//! Owner-only permissions on the live DB AND its `-wal` / `-shm`
//! sidecars. SQLite's default WAL journal mode keeps
//! recent committed pages in the sidecars, so leaving them at the
//! process umask leaks wallet state on multi-user hosts.

#![cfg(unix)]
#![allow(clippy::field_reassign_with_default)]

mod common;

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::symlink;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use common::{ensure_wallet_meta, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

#[test]
fn open_rejects_group_or_other_writable_parent() {
    let tmp = common::secure_tempdir().unwrap();
    let parent = tmp.path().join("insecure");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o777)).unwrap();
    let db_path = parent.join("wallet.db");

    let result = SqlitePersister::open(SqlitePersisterConfig::new(&db_path));

    assert!(
        matches!(
            result,
            Err(WalletStorageError::InsecureParentDir { mode }) if mode & 0o022 != 0
        ),
        "open must return the typed insecure-parent error"
    );
    assert!(!db_path.exists(), "the database must not be pre-created");
}

#[test]
fn open_rejects_parent_owned_by_another_user_when_chown_is_permitted() {
    let tmp = common::secure_tempdir().unwrap();
    let parent = tmp.path().join("foreign-owner");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();

    // SAFETY: `geteuid` takes no arguments and cannot fail.
    let current_uid = unsafe { libc::geteuid() };
    let root_uid = std::fs::metadata("/").unwrap().uid();
    let foreign_uid = (1..=u16::MAX as u32)
        .find(|uid| *uid != current_uid && *uid != root_uid)
        .unwrap();
    let path = std::ffi::CString::new(parent.as_os_str().as_bytes()).unwrap();
    // SAFETY: `path` is a live NUL-terminated string and both IDs are valid values.
    let result = unsafe { libc::chown(path.as_ptr(), foreign_uid, !0 as libc::gid_t) };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if matches!(error.raw_os_error(), Some(libc::EPERM) | Some(libc::EINVAL)) {
            return;
        }
        panic!("chown fixture failed: {error}");
    }

    let db_path = parent.join("wallet.db");
    let result = SqlitePersister::open(SqlitePersisterConfig::new(&db_path));

    assert!(matches!(
        result,
        Err(WalletStorageError::InsecureParentDir { mode }) if mode == 0o755
    ));
    assert!(!db_path.exists(), "the database must not be pre-created");
}

#[test]
fn open_rejects_writable_non_sticky_ancestor_above_secure_parent() {
    let tmp = common::secure_tempdir().unwrap();
    let insecure_ancestor = tmp.path().join("replaceable");
    let secure_parent = insecure_ancestor.join("wallet");
    std::fs::create_dir_all(&secure_parent).unwrap();
    std::fs::set_permissions(&insecure_ancestor, std::fs::Permissions::from_mode(0o777)).unwrap();
    std::fs::set_permissions(&secure_parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    let db_path = secure_parent.join("wallet.db");

    let result = SqlitePersister::open(SqlitePersisterConfig::new(&db_path));

    assert!(matches!(
        result,
        Err(WalletStorageError::InsecureParentDir { mode }) if mode & 0o022 != 0
    ));
    assert!(
        !db_path.exists(),
        "an insecure ancestor must fail before create"
    );
}

#[test]
fn open_rejects_insecure_ancestor_reached_through_parent_symlink() {
    let tmp = common::secure_tempdir().unwrap();
    let insecure_target_ancestor = tmp.path().join("replaceable-target");
    let secure_target_parent = insecure_target_ancestor.join("wallet");
    std::fs::create_dir_all(&secure_target_parent).unwrap();
    std::fs::set_permissions(
        &insecure_target_ancestor,
        std::fs::Permissions::from_mode(0o777),
    )
    .unwrap();
    std::fs::set_permissions(
        &secure_target_parent,
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let linked_parent = tmp.path().join("wallet-link");
    symlink(&secure_target_parent, &linked_parent).unwrap();
    let db_path = linked_parent.join("wallet.db");

    let result = SqlitePersister::open(SqlitePersisterConfig::new(&db_path));

    assert!(matches!(
        result,
        Err(WalletStorageError::InsecureParentDir { mode }) if mode & 0o022 != 0
    ));
    assert!(
        !secure_target_parent.join("wallet.db").exists(),
        "the resolved target ancestor must be checked before create"
    );
}

#[test]
fn open_accepts_sticky_writable_parent() {
    let tmp = common::secure_tempdir().unwrap();
    let parent = tmp.path().join("shared");
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o1777)).unwrap();
    let db_path = parent.join("wallet.db");

    // The sticky bit protects EXISTING entries only: another uid cannot
    // unlink or rename what it does not own, but it can still create a new
    // name that does not yet exist. The create case is covered by the
    // DB-path symlink refusal, not by this acceptance.
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&db_path))
        .expect("a sticky world-writable parent stays acceptable");
    drop(persister);
    assert!(db_path.exists());
}

/// `O_CREAT|O_EXCL` reports EEXIST for a planted symlink exactly as it does
/// for a legitimate database, so EEXIST cannot be treated as "re-open".
/// Both the SQLite open and the `0o600` chmod would otherwise resolve the
/// link and land on its target.
#[test]
fn open_rejects_a_symlinked_database_path() {
    let tmp = common::secure_tempdir().unwrap();
    let victim = tmp.path().join("victim.txt");
    std::fs::write(&victim, b"victim contents").unwrap();
    std::fs::set_permissions(&victim, std::fs::Permissions::from_mode(0o644)).unwrap();
    let db_path = tmp.path().join("wallet.db");
    symlink(&victim, &db_path).unwrap();

    let result = SqlitePersister::open(SqlitePersisterConfig::new(&db_path));

    assert!(
        matches!(
            result,
            Err(WalletStorageError::DatabasePathIsSymlink { .. })
        ),
        "open must refuse a symlinked database path with the typed error"
    );
    assert_eq!(
        std::fs::read(&victim).unwrap(),
        b"victim contents",
        "the link target's contents must be untouched"
    );
    assert_eq!(
        std::fs::metadata(&victim).unwrap().mode() & 0o777,
        0o644,
        "the owner-only chmod must not have followed the link"
    );
}

/// The same redirect on the restore destination. `exists()` follows a link
/// to a live target, so the placeholder block cannot be the thing that
/// catches it.
#[test]
fn restore_rejects_a_symlinked_destination() {
    let tmp = common::secure_tempdir().unwrap();
    let source_path = tmp.path().join("source.db");
    let source = SqlitePersister::open(SqlitePersisterConfig::new(&source_path)).unwrap();
    let backup = source.backup_to(tmp.path()).unwrap();
    drop(source);

    let victim = tmp.path().join("victim.txt");
    std::fs::write(&victim, b"victim contents").unwrap();
    let destination = tmp.path().join("restored.db");
    symlink(&victim, &destination).unwrap();

    let result = SqlitePersister::restore_from_skip_backup(&destination, &backup);

    assert!(
        matches!(
            result,
            Err(WalletStorageError::DatabasePathIsSymlink { .. })
        ),
        "restore must refuse a symlinked destination with the typed error"
    );
    assert_eq!(
        std::fs::read(&victim).unwrap(),
        b"victim contents",
        "the link target must not be restored over"
    );
}

#[test]
fn restore_rejects_insecure_destination_parent() {
    let tmp = common::secure_tempdir().unwrap();
    let source_path = tmp.path().join("source.db");
    let source = SqlitePersister::open(SqlitePersisterConfig::new(&source_path)).unwrap();
    let backup = source.backup_to(tmp.path()).unwrap();
    drop(source);

    let insecure_parent = tmp.path().join("restore-target");
    std::fs::create_dir(&insecure_parent).unwrap();
    std::fs::set_permissions(&insecure_parent, std::fs::Permissions::from_mode(0o777)).unwrap();
    let destination = insecure_parent.join("restored.db");

    let result = SqlitePersister::restore_from_skip_backup(&destination, &backup);

    assert!(matches!(
        result,
        Err(WalletStorageError::InsecureParentDir { mode }) if mode & 0o022 != 0
    ));
    assert!(
        !destination.exists(),
        "restore must reject the parent before staging or replacing the destination"
    );
}

#[test]
fn restore_checks_destination_permissions_before_auto_backup_policy() {
    let tmp = common::secure_tempdir().unwrap();
    let source_path = tmp.path().join("source-for-ordinary-restore.db");
    let source = SqlitePersister::open(SqlitePersisterConfig::new(&source_path)).unwrap();
    let backup = source.backup_to(tmp.path()).unwrap();
    drop(source);

    let insecure_parent = tmp.path().join("ordinary-restore-target");
    std::fs::create_dir(&insecure_parent).unwrap();
    std::fs::set_permissions(&insecure_parent, std::fs::Permissions::from_mode(0o777)).unwrap();
    let destination = insecure_parent.join("restored.db");
    std::fs::write(&destination, b"existing destination").unwrap();

    let result = SqlitePersister::restore_from(&destination, &backup, None);

    assert!(matches!(
        result,
        Err(WalletStorageError::InsecureParentDir { mode }) if mode & 0o022 != 0
    ));
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"existing destination",
        "the gate must run before opening or backing up the destination"
    );
}

#[test]
fn wal_and_shm_sidecars_are_chmodded_0o600() {
    let tmp = common::secure_tempdir().unwrap();
    let db_path = tmp.path().join("wallet.db");
    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&db_path)).expect("open");

    // Seed the parent row and trigger a write so SQLite materializes
    // the WAL/SHM siblings.
    let w = wid(0xA1);
    ensure_wallet_meta(&persister, &w);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(5),
        last_processed_height: Some(5),
        ..Default::default()
    });
    persister.store(w, cs).expect("store");

    let wal = tmp.path().join("wallet.db-wal");
    let shm = tmp.path().join("wallet.db-shm");
    assert!(wal.exists(), "WAL sibling should exist after a write");
    assert!(shm.exists(), "SHM sibling should exist after a write");

    // Loosen sidecar perms behind the helper's back, then re-apply.
    // This isolates the sidecar-chmod codepath from whatever umask the
    // test runner happened to inherit.
    for path in [&wal, &shm] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666)).unwrap();
    }
    platform_wallet_storage::sqlite::util::permissions::apply_secure_permissions(&db_path)
        .expect("apply_secure_permissions");

    for path in [&db_path, &wal, &shm] {
        let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode,
            0o600,
            "expected 0o600 on {} after apply_secure_permissions, got {:o}",
            path.display(),
            mode
        );
    }
}

/// `apply_secure_permissions` survives a non-ASCII DB filename whose
/// bytes round-trip through `OsString` (the codepath
/// builds sidecar names via `OsString::push`, not `format!` over a
/// lossy `String`). The chosen prefix `ÿþ` (`U+00FF U+00FE`, UTF-8
/// bytes `c3 bf c3 be`) is multi-byte non-ASCII that both Linux and
/// macOS APFS accept — APFS rejects raw non-UTF-8 with `EILSEQ`, so
/// the bytes here are deliberately valid UTF-8 while still exercising
/// the `OsString`-end-to-end path the pre-fix `to_string_lossy()` would
/// have mangled into the wrong sibling names.
#[test]
fn tc_code_011_a_non_ascii_db_path_sidecars_chmodded() {
    let tmp = common::secure_tempdir().unwrap();
    // Valid-UTF-8 multi-byte prefix `ÿþ` + `.db` / `.db-wal` / `.db-shm`.
    // We still go through `OsString::from_vec` to mirror the production
    // codepath's `OsStr`/`OsString` API surface end-to-end.
    let prefix: &[u8] = &[0xC3, 0xBF, 0xC3, 0xBE]; // "ÿþ" in UTF-8
    debug_assert_eq!(std::str::from_utf8(prefix).unwrap(), "ÿþ");
    let mk = |suffix: &[u8]| -> OsString {
        let mut v = prefix.to_vec();
        v.extend_from_slice(suffix);
        OsString::from_vec(v)
    };
    let db_name = mk(b".db");
    let wal_name = mk(b".db-wal");
    let shm_name = mk(b".db-shm");
    let db_path = tmp.path().join(&db_name);
    let wal = tmp.path().join(&wal_name);
    let shm = tmp.path().join(&shm_name);
    // Plant the trio with permissive perms so the chmod is observable.
    for p in [&db_path, &wal, &shm] {
        std::fs::write(p, b"x").unwrap();
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o666)).unwrap();
    }

    platform_wallet_storage::sqlite::util::permissions::apply_secure_permissions(&db_path)
        .expect("apply_secure_permissions");

    for p in [&db_path, &wal, &shm] {
        let mode = std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode,
            0o600,
            "expected 0o600 on non-ASCII path {} after apply_secure_permissions, got {:o}",
            p.display(),
            mode
        );
    }
}

/// `apply_secure_permissions` is a no-op (Ok) when the sidecars don't
/// exist. The `set_permissions` call sees
/// `ErrorKind::NotFound` and swallows it — no `exists()` gate, no
/// race window.
#[test]
fn tc_code_011_b_no_sidecars_is_ok() {
    let tmp = common::secure_tempdir().unwrap();
    let db_path = tmp.path().join("solo.db");
    std::fs::write(&db_path, b"x").unwrap();
    // No -wal / -shm planted on purpose.
    platform_wallet_storage::sqlite::util::permissions::apply_secure_permissions(&db_path)
        .expect("apply_secure_permissions on solo DB must be Ok");
    let mode = std::fs::metadata(&db_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    // Source-level regression: the helper must NOT contain `exists(`
    // anywhere in its sibling-chmod path.
    let src = include_str!("../src/sqlite/util/permissions.rs");
    assert!(
        !src.contains("sibling.exists("),
        "permissions.rs must not pre-gate set_permissions on sibling.exists() (TOCTOU)"
    );
    assert!(
        !src.contains(".to_string_lossy().to_string()"),
        "permissions.rs must not build sibling paths via .to_string_lossy().to_string() (loses non-UTF-8 bytes)"
    );
}

/// The same OsString + NotFound-swallow pattern in `backup.rs`'s
/// WAL/SHM-unlink loop (DRY motif).
#[test]
fn tc_code_011_c_backup_wal_shm_unlink_no_lossy_no_exists_gate() {
    let src = include_str!("../src/sqlite/backup.rs");
    // The unlink loop now uses OsString::push, not to_string_lossy.
    // We can't structurally diff the loop, but the file must not
    // contain the lossy pattern on the sidecar build path.
    assert!(
        !src.contains("s.to_string_lossy().to_string()"),
        "backup.rs must not build sibling paths via to_string_lossy().to_string()"
    );
    // And remove_file must not be gated on sibling.exists().
    assert!(
        !src.contains("sibling.exists()"),
        "backup.rs WAL/SHM-unlink must not pre-gate remove_file on sibling.exists() (TOCTOU)"
    );
}
