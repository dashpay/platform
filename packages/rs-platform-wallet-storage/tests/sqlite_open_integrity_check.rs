#![allow(clippy::field_reassign_with_default)]

//! `open()` runs `PRAGMA integrity_check` on a pre-existing DB BEFORE
//! migrations alter it, so bit-rot / escaped-WAL corruption surfaces
//! as the typed `IntegrityCheckFailed` instead of being silently
//! migrated (and snapshotted into the pre-migration auto-backup,
//! defeating rollback).

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
    assert!(
        len >= 8192,
        "need at least two full pages to corrupt page 2; got {len} bytes"
    );
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

/// Opening a corrupt DB returns `IntegrityCheckFailed` instead of
/// running migrations against it.
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

/// Flip bytes inside pages 2 AND 3 so SQLite's `PRAGMA integrity_check`
/// has multiple problems to report. This widens the surface beyond the
/// single-row "ok" case so the multi-row collection path is exercised.
fn corrupt_multiple_pages(path: &std::path::Path) {
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("open db for multi-page corruption");
    let len = f.metadata().unwrap().len();
    // We corrupt two full 4096-byte pages starting at offsets 4096 and
    // 8192; `read_exact` at offset 8192 needs the file to be at least
    // THREE full pages long, so require >= 12_288 bytes.
    assert!(
        len >= 12_288,
        "expected at least three full pages ({} bytes); got {len}",
        12_288
    );
    for page_start in [4096u64, 8192] {
        f.seek(SeekFrom::Start(page_start)).unwrap();
        let mut buf = vec![0u8; 4096];
        f.read_exact(&mut buf).unwrap();
        for b in buf.iter_mut().step_by(2) {
            *b ^= 0xFF;
        }
        f.seek(SeekFrom::Start(page_start)).unwrap();
        f.write_all(&buf).unwrap();
    }
    f.sync_all().unwrap();
}

/// A multi-problem DB surfaces every diagnostic line in
/// `IntegrityCheckFailed::report` — the collection must not stop at the
/// first row. We assert the report is non-empty and not the truncated
/// single-row "ok" sentinel; we don't bind to a fixed line count
/// because SQLite's exact diagnostic shape isn't stable across builds.
#[test]
fn tc_code_016_a_integrity_report_collects_all_rows() {
    let (persister, tmp, path) = fresh_persister();
    {
        use rusqlite::params;
        let conn = persister.lock_conn_for_test();
        for i in 0..40u32 {
            conn.execute(
                "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', ?2)",
                params![vec![i as u8; 32].as_slice(), i as i64],
            )
            .unwrap();
        }
    }
    drop(persister);

    corrupt_multiple_pages(&path);

    let cfg = SqlitePersisterConfig::new(&path);
    let err = match SqlitePersister::open(cfg) {
        Ok(_) => panic!("open must reject multi-page corrupt DB"),
        Err(e) => e,
    };
    let report = match err {
        WalletStorageError::IntegrityCheckFailed { report } => report,
        other => panic!("expected IntegrityCheckFailed, got {other:?}"),
    };
    assert!(!report.is_empty(), "report must be non-empty");
    assert_ne!(
        report.trim(),
        "ok",
        "report must NOT be the healthy sentinel"
    );
    // Source-level regression: the helper must use `query_map`, not
    // `query_row`, so multi-row reports are preserved.
    let helper_src = include_str!("../src/sqlite/backup.rs");
    assert!(
        helper_src.contains("PRAGMA integrity_check") && helper_src.contains("query_map"),
        "run_integrity_check must use query_map (not query_row) to collect every diagnostic row"
    );
    drop(tmp);
}
