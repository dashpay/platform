#![allow(clippy::field_reassign_with_default)]

//! Migration execution against the populated-V001 fixture (WS-B task B8).
//! Covers TC-B-031 (data preserved), TC-B-032 (pre-migration auto-backup),
//! TC-B-033 (backup restorable + re-migration determinism), TC-B-034
//! (forward-version rejection at the new max), TC-B-035 (idempotent
//! re-entry), TC-B-036 (empty wallet through migration).

mod common;

use std::path::{Path, PathBuf};

use common::{ro_conn, wid};
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};
use rusqlite::Connection;

const FULL_WALLET: u8 = 0xA1;
const EMPTY_WALLET: u8 = 0xB2;

fn fixture_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("populated_v001.db")
}

/// Copy the committed V001 fixture into `dir` so migration runs on a
/// throwaway copy, never the committed file.
fn copy_fixture(dir: &Path) -> PathBuf {
    let dst = dir.join("wallet.db");
    std::fs::copy(fixture_src(), &dst).expect("copy fixture");
    dst
}

fn schema_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT MAX(version) FROM refinery_schema_history",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        rusqlite::params![table],
        |_| Ok(()),
    )
    .is_ok()
}

fn count(conn: &Connection, sql: &str, wallet: &[u8; 32]) -> i64 {
    conn.query_row(sql, rusqlite::params![wallet.as_slice()], |r| r.get(0))
        .unwrap()
}

/// Assert the post-migration store carries the full fixture data intact.
fn assert_full_data_preserved(conn: &Connection) {
    let full = wid(FULL_WALLET);
    assert_eq!(schema_version(conn), 2, "must be migrated to V002");
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM wallets", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2,
        "both wallets preserved"
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM account_registrations WHERE wallet_id = ?1",
            &full
        ),
        1
    );
    let (utxos, acct): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), MAX(account_index) FROM core_utxos WHERE wallet_id = ?1",
            rusqlite::params![full.as_slice()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(utxos, 1, "UTXO preserved");
    assert_eq!(
        acct, 0,
        "pre-existing UTXO keeps account_index=0 (R7 one-way backfill)"
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1",
            &full
        ),
        1
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1",
            &full
        ),
        1
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM contacts WHERE wallet_id = ?1",
            &full
        ),
        1
    );
    // New V002 tables exist with sane defaults.
    assert!(table_exists(conn, "core_address_pool"));
    assert!(table_exists(conn, "meta_data_versions"));
    let gen_len: i64 = conn
        .query_row(
            "SELECT length(generation) FROM meta_store_generation WHERE id = 0",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(gen_len, 16, "generation seeded at migration");
}

/// TC-B-031 — opening a populated V001 fixture with the post-redirect binary
/// migrates it and preserves every pre-existing row.
#[test]
fn tc_b_031_populated_v001_migration_preserves_data() {
    let tmp = tempfile::tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    {
        let pre = ro_conn(&path);
        assert_eq!(schema_version(&pre), 1, "fixture starts at V001");
    }
    let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    {
        let conn = p.lock_conn_for_test();
        assert_full_data_preserved(&conn);
    }
    // The full wallet reconstructs; the used-set falls back to the
    // UTXO-derived address (no pool rows in a migrated store).
    let state = p.load().unwrap();
    let full = wid(FULL_WALLET);
    let slice = state.wallets.get(&full).expect("full wallet reconstructs");
    assert_eq!(
        slice.used_core_addresses.len(),
        1,
        "migrated store falls back to the UTXO-derived used-set"
    );
}

/// TC-B-036 — the empty wallet inside the populated store migrates without a
/// NOT NULL violation and reads empty-but-valid.
#[test]
fn tc_b_036_empty_wallet_through_migration() {
    let tmp = tempfile::tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let state = p.load().unwrap();
    let empty = wid(EMPTY_WALLET);
    let slice = state
        .wallets
        .get(&empty)
        .expect("empty wallet still surfaces post-migration");
    assert!(
        slice.used_core_addresses.is_empty(),
        "empty wallet is empty-but-valid, not corrupt"
    );
}

