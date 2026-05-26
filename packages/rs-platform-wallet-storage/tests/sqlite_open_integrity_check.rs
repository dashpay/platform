#![allow(clippy::field_reassign_with_default)]

//! ATOM-013 (A-8) — `open()` runs `PRAGMA integrity_check` on a
//! pre-existing DB BEFORE migrations alter it, so bit-rot / escaped-
//! WAL corruption surfaces as the typed `IntegrityCheckFailed` instead
//! of being silently migrated (and snapshotted into the pre-migration
//! auto-backup, defeating rollback).

mod common;

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};

use common::fresh_persister;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

/// Deliberately corrupt the SQLite file at `path` by flipping bytes
/// well past the 100-byte header (where the schema/btree pages live).
/// We avoid the header so the file still opens as SQLite — the
/// integrity_check catches the structural rot.
fn corrupt_btree_pages(path: &std::path::Path) {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open db for corruption");
    let len = f.metadata().unwrap().len();
    assert!(len > 4096, "expected at least one full page");
    // Read page 2 (bytes 4096..8192), flip every other byte, write back.
    f.seek(SeekFrom::Start(4096)).unwrap();
    let mut buf = vec![0u8; 4096];
    f.read_exact(&mut buf).unwrap();
    for b in buf.iter_mut().step_by(2) {
        *b ^= 0xFF;
    }
    f.seek(SeekFrom::Start(4096)).unwrap();
    f.write_all(&buf).unwrap();
    f.sync_all().unwrap();
}

/// ATOM-013: opening a corrupt DB returns `IntegrityCheckFailed`
/// instead of running migrations against it.
#[test]
fn atom_013_open_rejects_corrupt_db() {
    let (persister, tmp, path) = fresh_persister();
    // Add at least one user row so there's content to corrupt past the header.
    {
        use rusqlite::params;
        let conn = persister.lock_conn_for_test();
        // Push the DB past a few pages with a chunky meta row.
        for i in 0..20u32 {
            conn.execute(
                "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', ?2)",
                params![vec![i as u8; 32].as_slice(), i as i64],
            )
            .unwrap();
        }
    }
    drop(persister);

    corrupt_btree_pages(&path);

    let cfg = SqlitePersisterConfig::new(&path);
    let res = SqlitePersister::open(cfg);
    let err = match res {
        Ok(_) => panic!("open must reject corrupt DB"),
        Err(e) => e,
    };
    assert!(
        matches!(err, WalletStorageError::IntegrityCheckFailed { .. }),
        "expected IntegrityCheckFailed, got {err:?}"
    );
    drop(tmp);
}
