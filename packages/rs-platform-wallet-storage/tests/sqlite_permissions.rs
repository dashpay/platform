//! CMT-003 / CMT-004 — owner-only permissions on the live DB AND its
//! `-wal` / `-shm` sidecars. SQLite's default WAL journal mode keeps
//! recent committed pages in the sidecars, so leaving them at the
//! process umask leaks wallet state on multi-user hosts.

#![cfg(unix)]
#![allow(clippy::field_reassign_with_default)]

mod common;

use std::os::unix::fs::PermissionsExt;

use common::{ensure_wallet_meta, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

#[test]
fn wal_and_shm_sidecars_are_chmodded_0o600() {
    let tmp = tempfile::tempdir().unwrap();
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
