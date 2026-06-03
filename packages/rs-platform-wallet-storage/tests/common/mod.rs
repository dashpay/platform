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

/// Insert a stub `token_balances` row so `meta_token` writes pass the
/// composite FK to `token_balances(identity_id, token_id)`. The parent
/// `identities` row must already exist (seed via [`ensure_identity`]).
pub fn ensure_token_balance(
    persister: &SqlitePersister,
    identity_id: &[u8; 32],
    token_id: &[u8; 32],
) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO token_balances \
            (identity_id, token_id, balance, updated_at) \
         VALUES (?1, ?2, 0, 0)",
        params![&identity_id[..], &token_id[..]],
    )
    .expect("ensure token_balance");
}

/// Insert a stub `established` row in the unified `contacts` table so
/// the `cascade_meta_contact_on_contact_delete` trigger has an
/// established-contact parent to fire on for `meta_contact` writes keyed
/// by `(wallet_id, owner_id, contact_id)`. The parent `wallet_metadata`
/// row must already exist (seed via [`ensure_wallet_meta`]).
pub fn ensure_contact_established(
    persister: &SqlitePersister,
    wallet_id: &WalletId,
    owner_id: &[u8; 32],
    contact_id: &[u8; 32],
) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO contacts \
            (wallet_id, owner_id, contact_id, state) \
         VALUES (?1, ?2, ?3, 'established')",
        params![wallet_id.as_slice(), &owner_id[..], &contact_id[..]],
    )
    .expect("ensure contact_established");
}

/// Insert a stub `sent` contact row (pending outgoing request) so a
/// `meta_contact` write keyed by `(wallet_id, owner_id, contact_id)` has
/// a non-established parent to exercise. The parent `wallet_metadata`
/// row must already exist.
pub fn ensure_contact_sent(
    persister: &SqlitePersister,
    wallet_id: &WalletId,
    owner_id: &[u8; 32],
    contact_id: &[u8; 32],
) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO contacts \
            (wallet_id, owner_id, contact_id, state) \
         VALUES (?1, ?2, ?3, 'sent')",
        params![wallet_id.as_slice(), &owner_id[..], &contact_id[..]],
    )
    .expect("ensure contact_sent");
}

/// Insert a stub `received` contact row (pending incoming request),
/// symmetric to [`ensure_contact_sent`].
pub fn ensure_contact_received(
    persister: &SqlitePersister,
    wallet_id: &WalletId,
    owner_id: &[u8; 32],
    contact_id: &[u8; 32],
) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO contacts \
            (wallet_id, owner_id, contact_id, state) \
         VALUES (?1, ?2, ?3, 'received')",
        params![wallet_id.as_slice(), &owner_id[..], &contact_id[..]],
    )
    .expect("ensure contact_received");
}

/// Insert a stub `platform_addresses` row so `meta_platform_address`
/// writes pass the composite FK to
/// `platform_addresses(wallet_id, address)`. The parent
/// `wallet_metadata` row must already exist (seed via
/// [`ensure_wallet_meta`]). `address` is an opaque BLOB.
pub fn ensure_platform_address(persister: &SqlitePersister, wallet_id: &WalletId, address: &[u8]) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO platform_addresses \
            (wallet_id, account_index, address_index, address, balance, nonce) \
         VALUES (?1, 0, 0, ?2, 0, 0)",
        params![wallet_id.as_slice(), address],
    )
    .expect("ensure platform_address");
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