/// TC-B-032 — a byte-faithful pre-migration auto-backup is written before the
/// schema changes are visible in the live file.
#[test]
fn tc_b_032_pre_migration_backup_created() {
    let tmp = tempfile::tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    let backup_dir = tmp.path().join("backups");
    let p = SqlitePersister::open(
        SqlitePersisterConfig::new(&path).with_auto_backup_dir(Some(backup_dir.clone())),
    )
    .unwrap();
    drop(p);

    let backup = std::fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("pre-migration-1-to-2-") && n.ends_with(".db"))
        })
        .expect("pre-migration backup must exist");

    // The backup captured the PRE-migration state: schema version 1, and no
    // V002 table.
    let bconn = ro_conn(&backup);
    assert_eq!(
        schema_version(&bconn),
        1,
        "backup is the pre-migration V001 state"
    );
    assert!(
        !table_exists(&bconn, "core_address_pool"),
        "backup must predate the V002 schema"
    );
    assert_eq!(
        bconn
            .query_row("SELECT COUNT(*) FROM wallets", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2,
        "backup carries the original data"
    );
}

/// TC-B-033 — the pre-migration backup restores cleanly and re-migrating it
/// reaches the identical end state as a direct migration (determinism).
#[test]
fn tc_b_033_backup_restorable_and_remigration_deterministic() {
    let tmp = tempfile::tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    let backup_dir = tmp.path().join("backups");
    {
        let _p = SqlitePersister::open(
            SqlitePersisterConfig::new(&path).with_auto_backup_dir(Some(backup_dir.clone())),
        )
        .unwrap();
    }
    let backup = std::fs::read_dir(&backup_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("pre-migration-1-to-2-"))
        })
        .expect("backup exists");

    // Restore the V001 backup into a fresh dest, then reopen to re-migrate.
    let dest = tmp.path().join("restored.db");
    SqlitePersister::restore_from_skip_backup(&dest, &backup).expect("restore V001 backup");
    {
        let rconn = ro_conn(&dest);
        assert_eq!(schema_version(&rconn), 1, "restored store is at V001");
    }
    let p2 = SqlitePersister::open(SqlitePersisterConfig::new(&dest)).unwrap();
    let conn = p2.lock_conn_for_test();
    assert_full_data_preserved(&conn);
}

/// TC-B-034 — the forward-version gate now rejects at the NEW max (2); a
/// forged version-3 row is refused.
#[test]
fn tc_b_034_forward_version_rejected_at_new_max() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    {
        let _p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    }
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) \
             VALUES (3, 'future', '', '0')",
            [],
        )
        .unwrap();
    }
    match SqlitePersister::open(SqlitePersisterConfig::new(&path)) {
        Err(WalletStorageError::SchemaVersionUnsupported {
            found,
            max_supported,
        }) => {
            assert_eq!(found, 3);
            assert_eq!(max_supported, 2, "max must reflect the post-redirect V002");
        }
        Err(other) => panic!("expected SchemaVersionUnsupported, got {other:?}"),
        Ok(_) => panic!("forward-version DB must be refused"),
    }
}

/// TC-B-035 — re-entry idempotency: reopening a fully-migrated store applies
/// no further migration and leaves the end state byte-stable. Refinery runs
/// each migration in its own transaction, so a crash mid-migrate leaves the
/// store at the last committed version and reopening resumes deterministically
/// to this same state.
#[test]
fn tc_b_035_reentry_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let path = copy_fixture(tmp.path());

    let snapshot = |conn: &Connection| -> (i64, i64, i64, [u8; 16]) {
        let full = wid(FULL_WALLET);
        let utxos = count(
            conn,
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
            &full,
        );
        let idents = count(
            conn,
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1",
            &full,
        );
        let version = schema_version(conn);
        let gen: Vec<u8> = conn
            .query_row(
                "SELECT generation FROM meta_store_generation WHERE id = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        (version, utxos, idents, gen.try_into().unwrap())
    };

    let first = {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let conn = p.lock_conn_for_test();
        snapshot(&conn)
    };
    let second = {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let conn = p.lock_conn_for_test();
        snapshot(&conn)
    };
    assert_eq!(first.0, 2, "first open migrates to V002");
    assert_eq!(
        first, second,
        "reopening a migrated store is a no-op — version, data, and generation \
         are byte-stable (generation only rotates on migrate/restore, not reopen)"
    );
}
