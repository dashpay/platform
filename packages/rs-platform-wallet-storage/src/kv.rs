//! Generic key/value store API.
//!
//! Backend-neutral surface for stashing arbitrary application-managed
//! data alongside wallet state. Today the only shipped implementation
//! is on [`crate::sqlite::SqlitePersister`]; the trait is defined here,
//! at the top level, so a future backend can implement it without
//! reaching into the SQLite submodule.
//!
//! Values are opaque `Vec<u8>` blobs — the app picks its own
//! serialization (bincode, JSON, protobuf, raw bytes). Keys are
//! bounded `TEXT` (1..=128 chars).
//!
//! Scoping: `wallet_id = None` is a global slot that survives wallet
//! deletion; `wallet_id = Some(id)` is scoped to a single wallet and
//! cascades on wallet delete. The same key string under `None` and
//! under `Some(id)` are independent — they live in different
//! partitions of the underlying index.
//!
//! This API is **independent of [`platform_wallet::changeset::PlatformWalletPersistence`]**:
//! KV is for app data, not wallet domain state. Reads and writes go
//! straight to the underlying store without flowing through the wallet
//! changeset buffer or transaction.

use platform_wallet::wallet::platform_wallet::WalletId;

/// Maximum allowed key length, in bytes/chars (ASCII assumed for the
/// fast path; SQLite's `length()` counts UTF-8 chars for TEXT, so
/// non-ASCII keys are also capped at 128 code points). Enforced in
/// Rust (typed-error pre-check) AND in the SQL schema (`CHECK
/// (length(key) BETWEEN 1 AND 128)`).
pub const MAX_KEY_LEN: usize = 128;

/// Hard cap on the size of a single KV value, in bytes. Mirrors the
/// `BLOB_SIZE_LIMIT_BYTES` ceiling on bincode-serde blobs in
/// `sqlite::schema::blob` so a tampered or corrupted backup row cannot
/// force a multi-gigabyte allocation on the next `get`. CMT-006.
pub const MAX_VALUE_LEN: usize = 16 * 1024 * 1024;

/// Errors returned by [`KvStore`] operations.
///
/// `Sqlite` is the only backend-specific variant today; new backends
/// add their own variant rather than reusing it.
#[derive(Debug, thiserror::Error)]
pub enum KvError {
    /// Key was empty (`""`). Keys must be 1..=[`MAX_KEY_LEN`] chars.
    #[error("kv key is empty")]
    KeyEmpty,

    /// Key exceeded [`MAX_KEY_LEN`].
    #[error("kv key too long: {len} bytes (max {})", MAX_KEY_LEN)]
    KeyTooLong { len: usize },

    /// Stored value exceeded [`MAX_VALUE_LEN`] on read. Surfaced before
    /// the bytes are materialised so a tampered row cannot OOM the
    /// process. CMT-006.
    #[error("kv value too large: {found} bytes (max {max})")]
    ValueTooLarge { found: usize, max: usize },

    /// Per-wallet `put` referenced a wallet that has no
    /// `wallet_metadata` row. Surfaced as a typed variant instead of a
    /// raw foreign-key violation so callers can branch on it.
    #[error("wallet not found for kv put: {}", hex::encode(wallet_id))]
    WalletNotFound { wallet_id: [u8; 32] },

    /// Backend-specific SQLite failure.
    #[error("sqlite error")]
    Sqlite(#[from] rusqlite::Error),

    /// A previous holder of the persister's connection mutex panicked.
    /// Mirrors [`crate::sqlite::error::WalletStorageError::LockPoisoned`].
    #[error("persister lock poisoned")]
    LockPoisoned,
}

/// Generic key/value store for arbitrary application-managed data.
///
/// See the module-level docs for scoping and value semantics.
pub trait KvStore {
    /// Read the value bound to `(wallet_id, key)`. Returns `Ok(None)`
    /// when the key is absent. Backends MUST reject values larger than
    /// [`MAX_VALUE_LEN`] with [`KvError::ValueTooLarge`] before
    /// materialising the bytes (CMT-006).
    fn get(&self, wallet_id: Option<&WalletId>, key: &str) -> Result<Option<Vec<u8>>, KvError>;

    /// Insert or overwrite the value bound to `(wallet_id, key)`.
    /// Upserts via `INSERT … ON CONFLICT(…) DO UPDATE` — repeat puts
    /// of the same key replace the previous value.
    fn put(&self, wallet_id: Option<&WalletId>, key: &str, value: &[u8]) -> Result<(), KvError>;

    /// Remove the row bound to `(wallet_id, key)`. Idempotent — a
    /// missing key returns `Ok(())` rather than an error (mirrors the
    /// `SecretStore::delete` convention).
    fn delete(&self, wallet_id: Option<&WalletId>, key: &str) -> Result<(), KvError>;

    /// List keys in the given scope. `prefix = None` returns every
    /// key in the scope; `prefix = Some(p)` returns only keys that
    /// start with `p`. The implementation escapes `%`, `_`, and `\`
    /// inside `p` so the prefix is treated as a literal — pattern
    /// metacharacters in app keys are never interpreted as wildcards.
    /// Order is ascending by key.
    fn list_keys(
        &self,
        wallet_id: Option<&WalletId>,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, KvError>;
}

/// Validate a key against the length bounds. Used by [`KvStore`]
/// implementations as a typed-error pre-check before reaching SQL.
pub(crate) fn validate_key(key: &str) -> Result<(), KvError> {
    if key.is_empty() {
        return Err(KvError::KeyEmpty);
    }
    if key.len() > MAX_KEY_LEN {
        return Err(KvError::KeyTooLong { len: key.len() });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty() {
        assert!(matches!(validate_key(""), Err(KvError::KeyEmpty)));
    }

    #[test]
    fn validate_rejects_too_long() {
        let k = "a".repeat(MAX_KEY_LEN + 1);
        match validate_key(&k) {
            Err(KvError::KeyTooLong { len }) => assert_eq!(len, MAX_KEY_LEN + 1),
            other => panic!("expected KeyTooLong, got {other:?}"),
        }
    }

    #[test]
    fn validate_accepts_boundary_lengths() {
        assert!(validate_key("k").is_ok());
        let k = "a".repeat(MAX_KEY_LEN);
        assert!(validate_key(&k).is_ok());
    }
}
