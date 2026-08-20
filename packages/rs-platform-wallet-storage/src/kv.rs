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
//! Scoping: each [`ObjectId`] variant addresses a dedicated table, so the
//! same key string under different scopes is independent.
//! [`ObjectId::Global`] has no parent and survives wallet deletion. Other
//! variants name a wallet object but a write does NOT require it to exist
//! yet (metadata may be attached ahead of sync); an `AFTER DELETE` trigger
//! reaps the metadata when the object is deleted. Rows whose parent is
//! never created, or removed via a path the trigger misses, may persist as
//! orphans — an accepted limitation; a future GC pass is expected to reap
//! them, so callers must not rely on orphans living forever.
//!
//! This API is **independent of [`platform_wallet::changeset::PlatformWalletPersistence`]**:
//! KV is for app metadata, not wallet domain state. Reads and writes go
//! straight to the underlying store without flowing through the wallet
//! changeset buffer or transaction.

use platform_wallet::wallet::platform_wallet::WalletId;

/// Scope of a metadata entry — one variant per dedicated `meta_*` table.
///
/// [`ObjectId::Global`] has no parent and survives wallet deletion. Other
/// variants name a wallet object but may be written before it is synced;
/// an `AFTER DELETE` trigger reaps the metadata when the object is deleted.
/// See the module docs for the orphan-metadata limitation.
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

/// Maximum allowed key length, in **code points**. `validate_key` counts
/// `chars().count()`; the SQL `CHECK (length(key) BETWEEN 1 AND 128)` uses
/// the same unit (SQLite `length()` counts code points), so the two bounds
/// accept exactly the same key set.
pub const MAX_KEY_LEN: usize = 128;

/// Hard cap on the size of a single KV value, in bytes, so a tampered or
/// corrupted backup row cannot force a multi-gigabyte allocation on the
/// next `get`. Shares the crate-root [`SIZE_LIMIT_BYTES`](crate::SIZE_LIMIT_BYTES)
/// ceiling with the bincode-serde BLOB decode cap.
pub const MAX_VALUE_LEN: usize = crate::SIZE_LIMIT_BYTES;

/// Errors returned by [`KvStore`] operations.
///
/// `Sqlite` is the only backend-specific variant today; new backends
/// add their own variant rather than reusing it.
#[derive(Debug, thiserror::Error)]
pub enum KvError {
    /// Key was empty (`""`). Keys must be 1..=[`MAX_KEY_LEN`] chars.
    #[error("kv key is empty")]
    KeyEmpty,

    /// Key contained an embedded NUL (`\0`). SQLite `length()` counts only
    /// the bytes before the first NUL, so a NUL-bearing key would break the
    /// `chars().count()` == SQLite `length()` invariant the CHECK relies on.
    #[error("kv key contains an embedded NUL")]
    KeyContainsNul,

    /// Key exceeded [`MAX_KEY_LEN`]. `len` is the key's code-point count
    /// (the same unit the SQL `length()` CHECK uses).
    #[error("kv key too long: {len} code points (max {})", MAX_KEY_LEN)]
    KeyTooLong { len: usize },

    /// A value exceeded [`MAX_VALUE_LEN`]. Raised by `put` before the
    /// INSERT and by `get` before materialising, so a tampered row can't
    /// OOM the process.
    #[error("kv value too large: {found} bytes (max {max})")]
    ValueTooLarge { found: usize, max: usize },

    /// Backend-specific SQLite failure.
    #[error("sqlite error")]
    Sqlite(#[from] rusqlite::Error),

    /// A previous holder of the persister's connection mutex panicked.
    /// Mirrors [`crate::sqlite::error::WalletStorageError::LockPoisoned`].
    #[error("persister lock poisoned")]
    LockPoisoned,

    /// A `put` / `delete` was attempted while the backing store is open
    /// read-only for recovery. Reads stay available. `operation` names the
    /// blocked entry point.
    #[error(
        "`{operation}` is blocked: the backing store is open in recovery mode (read-only) — \
         repair the database, then reopen it under the strict load policy to write again"
    )]
    ReadOnlyRecoveryMode { operation: &'static str },
}

/// Per-object-type key/value metadata store.
///
/// See the module-level docs for scoping and value semantics. Each
/// [`ObjectId`] variant addresses a dedicated table.
///
/// # Security
///
/// Values are stored **PLAINTEXT** in the persister `.db` and in every
/// backup copied from it. This API is the explicit, caller-policed
/// plaintext exception to the crate's no-secrets-in-the-db boundary
/// (see `SECRETS.md`). **NEVER store key or signing material here** —
/// mnemonics, seeds, private keys, or anything that could move funds.
/// Use [`SecretStore`](crate::secrets::SecretStore) for secret material.
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
    ///
    /// # Security
    ///
    /// `value` is stored **PLAINTEXT** in the `.db` and all backups.
    /// NEVER store key/signing material here — use
    /// [`SecretStore`](crate::secrets::SecretStore).
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

/// Typed-error pre-check used by [`KvStore`] impls before reaching SQL.
/// Counts code points to match the SQL CHECK unit (see [`MAX_KEY_LEN`]).
pub(crate) fn validate_key(key: &str) -> Result<(), KvError> {
    if key.is_empty() {
        return Err(KvError::KeyEmpty);
    }
    // An embedded NUL truncates SQLite's `length()` (and string comparisons),
    // so reject it before the count below — otherwise `chars().count()` and the
    // SQL CHECK would disagree on the key's length and identity.
    if key.contains('\0') {
        return Err(KvError::KeyContainsNul);
    }
    let code_points = key.chars().count();
    if code_points > MAX_KEY_LEN {
        return Err(KvError::KeyTooLong { len: code_points });
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

    #[test]
    fn validate_rejects_embedded_nul() {
        assert!(matches!(validate_key("a\0b"), Err(KvError::KeyContainsNul)));
        // A leading/trailing NUL is rejected too.
        assert!(matches!(validate_key("\0"), Err(KvError::KeyContainsNul)));
    }
}
