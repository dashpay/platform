#![allow(clippy::field_reassign_with_default)]

//! Store-generation token behaviour (WS-B task B5). Covers TC-B-004
//! (present + stable across a normal flush) and TC-B-024 (regenerated on
//! restore so a restored copy is distinguishable from its source).

mod common;

use common::{ensure_wallet_meta, fresh_persister, ro_conn, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::sqlite::schema::versions;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

/// TC-B-004 — the generation is present, 16 bytes, and unchanged by a normal
/// changeset flush (it only rotates on migrate/restore).
#[test]
fn tc_b_004_generation_present_and_stable_across_flush() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x01);
    ensure_wallet_meta(&persister, &w);

    let g1 = {
        let conn = persister.lock_conn_for_test();
        versions::read_generation(&conn)
            .unwrap()
            .expect("fresh V002 store carries a generation")
    };
    assert!(
        g1.iter().any(|b| *b != 0),
        "generation must not be all-zero"
    );

    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    synced_height: Some(10),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let g2 = {
        let conn = persister.lock_conn_for_test();
        versions::read_generation(&conn).unwrap().unwrap()
    };
    assert_eq!(g1, g2, "a normal flush must not rotate the generation");
    drop(persister);
    let _ = path;
}

/// TC-B-024 — restoring from a backup rotates the generation, so a client
/// cache keyed on the pre-restore generation misses rather than serving
/// stale entries.
#[test]
fn tc_b_024_generation_rotates_on_restore() {
    let (persister, tmp, path) = fresh_persister();
    let w = wid(0x02);
    ensure_wallet_meta(&persister, &w);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    synced_height: Some(5),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let g1 = {
        let conn = persister.lock_conn_for_test();
        versions::read_generation(&conn).unwrap().unwrap()
    };
    let backup_path = persister.backup_to(tmp.path()).unwrap();
    // The backup is a byte-copy, so it carries the same generation.
    {
        let bconn = ro_conn(&backup_path);
        assert_eq!(
            versions::read_generation(&bconn).unwrap().unwrap(),
            g1,
            "backup carries the source generation verbatim"
        );
    }
    drop(persister);

    SqlitePersister::restore_from_skip_backup(&path, &backup_path).expect("restore");

    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let g2 = {
        let conn = p2.lock_conn_for_test();
        versions::read_generation(&conn).unwrap().unwrap()
    };
    assert_ne!(
        g1, g2,
        "restore must rotate the generation (restored copy != source)"
    );
    drop(p2);
    drop(tmp);
}
