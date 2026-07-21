#![allow(clippy::field_reassign_with_default)]

//! Schema-history-presence and max-version gates must bind to the
//! STAGED copy, not the first source handle.
//!
//! These regression tests pin that a forward-version or
//! schema-history-missing source is rejected AND the live destination
//! is left untouched, closing the validate-then-reopen TOCTOU window.

mod common;

use std::fs;

use common::{ensure_wallet_meta, fresh_persister, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet_storage::{SqlitePersister, WalletStorageError};

fn seed_one_row(persister: &SqlitePersister, w: &[u8; 32]) {
    ensure_wallet_meta(persister, w);
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(5),
        last_processed_height: Some(5),
        ..Default::default()
    });
    persister.store(*w, cs).unwrap();
}

const SENTINEL: &[u8] = b"do-not-replace-me";

/// A source whose `refinery_schema_history` MAX(version) exceeds the
/// embedded max is rejected and the destination is left untouched.
#[test]
fn forward_version_rejected_destination_unchanged() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xF0));
    let backup_path = persister.backup_to(tmp.path()).unwrap();
    drop(persister);

    // Bump the staged source past the embedded max so the staged-copy
    // gate must reject it.
    let bumped = 1_000_000i64;
    {
        let conn = rusqlite::Connection::open(&backup_path).unwrap();
        conn.execute(
            "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) \
             VALUES (?1, 'future', '', '0')",
            rusqlite::params![bumped],
        )
        .unwrap();
    }

    let dest = tmp.path().join("dest.db");
    fs::write(&dest, SENTINEL).unwrap();
    let err = SqlitePersister::restore_from_skip_backup(&dest, &backup_path);
    assert!(
        matches!(
            err,
            Err(WalletStorageError::SchemaVersionUnsupported { .. })
        ),
        "expected SchemaVersionUnsupported, got {err:?}"
    );
    assert_eq!(
        fs::read(&dest).unwrap(),
        SENTINEL,
        "destination must be untouched when the staged copy is rejected"
    );
}

/// A source lacking `refinery_schema_history` but otherwise
/// integrity-valid is rejected and the destination is left untouched.
#[test]
fn missing_schema_history_rejected_destination_unchanged() {
    let tmp = common::secure_tempdir().unwrap();
    let fake_src = tmp.path().join("empty.db");
    rusqlite::Connection::open(&fake_src).unwrap();

    let dest = tmp.path().join("dest.db");
    fs::write(&dest, SENTINEL).unwrap();
    let err = SqlitePersister::restore_from_skip_backup(&dest, &fake_src);
    assert!(
        matches!(err, Err(WalletStorageError::SchemaHistoryMissing)),
        "expected SchemaHistoryMissing, got {err:?}"
    );
    assert_eq!(
        fs::read(&dest).unwrap(),
        SENTINEL,
        "destination must be untouched when the staged copy is rejected"
    );
}

/// If the staged copy fails its forward-version gate, the
/// destination's `<dest>-wal` / `<dest>-shm` siblings must NOT be
/// unlinked. Deleting them before validation succeeds = un-checkpointed
/// committed pages lost on rollback.
#[test]
fn rejected_restore_leaves_wal_shm_siblings_intact() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xF1));
    let backup_path = persister.backup_to(tmp.path()).unwrap();
    drop(persister);

    // Bump the source past the embedded max.
    let bumped = 1_000_000i64;
    {
        let conn = rusqlite::Connection::open(&backup_path).unwrap();
        conn.execute(
            "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) \
             VALUES (?1, 'future', '', '0')",
            rusqlite::params![bumped],
        )
        .unwrap();
    }

    let dest = tmp.path().join("dest.db");
    fs::write(&dest, SENTINEL).unwrap();
    let wal = tmp.path().join("dest.db-wal");
    let shm = tmp.path().join("dest.db-shm");
    fs::write(&wal, b"wal-sentinel").unwrap();
    fs::write(&shm, b"shm-sentinel").unwrap();

    let err = SqlitePersister::restore_from_skip_backup(&dest, &backup_path);
    assert!(
        matches!(
            err,
            Err(WalletStorageError::SchemaVersionUnsupported { .. })
        ),
        "expected SchemaVersionUnsupported, got {err:?}"
    );
    assert_eq!(fs::read(&dest).unwrap(), SENTINEL, "main DB preserved");
    assert!(
        wal.exists(),
        "WAL sibling must NOT be unlinked on rejection"
    );
    assert!(
        shm.exists(),
        "SHM sibling must NOT be unlinked on rejection"
    );
    assert_eq!(fs::read(&wal).unwrap(), b"wal-sentinel");
    assert_eq!(fs::read(&shm).unwrap(), b"shm-sentinel");
}

/// A forward-version source must fail BEFORE the full file is streamed
/// into the destination's parent dir. We assert no
/// NamedTempFile from the staging copy survives in the parent dir
/// after the rejection — the cheap source-side sniff fails fast.
#[test]
fn forward_version_rejected_before_staging() {
    let (persister, tmp, _path) = fresh_persister();
    seed_one_row(&persister, &wid(0xF3));
    let backup_path = persister.backup_to(tmp.path()).unwrap();
    drop(persister);

    // Bump the source past the embedded max.
    {
        let conn = rusqlite::Connection::open(&backup_path).unwrap();
        conn.execute(
            "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) \
             VALUES (?1, 'future', '', '0')",
            rusqlite::params![1_000_000i64],
        )
        .unwrap();
    }

    let dest_dir = common::secure_tempdir().unwrap();
    let dest = dest_dir.path().join("dest.db");
    fs::write(&dest, SENTINEL).unwrap();

    let before: usize = fs::read_dir(dest_dir.path()).unwrap().count();
    let err = SqlitePersister::restore_from_skip_backup(&dest, &backup_path);
    let after: usize = fs::read_dir(dest_dir.path()).unwrap().count();

    assert!(
        matches!(
            err,
            Err(WalletStorageError::SchemaVersionUnsupported { .. })
        ),
        "expected SchemaVersionUnsupported, got {err:?}"
    );
    assert_eq!(
        after, before,
        "no staging temp file should be left behind on pre-staging rejection"
    );
}

/// Happy path: a valid in-range backup still restores and the
/// destination reflects the restored bytes.
#[test]
fn valid_backup_roundtrips() {
    let (persister, tmp, path) = fresh_persister();
    let w = wid(0xF2);
    seed_one_row(&persister, &w);
    let backup_path = persister.backup_to(tmp.path()).unwrap();
    let mut cs = PlatformWalletChangeSet::default();
    cs.core = Some(CoreChangeSet {
        synced_height: Some(999),
        last_processed_height: Some(999),
        ..Default::default()
    });
    persister.store(w, cs).unwrap();
    drop(persister);

    SqlitePersister::restore_from_skip_backup(&path, &backup_path).expect("restore_from");

    let cfg = platform_wallet_storage::SqlitePersisterConfig::new(&path);
    let p2 = SqlitePersister::open(cfg).unwrap();
    let conn = p2.lock_conn_for_test();
    let h: i64 = conn
        .query_row(
            "SELECT synced_height FROM core_sync_state WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        h, 5,
        "restored bytes must reflect the backup, not the mutation"
    );
}
