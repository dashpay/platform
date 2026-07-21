//! Owner-only permissions on the live DB AND its `-wal` / `-shm`
//! sidecars. SQLite's default WAL journal mode keeps
//! recent committed pages in the sidecars, so leaving them at the
//! process umask leaks wallet state on multi-user hosts.

#![cfg(unix)]
#![allow(clippy::field_reassign_with_default)]

mod common;

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;

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
