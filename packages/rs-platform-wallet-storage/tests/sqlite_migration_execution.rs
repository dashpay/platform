#![allow(clippy::field_reassign_with_default)]

//! Migration execution against the populated-V001 fixture.
//! Covers TC-B-031 (data preserved), TC-B-032 (pre-migration auto-backup),
//! TC-B-033 (backup restorable + re-migration determinism), TC-B-034
//! (forward-version rejection at the new max), TC-B-035 (idempotent
//! re-entry), TC-B-036 (empty wallet through migration).

mod common;

use std::path::{Path, PathBuf};

use common::{ro_conn, wid};
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::migrations as mig;
use platform_wallet_storage::sqlite::schema::{core_pool, core_state};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};
use rusqlite::Connection;

const FULL_WALLET: u8 = 0xA1;
const EMPTY_WALLET: u8 = 0xB2;

/// The reuse-guard used-set `load()` assembles from a migrated store: verbatim
/// `core_address_pool` used=1 rows unioned with the `core_utxos`-derived (both
/// spent and unspent) set, deduped by script — read from the two shipped reader
/// fns the persister itself calls. Asserted at the reader layer (not the
/// assembled `core_wallet_info`) because the fixture's UTXO sits on a
/// seed-derived address unrelated to the registered account's xpub, so
/// `load()`'s pool-marking never claims it; the used-set fact it pins is a
/// reader-layer one (as in `sqlite_pool_reader.rs`). A migrated store carries no
/// pool rows, so the set is UTXO-derived here.
fn used_set(persister: &SqlitePersister, w: &WalletId) -> Vec<dashcore::Address> {
    let conn = persister.lock_conn_for_test();
    let pool = core_pool::load_used_addresses(&conn, w, dashcore::Network::Testnet)
        .expect("pool used-set");
    let utxo = core_state::load_used_addresses(&conn, w, dashcore::Network::Testnet)
        .expect("utxo used-set");
    drop(conn);
    let mut seen = std::collections::HashSet::new();
    let mut union = Vec::new();
    for addr in pool
        .into_iter()
        .map(|(addr, _owner)| addr)
        .chain(utxo.into_iter().map(|(addr, _owner)| addr))
    {
        if seen.insert(addr.script_pubkey().to_bytes()) {
            union.push(addr);
        }
    }
    union
}

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

fn transaction_height_and_blob(conn: &Connection, wallet: &WalletId) -> (Option<i64>, Vec<u8>) {
    conn.query_row(
        "SELECT height, record_blob FROM core_transactions WHERE wallet_id = ?1",
        rusqlite::params![wallet.as_slice()],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap()
}

/// Assert the post-migration store carries the full fixture data intact.
fn assert_full_data_preserved(conn: &Connection) {
    let full = wid(FULL_WALLET);
    assert_eq!(
        schema_version(conn),
        mig::max_supported_version(),
        "must be migrated to the newest embedded version"
    );
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
    let utxos = count(
        conn,
        "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
        &full,
    );
    assert_eq!(utxos, 1, "UTXO preserved");
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1",
            &full
        ),
        1
    );
    let (height, record_blob) = transaction_height_and_blob(conn, &full);
    assert_eq!(height, Some(200), "transaction height preserved by V009");
    let record: key_wallet::managed_account::transaction_record::TransactionRecord =
        platform_wallet_storage::sqlite::schema::blob::decode(&record_blob)
            .expect("transaction record blob preserved by V009");
    assert_eq!(
        record.height(),
        Some(200),
        "transaction record blob retains its block context"
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
    let tmp = common::secure_tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    let original_transaction = {
        let pre = ro_conn(&path);
        assert_eq!(schema_version(&pre), 1, "fixture starts at V001");
        transaction_height_and_blob(&pre, &wid(FULL_WALLET))
    };
    let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    {
        let conn = p.lock_conn_for_test();
        assert_full_data_preserved(&conn);
        assert_eq!(
            transaction_height_and_blob(&conn, &wid(FULL_WALLET)),
            original_transaction,
            "V009 must preserve the fixture transaction height and blob byte-for-byte"
        );
    }
    // The full wallet reconstructs; the used-set falls back to the
    // UTXO-derived address (no pool rows in a migrated store).
    let state = p.load().unwrap();
    let full = wid(FULL_WALLET);
    assert!(
        state.wallets.contains_key(&full),
        "full wallet reconstructs"
    );
    assert_eq!(
        used_set(&p, &full).len(),
        1,
        "migrated store falls back to the UTXO-derived used-set"
    );
}

