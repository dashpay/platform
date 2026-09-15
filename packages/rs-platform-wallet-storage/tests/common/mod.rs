#![allow(clippy::field_reassign_with_default)]

//! Shared test helpers for the SQLite persister integration tests.

#![allow(dead_code)]

use std::path::PathBuf;

use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::Connection;

pub use platform_wallet_storage::{FlushMode, LoadPolicy, SqlitePersister, SqlitePersisterConfig};

/// Open an empty temp directory + persister for one test. Returns the
/// persister, the keep-alive `tempfile::TempDir`, and the DB path.
pub fn fresh_persister() -> (SqlitePersister, tempfile::TempDir, PathBuf) {
    fresh_persister_with_mode(FlushMode::Immediate)
}

pub fn fresh_persister_with_mode(mode: FlushMode) -> (SqlitePersister, tempfile::TempDir, PathBuf) {
    let tmp = secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let cfg = SqlitePersisterConfig::new(&path).with_flush_mode(mode);
    let p = SqlitePersister::open(cfg).expect("open persister");
    (p, tmp, path)
}

/// Seed a database through a strict persister, then reopen it in
/// [`LoadPolicy::Recovery`].
///
/// The strict handle is dropped before the reopen: the process-wide
/// open-path registry refuses a second live persister on one path.
pub fn fresh_recovery_persister(
    seed: impl FnOnce(&SqlitePersister),
) -> (SqlitePersister, tempfile::TempDir, PathBuf) {
    let tmp = secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let strict = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("open strict");
    seed(&strict);
    drop(strict);
    let cfg = SqlitePersisterConfig::new(&path).with_load_policy(LoadPolicy::Recovery);
    let p = SqlitePersister::open(cfg).expect("open recovery");
    (p, tmp, path)
}

/// Create a test directory that satisfies the persister's Unix parent policy.
pub fn secure_tempdir() -> std::io::Result<tempfile::TempDir> {
    let tmp = tempfile::tempdir()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(tmp)
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

/// Insert a stub `wallets` row so child writes pass the native
/// FK. Bypasses the buffer/flush layer — tests use this when they
/// want to exercise a single sub-changeset writer in isolation.
pub fn ensure_wallet_meta(persister: &SqlitePersister, wallet_id: &WalletId) {
    use rusqlite::params;
    let conn = persister.lock_conn_for_test();
    conn.execute(
        "INSERT OR IGNORE INTO wallets (wallet_id, network, birth_height) \
         VALUES (?1, 'testnet', 0)",
        params![wallet_id.as_slice()],
    )
    .expect("ensure wallets");
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
    let conn = persister.lock_conn_for_test();
    // Delegate to the production stub writer so `entry_blob` holds a
    // real, decodable `IdentityEntry` (the wired `load()` decodes every
    // identity row). The all-zero sentinel WalletId maps to a NULL
    // `wallet_id` column, so `None` lands as an orphan identity.
    let scope: WalletId = parent_wallet_id.copied().unwrap_or([0u8; 32]);
    platform_wallet_storage::sqlite::schema::identities::ensure_exists(&conn, &scope, identity_id)
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
/// by `(wallet_id, owner_id, contact_id)`. The parent `wallets`
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
/// a non-established parent to exercise. The parent `wallets`
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
/// `wallets` row must already exist (seed via
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

/// Run `action` on another thread, released exactly at the seam
/// between a `store()`'s buffer merge and its flush, and block that
/// `store()` for up to `budget` waiting for `action` to return.
///
/// This is how the crate tests what a second actor can and cannot do
/// inside that window without racing for it. An `Immediate` `store()`
/// holds the write connection across the whole seam, so an `action`
/// that needs it is parked until the `store()` returns and the wait
/// simply expires; waiting is not the assertion, it only guarantees the
/// action had its chance, so no outcome rides on thread scheduling.
///
/// The seam is ONE-SHOT: `action` may itself call `store`, and that
/// call must not be released back into this same rendezvous.
pub fn release_at_store_seam<T, F>(
    persister: &std::sync::Arc<SqlitePersister>,
    budget: std::time::Duration,
    action: F,
) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};

    let (go_tx, go_rx) = mpsc::channel::<()>();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let handle = std::thread::spawn(move || {
        go_rx.recv().expect("the seam released the action");
        let out = action();
        let _ = done_tx.send(());
        out
    });
    let done_rx = Mutex::new(done_rx);
    let released = AtomicBool::new(false);
    persister.set_store_flush_seam_for_test(Arc::new(move || {
        if released.swap(true, Ordering::SeqCst) {
            return;
        }
        go_tx.send(()).expect("the action thread is listening");
        let _ = done_rx.lock().expect("seam channel").recv_timeout(budget);
    }));
    handle
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
