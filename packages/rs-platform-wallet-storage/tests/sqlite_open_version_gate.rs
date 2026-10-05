//! `SqlitePersister::open` must refuse a database whose
//! `refinery_schema_history` MAX(version) exceeds the embedded max.
//! Symmetric with `restore_from`: a forged forward-version row that
//! older binary would otherwise migration::run() no-op past gets
//! caught at open time.

mod common;

use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

#[test]
fn open_rejects_forward_schema_version() {
    let tmp = common::secure_tempdir().unwrap();
    let db_path = tmp.path().join("wallet.db");

    // First open to run the embedded migrations.
    {
        let _p = SqlitePersister::open(SqlitePersisterConfig::new(&db_path)).expect("open");
    }
    // Forge a row claiming a future migration was already applied.
    {
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) \
             VALUES (?1, 'future', '', '0')",
            rusqlite::params![1_000_000i64],
        )
        .unwrap();
    }
    // Re-open must fail with SchemaVersionUnsupported.
    match SqlitePersister::open(SqlitePersisterConfig::new(&db_path)) {
        Err(WalletStorageError::SchemaVersionUnsupported { .. }) => {}
        Err(other) => panic!("expected SchemaVersionUnsupported, got {other:?}"),
        Ok(_) => panic!("forward-version DB must be refused"),
    }
}
