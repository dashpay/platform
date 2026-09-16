#![allow(clippy::field_reassign_with_default)]

//! Wallet-DB identity gates: the `application_id` header magic and the
//! `refinery_schema_history` well-formedness probe.
//!
//! - A foreign refinery-versioned SQLite DB (has `schema_history`, passes
//!   `integrity_check`, version within range) but the WRONG
//!   `application_id` must be rejected as `NotAWalletDb` — both on
//!   `restore_from` (destination untouched) and on `open()`.
//! - A wallet DB whose `refinery_schema_history` carries a malformed
//!   `applied_on` / `checksum` must surface a typed
//!   `SchemaHistoryMalformed` rather than panicking inside refinery.

mod common;

use common::{fresh_persister, wid};
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};
use rusqlite::Connection;

/// `SqlitePersister` is not `Debug`, so `Result::expect_err` can't be
/// used on an `open()` result — extract the error by matching instead.
fn open_err(cfg: SqlitePersisterConfig) -> WalletStorageError {
    match SqlitePersister::open(cfg) {
        Ok(_) => panic!("expected open() to fail"),
        Err(e) => e,
    }
}

/// Build a "foreign" refinery-versioned DB at `path`: it has a
/// `refinery_schema_history` table with a well-formed row and passes
/// `integrity_check`, but carries a DIFFERENT `application_id`, so it is
/// NOT a wallet-storage database.
fn write_foreign_refinery_db(path: &std::path::Path, application_id: i32) {
    let conn = Connection::open(path).expect("open foreign db");
    conn.pragma_update(None, "application_id", application_id)
        .expect("stamp foreign application_id");
    conn.execute_batch(
        "CREATE TABLE refinery_schema_history (
             version INTEGER PRIMARY KEY,
             name TEXT,
             applied_on TEXT,
             checksum TEXT
         );
         INSERT INTO refinery_schema_history (version, name, applied_on, checksum)
             VALUES (1, 'initial', '2026-01-01T00:00:00+00:00', '12345');
         CREATE TABLE some_foreign_table (x INTEGER);",
    )
    .expect("seed foreign schema");
    drop(conn);
}

/// Materialize a real wallet DB on disk, then return its path inside a
/// kept-alive tempdir.
fn fresh_wallet_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let (persister, tmp, path) = fresh_persister();
    let w = wid(0x11);
    let mut cs = PlatformWalletChangeSet::default();
    cs.wallet_metadata = Some(WalletMetadataEntry {
        network: key_wallet::Network::Testnet,
        wallet_group_id: [0u8; 32],
        birth_height: 0,
    });
    cs.core = Some(CoreChangeSet {
        synced_height: Some(5),
        last_processed_height: Some(5),
        ..Default::default()
    });
    persister.store(w, cs).expect("store");
    persister.flush(w).expect("flush");
    drop(persister);
    (tmp, path)
}

#[test]
fn restore_from_rejects_foreign_application_id_destination_untouched() {
    let (_tmp, dest) = fresh_wallet_db();

    // Snapshot the live destination bytes so we can prove restore left it
    // untouched on rejection.
    let before = std::fs::read(&dest).expect("read dest before");

    let src_tmp = common::secure_tempdir().unwrap();
    let foreign = src_tmp.path().join("foreign.db");
    // Anything but the wallet-storage magic.
    write_foreign_refinery_db(&foreign, 0x0BAD_F00D_u32 as i32);

    let err = SqlitePersister::restore_from_skip_backup(&dest, &foreign)
        .expect_err("restore of a foreign refinery DB must fail");
    assert!(
        matches!(err, WalletStorageError::NotAWalletDb { .. }),
        "expected NotAWalletDb, got {err:?}"
    );

    let after = std::fs::read(&dest).expect("read dest after");
    assert_eq!(
        before, after,
        "destination wallet DB must be byte-identical after a rejected restore"
    );

    // The destination must still open as a wallet DB.
    SqlitePersister::open(SqlitePersisterConfig::new(&dest))
        .expect("destination still opens after rejected restore");
}

#[test]
fn open_rejects_foreign_application_id() {
    let tmp = common::secure_tempdir().unwrap();
    let foreign = tmp.path().join("foreign.db");
    write_foreign_refinery_db(&foreign, 0x0BAD_F00D_u32 as i32);

    let err = open_err(SqlitePersisterConfig::new(&foreign));
    assert!(
        matches!(err, WalletStorageError::NotAWalletDb { .. }),
        "expected NotAWalletDb, got {err:?}"
    );
}

#[test]
fn open_accepts_a_real_wallet_db_with_stamped_application_id() {
    let (_tmp, path) = fresh_wallet_db();
    // Reopening a genuine wallet DB must pass the application_id gate.
    SqlitePersister::open(SqlitePersisterConfig::new(&path))
        .expect("reopen of a genuine wallet DB must succeed");
}

#[test]
fn open_rejects_malformed_schema_history_without_panicking() {
    let (_tmp, path) = fresh_wallet_db();

    // Corrupt the schema_history `applied_on` to a non-RFC3339 value via
    // a side connection, then reopen. Refinery would unwrap()-panic on
    // this; the pre-run probe must turn it into a typed error.
    {
        let conn = Connection::open(&path).expect("side conn");
        conn.execute(
            "UPDATE refinery_schema_history SET applied_on = 'not-a-timestamp'",
            [],
        )
        .expect("corrupt applied_on");
    }

    let err = open_err(SqlitePersisterConfig::new(&path));
    assert!(
        matches!(err, WalletStorageError::SchemaHistoryMalformed { .. }),
        "expected SchemaHistoryMalformed, got {err:?}"
    );
}

#[test]
fn open_rejects_non_numeric_checksum_in_schema_history() {
    let (_tmp, path) = fresh_wallet_db();
    {
        let conn = Connection::open(&path).expect("side conn");
        conn.execute(
            "UPDATE refinery_schema_history SET checksum = 'deadbeef'",
            [],
        )
        .expect("corrupt checksum");
    }
    let err = open_err(SqlitePersisterConfig::new(&path));
    assert!(
        matches!(err, WalletStorageError::SchemaHistoryMalformed { .. }),
        "expected SchemaHistoryMalformed, got {err:?}"
    );
}