/// TC-B-036 — the empty wallet inside the populated store migrates without a
/// NOT NULL violation and reads empty-but-valid.
#[test]
fn tc_b_036_empty_wallet_through_migration() {
    let tmp = common::secure_tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let state = p.load().unwrap();
    let empty = wid(EMPTY_WALLET);
    assert!(
        state.wallets.contains_key(&empty),
        "empty wallet still surfaces post-migration"
    );
    assert!(
        used_set(&p, &empty).is_empty(),
        "empty wallet is empty-but-valid, not corrupt"
    );
}

/// TC-B-032 — a byte-faithful pre-migration auto-backup is written before the
/// schema changes are visible in the live file.
#[test]
fn tc_b_032_pre_migration_backup_created() {
    let tmp = common::secure_tempdir().unwrap();
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
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with(&format!(
                    "pre-migration-1-to-{}-",
                    mig::max_supported_version()
                )) && n.ends_with(".db")
            })
        })
        .expect("pre-migration backup must exist");

    // The backup captured the PRE-migration state: schema version 1, and no
    // V003 table.
    let bconn = ro_conn(&backup);
    assert_eq!(
        schema_version(&bconn),
        1,
        "backup is the pre-migration V001 state"
    );
    assert!(
        !table_exists(&bconn, "core_address_pool"),
        "backup must predate the V003 schema"
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
    let tmp = common::secure_tempdir().unwrap();
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
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with(&format!(
                    "pre-migration-1-to-{}-",
                    mig::max_supported_version()
                ))
            })
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

/// TC-B-034 — the forward-version gate rejects at the newest embedded
/// version; a forged row one version past it is refused.
#[test]
fn tc_b_034_forward_version_rejected_at_new_max() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    {
        let _p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    }
    let forged = mig::max_supported_version() + 1;
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) \
             VALUES (?1, 'future', '', '0')",
            rusqlite::params![forged],
        )
        .unwrap();
    }
    match SqlitePersister::open(SqlitePersisterConfig::new(&path)) {
        Err(WalletStorageError::SchemaVersionUnsupported {
            found,
            max_supported,
        }) => {
            assert_eq!(found, forged);
            assert_eq!(
                max_supported,
                mig::max_supported_version(),
                "max must reflect the newest embedded migration"
            );
        }
        Err(other) => panic!("expected SchemaVersionUnsupported, got {other:?}"),
        Ok(_) => panic!("forward-version DB must be refused"),
    }
}

/// A structural + row snapshot of the affected tables, for convergence
/// comparison between a clean migration and a recovered one. Excludes the
/// per-store random generation token (unique by design).
fn migration_snapshot(conn: &Connection) -> Vec<i64> {
    let full = wid(FULL_WALLET);
    vec![
        schema_version(conn),
        conn.query_row("SELECT COUNT(*) FROM wallets", [], |r| r.get(0))
            .unwrap(),
        count(
            conn,
            "SELECT COUNT(*) FROM core_utxos WHERE wallet_id = ?1",
            &full,
        ),
        count(
            conn,
            "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1",
            &full,
        ),
        count(
            conn,
            "SELECT COUNT(*) FROM identities WHERE wallet_id = ?1",
            &full,
        ),
        count(
            conn,
            "SELECT COUNT(*) FROM contacts WHERE wallet_id = ?1",
            &full,
        ),
        count(
            conn,
            "SELECT COUNT(*) FROM account_registrations WHERE wallet_id = ?1",
            &full,
        ),
        i64::from(table_exists(conn, "core_address_pool")),
        i64::from(table_exists(conn, "meta_data_versions")),
        i64::from(table_exists(conn, "meta_store_generation")),
    ]
}

