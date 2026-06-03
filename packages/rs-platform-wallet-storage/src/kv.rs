//! Per-object-type key/value metadata API.
//!
//! Backend-neutral surface for stashing arbitrary application-managed
//! metadata alongside wallet objects. Today the only shipped
//! implementation is on [`crate::sqlite::SqlitePersister`]; the trait is
//! defined here, at the top level, so a future backend can implement it
//! without reaching into the SQLite submodule.
//!
//! Values are opaque `Vec<u8>` blobs — the app picks its own
//! serialization (bincode, JSON, protobuf, raw bytes). Keys are
//! bounded `TEXT` (1..=128 chars).
//!
//! Scoping: each [`ObjectId`] variant addresses a dedicated table. The
//! [`ObjectId::Global`] slot has no parent and survives wallet deletion.
//! Every other variant names a wallet object, but a write does NOT
//! require that object to exist yet — metadata may be attached ahead of
//! sync. When the object is later deleted, an `AFTER DELETE` trigger on
//! its parent table removes the matching metadata. However, if the
//! parent object is never created, or is removed via a path the trigger
//! does not cover, the metadata row may persist as an orphan. This is an
//! accepted limitation across all scopes; a future garbage-collection pass
//! is expected to reap such orphans (no live parent, e.g. older than ~1
//! week) — callers should not rely on orphan metadata persisting forever.
//! The same key string under different scopes is independent — the scopes
//! live in separate tables.
//!
//! This API is **independent of [`platform_wallet::changeset::PlatformWalletPersistence`]**:
//! KV is for app metadata, not wallet domain state. Reads and writes go
//! straight to the underlying store without flowing through the wallet
//! changeset buffer or transaction.

use platform_wallet::wallet::platform_wallet::WalletId;

/// Scope of a metadata entry — one variant per dedicated `meta_*` table.
///
/// [`ObjectId::Global`] has no parent and survives wallet deletion. The
/// other variants name a wallet object but carry no insert-time
/// existence requirement: metadata may be written before its parent
/// object is synced into its typed table. An `AFTER DELETE` trigger on
/// each parent removes the matching metadata when the object is deleted.
///
/// **Orphan metadata:** if the parent object is never created, or is
/// removed via a path the trigger does not cover, the metadata row may
/// persist as an orphan. A future GC pass is expected to reap such
/// rows; do not rely on them living forever.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectId {
    /// Global app metadata; no parent (`meta_global`).
    Global,
    /// Per-wallet metadata (`meta_wallet`).
    Wallet(WalletId),
    /// Per-identity metadata (`meta_identity`).
    Identity([u8; 32]),
    /// Per-token-balance metadata (`meta_token`).
    Token {
        identity_id: [u8; 32],
        token_id: [u8; 32],
    },
    /// Per-contact metadata (`meta_contact`) — for a contact in any lifecycle state.
    Contact {
        wallet_id: WalletId,
        owner_id: [u8; 32],
        contact_id: [u8; 32],
    },
    /// Per-platform-address metadata (`meta_platform_address`).
    PlatformAddress {
        wallet_id: WalletId,
        address: Vec<u8>,
    },
}

/// Maximum allowed key length. Enforced in Rust as a **byte**-length
/// bound (`validate_key` rejects with `KeyTooLong`/`KeyEmpty` on
/// `key.len()`) and in SQL as a **code-point** bound
/// (`CHECK (length(key) BETWEEN 1 AND 128)`, where SQLite's `length()`
/// counts UTF-8 code points). For ASCII keys the two coincide; for
/// non-ASCII keys the Rust byte bound is the stricter of the two, so no
/// over-length key reaches SQL.
pub const MAX_KEY_LEN: usize = 128;

/// Hard cap on the size of a single KV value, in bytes, so a tampered or
/// corrupted backup row cannot force a multi-gigabyte allocation on the
/// next `get`.
///
/// Kept in sync MANUALLY with the `BLOB_SIZE_LIMIT_BYTES` ceiling on
/// bincode-serde blobs in `sqlite::schema::blob`: the `sqlite` and `kv`
/// features compile independently, so a `const`-level cross-reference
/// between the two modules can't be relied on. Change both together if
/// the ceiling moves.
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

    /// A value exceeded [`MAX_VALUE_LEN`]. Raised by `put` before the
    /// INSERT and by `get` before the bytes are materialised, so an
    /// oversize value never lands and a tampered row never OOMs the
    /// process.
    #[error("kv value too large: {found} bytes (max {max})")]
    ValueTooLarge { found: usize, max: usize },

    /// Backend-specific SQLite failure.
    #[error("sqlite error")]
    Sqlite(#[from] rusqlite::Error),

    /// A previous holder of the persister's connection mutex panicked.
    /// Mirrors [`crate::sqlite::error::WalletStorageError::LockPoisoned`].
    #[error("persister lock poisoned")]
    LockPoisoned,
}

/// Per-object-type key/value metadata store.
///
/// See the module-level docs for scoping and value semantics. Each
/// [`ObjectId`] variant addresses a dedicated table.
pub trait KvStore {
    /// Read the value bound to `(scope, key)`. Returns `Ok(None)` when
    /// the key is absent. Backends MUST reject values larger than
    /// [`MAX_VALUE_LEN`] with [`KvError::ValueTooLarge`] before
    /// materialising the bytes.
    fn get(&self, scope: &ObjectId, key: &str) -> Result<Option<Vec<u8>>, KvError>;

    /// Insert or overwrite the value bound to `(scope, key)`. Upserts
    /// via `INSERT … ON CONFLICT(…) DO UPDATE` — repeat puts of the same
    /// key replace the previous value. Succeeds even when the scope's
    /// parent object does not exist yet; an `AFTER DELETE` trigger cleans
    /// the metadata up if that object is later removed.
    ///
    /// Backends MUST reject a `value` larger than [`MAX_VALUE_LEN`] with
    /// [`KvError::ValueTooLarge`] before writing, so a `put` can never
    /// plant a row a later `get` would refuse to materialise.
    fn put(&self, scope: &ObjectId, key: &str, value: &[u8]) -> Result<(), KvError>;

    /// Remove the row bound to `(scope, key)`. Idempotent — a missing
    /// key returns `Ok(())` rather than an error (mirrors the
    /// `SecretStore::delete` convention).
    fn delete(&self, scope: &ObjectId, key: &str) -> Result<(), KvError>;

    /// List keys in the given scope. `prefix = None` returns every
    /// key in the scope; `prefix = Some(p)` returns only keys that
    /// start with `p`. The implementation escapes `%`, `_`, and `\`
    /// inside `p` so the prefix is treated as a literal — pattern
    /// metacharacters in app keys are never interpreted as wildcards.
    /// Order is ascending by key.
    fn list_keys(&self, scope: &ObjectId, prefix: Option<&str>) -> Result<Vec<String>, KvError>;
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
