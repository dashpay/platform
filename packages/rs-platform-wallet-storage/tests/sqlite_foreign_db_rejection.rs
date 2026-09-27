#![allow(clippy::field_reassign_with_default)]

//! `open()` must reject a pre-existing NON-wallet SQLite file (schema objects
//! but no `refinery_schema_history`) instead of silently grafting wallet
//! tables onto a foreign schema.

mod common;

use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig, WalletStorageError};

#[test]
fn open_rejects_foreign_sqlite_without_refinery_history() {
    let tmp = common::secure_tempdir().unwrap();
    let path = tmp.path().join("foreign.db");

    // A plain SQLite DB with a user table but no refinery history and no
    // wallet application_id.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("CREATE TABLE not_ours (id INTEGER PRIMARY KEY)", [])
            .unwrap();
    }

    // `SqlitePersister` isn't `Debug`, so take `.err()` rather than
    // `.expect_err()` (which would need the Ok type to be `Debug`).
    let err = SqlitePersister::open(SqlitePersisterConfig::new(&path)).err();
    assert!(
        matches!(err, Some(WalletStorageError::NotAWalletDb { .. })),
        "a foreign sqlite db must be rejected as NotAWalletDb, got {err:?}"
    );
}