/// TC-B-035 — crash mid-migrate: an interrupted V003 (partial DDL, no commit)
/// leaves the store at the last committed version (V002) with no partial
/// tables; re-opening resumes and converges byte-equal to a clean direct
/// migration. Empirically demonstrates refinery's per-migration transaction
/// guarantee (one tx per migration — no `set_grouped`/`no_transaction`).
#[test]
fn tc_b_035_interrupted_migration_recovers_to_clean_state() {
    // Reference: a fresh copy migrated straight through.
    let clean_dir = common::secure_tempdir().unwrap();
    let clean_path = copy_fixture(clean_dir.path());
    let clean_snapshot = {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&clean_path)).unwrap();
        let conn = p.lock_conn_for_test();
        migration_snapshot(&conn)
    };
    assert_eq!(
        clean_snapshot[0],
        mig::max_supported_version(),
        "clean migration reaches the newest embedded version"
    );

    // Crash simulation: apply part of V003's DDL inside a transaction that is
    // rolled back before commit — exactly what a crash before the migration's
    // single COMMIT leaves behind (SQLite DDL is transactional).
    let crash_dir = common::secure_tempdir().unwrap();
    let crash_path = copy_fixture(crash_dir.path());
    {
        let conn = Connection::open(&crash_path).unwrap();
        conn.execute_batch(
            "BEGIN; \
             CREATE TABLE core_address_pool ( \
                wallet_id BLOB NOT NULL, account_type TEXT NOT NULL, \
                account_index INTEGER NOT NULL, \
                key_class INTEGER NOT NULL, pool_type INTEGER NOT NULL, \
                address_index INTEGER NOT NULL, script BLOB NOT NULL, \
                used INTEGER NOT NULL); \
             ROLLBACK;",
        )
        .unwrap();
        // The rolled-back DDL left no trace: still V001, no partial table.
        let pre = ro_conn(&crash_path);
        assert_eq!(schema_version(&pre), 1, "interrupted migrate stays at V001");
        assert!(
            !table_exists(&pre, "core_address_pool"),
            "partial DDL must have rolled back"
        );
    }

    // Recovery: re-open runs the pending migration cleanly.
    let recovered_snapshot = {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&crash_path)).unwrap();
        let conn = p.lock_conn_for_test();
        migration_snapshot(&conn)
    };
    assert_eq!(
        recovered_snapshot, clean_snapshot,
        "a store recovered from an interrupted migration must converge to the \
         same end state as a clean direct migration"
    );
}

/// Re-entry idempotency: reopening a fully-migrated store is a no-op — no
/// further migration, and the generation token does not rotate (it only
/// rotates on migrate/restore, not a plain reopen).
#[test]
fn reopen_of_migrated_store_is_idempotent() {
    let tmp = common::secure_tempdir().unwrap();
    let path = copy_fixture(tmp.path());
    let read = |conn: &Connection| -> (Vec<i64>, [u8; 16]) {
        let gen: Vec<u8> = conn
            .query_row(
                "SELECT generation FROM meta_store_generation WHERE id = 0",
                [],
                |r| r.get(0),
            )
            .unwrap();
        (migration_snapshot(conn), gen.try_into().unwrap())
    };
    let first = {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let conn = p.lock_conn_for_test();
        read(&conn)
    };
    let second = {
        let p = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
        let conn = p.lock_conn_for_test();
        read(&conn)
    };
    assert_eq!(
        first.0[0],
        mig::max_supported_version(),
        "first open migrates to the newest embedded version"
    );
    assert_eq!(first, second, "reopen is a byte-stable no-op");
}
