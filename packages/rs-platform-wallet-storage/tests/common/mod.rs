#![allow(clippy::field_reassign_with_default)]

//! Shared test helpers for the SQLite persister integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::Connection;

pub use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};

/// Open an empty temp directory + persister for one test. Returns the
/// persister, the keep-alive `tempfile::TempDir`, and the DB path.
pub fn fresh_persister() -> (SqlitePersister, tempfile::TempDir, PathBuf) {
    fresh_persister_with_mode(FlushMode::Immediate)
}

pub fn fresh_persister_with_mode(mode: FlushMode) -> (SqlitePersister, tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let cfg = SqlitePersisterConfig::new(&path).with_flush_mode(mode);
    let p = SqlitePersister::open(cfg).expect("open persister");
    (p, tmp, path)
}

/// Wallet id helper.
pub fn wid(byte: u8) -> WalletId {
    [byte; 32]
}

/// Open a read-only side connection — used by tests that probe the DB
/// while the persister still owns the write conn.
pub fn ro_conn(path: &std::path::Path) -> Connection {
    Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .expect("open ro conn")
}

/// Insert a stub `wallet_metadata` row so child writes pass the native
/// FK. Bypasses the buffer/flush layer — tests use this when they
/// want to exercise a single sub-changeset writer in isolation.
pub fn ensure_wallet_meta(persister: &SqlitePersister, wallet_id: &WalletId) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .expect("ensure wallet_metadata");
}

/// Insert a stub `identities` row so identity-owned table writes
/// (`token_balances`, `dashpay_profiles`, `identity_keys`) pass the
/// FK to `identities(identity_id)`. `parent_wallet_id` is
/// optional — when `Some`, the row is linked to that wallet so the
/// cascade chain works; when `None`, the row is an orphan identity
/// (NULL `wallet_id`), still satisfying the identity-owned FKs.
pub fn ensure_identity(
    persister: &SqlitePersister,
    identity_id: &[u8; 32],
    parent_wallet_id: Option<&WalletId>,
) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    let wid_param: Option<&[u8]> = parent_wallet_id.map(|w| w.as_slice());
    conn.execute(
        "INSERT OR IGNORE INTO identities \
            (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, NULL, X'00', 0)",
        params![&identity_id[..], wid_param],
    )
    .expect("ensure identity");
}

/// Echo a simple `store` + `flush` of an arbitrary changeset.
pub fn store_and_flush(
    persister: &SqlitePersister,
    wallet_id: WalletId,
    cs: platform_wallet::changeset::PlatformWalletChangeSet,
) {
    persister.store(wallet_id, cs).expect("store");
    persister.flush(wallet_id).expect("flush");
}
