//! SQLite-backed [`KvStore`] implementation for [`SqlitePersister`].
//!
//! Single physical table `kv_store` with a nullable `wallet_id`:
//! `NULL` rows live in the global slot, non-NULL rows are wallet-scoped
//! and `CASCADE` on parent delete via the FK to `wallet_metadata`.
//!
//! Uniqueness is enforced by two **partial** unique indexes:
//!
//! - `idx_kv_store_global  ON kv_store(key)             WHERE wallet_id IS NULL`
//! - `idx_kv_store_wallet  ON kv_store(wallet_id, key)  WHERE wallet_id IS NOT NULL`
//!
//! Partial indexes are needed because SQLite treats every NULL in a
//! plain `UNIQUE(wallet_id, key)` as distinct, which would allow
//! duplicate globals. Splitting into two predicates partitions the
//! rows and gives both halves the uniqueness guarantee we want.
//!
//! All operations reuse `SqlitePersister`'s single `Mutex<Connection>`
//! via the crate-private `conn()` accessor; no separate connection is
//! opened.

use rusqlite::{params, OptionalExtension};

use platform_wallet::wallet::platform_wallet::WalletId;

use crate::kv::{validate_key, KvError, KvStore, MAX_VALUE_LEN};
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::persister::SqlitePersister;

/// Character used to escape `%`, `_`, and itself inside a user-supplied
/// `LIKE` prefix. Backslash matches what every shell-friendly schema
/// uses; SQLite is told about it via `ESCAPE '\'` in the query.
const LIKE_ESCAPE_CHAR: char = '\\';

/// Escape `%`, `_`, and `\` in `prefix` so it can be embedded in a
/// `LIKE` pattern as a literal. The caller must use `ESCAPE '\'` on
/// the SQL side.
fn escape_like_prefix(prefix: &str) -> String {
    let mut out = String::with_capacity(prefix.len());
    for ch in prefix.chars() {
        if ch == '%' || ch == '_' || ch == LIKE_ESCAPE_CHAR {
            out.push(LIKE_ESCAPE_CHAR);
        }
        out.push(ch);
    }
    out
}

/// Translate a `rusqlite` error from a `put` into a typed `KvError`.
/// A foreign-key violation against `wallet_metadata` only happens on
/// per-wallet writes and is surfaced as
/// [`KvError::WalletNotFound`] rather than the raw SQLite error so
/// callers don't have to inspect extended error codes.
fn classify_put_error(err: rusqlite::Error, wallet_id: Option<&WalletId>) -> KvError {
    if let rusqlite::Error::SqliteFailure(ffi_err, _) = &err {
        if ffi_err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY {
            if let Some(wid) = wallet_id {
                return KvError::WalletNotFound { wallet_id: *wid };
            }
        }
    }
    KvError::Sqlite(err)
}

impl From<WalletStorageError> for KvError {
    fn from(err: WalletStorageError) -> Self {
        match err {
            WalletStorageError::LockPoisoned => KvError::LockPoisoned,
            WalletStorageError::Sqlite(e) => KvError::Sqlite(e),
            other => {
                // Other variants don't arise from the `conn()` accessor
                // — the accessor either yields `LockPoisoned` or hands
                // back the guard. Stuff anything else into `Sqlite`
                // via its Display, preserving the source chain.
                KvError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(other)))
            }
        }
    }
}

impl KvStore for SqlitePersister {
    fn get(&self, wallet_id: Option<&WalletId>, key: &str) -> Result<Option<Vec<u8>>, KvError> {
        validate_key(key)?;
        let conn = self.conn().map_err(KvError::from)?;
        // Two-step read: pull `length(value)` first so a tampered or
        // corrupted row that exceeds `MAX_VALUE_LEN` cannot force a
        // multi-gigabyte allocation when we materialise the blob.
        // Mirrors `sqlite::schema::blob`'s `BLOB_SIZE_LIMIT_BYTES`
        // ceiling. CMT-006.
        let len: Option<i64> = match wallet_id {
            None => conn
                .query_row(
                    "SELECT length(value) FROM kv_store WHERE wallet_id IS NULL AND key = ?1",
                    params![key],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?,
            Some(wid) => conn
                .query_row(
                    "SELECT length(value) FROM kv_store WHERE wallet_id = ?1 AND key = ?2",
                    params![wid.as_slice(), key],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?,
        };
        let Some(len) = len else { return Ok(None) };
        let found = usize::try_from(len.max(0)).unwrap_or(usize::MAX);
        if found > MAX_VALUE_LEN {
            return Err(KvError::ValueTooLarge {
                found,
                max: MAX_VALUE_LEN,
            });
        }
        let value = match wallet_id {
            None => conn
                .query_row(
                    "SELECT value FROM kv_store WHERE wallet_id IS NULL AND key = ?1",
                    params![key],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .optional()?,
            Some(wid) => conn
                .query_row(
                    "SELECT value FROM kv_store WHERE wallet_id = ?1 AND key = ?2",
                    params![wid.as_slice(), key],
                    |row| row.get::<_, Vec<u8>>(0),
                )
                .optional()?,
        };
        Ok(value)
    }

    fn put(&self, wallet_id: Option<&WalletId>, key: &str, value: &[u8]) -> Result<(), KvError> {
        validate_key(key)?;
        let conn = self.conn().map_err(KvError::from)?;
        // Two SQL statements because the conflict target differs: the
        // global slot's uniqueness is enforced by the partial index
        // `idx_kv_store_global` (key only), the per-wallet slot by
        // `idx_kv_store_wallet` (wallet_id, key). Naming the index in
        // `ON CONFLICT` lets SQLite pick the right partial index even
        // though no UNIQUE constraint exists on the columns directly.
        let res = match wallet_id {
            None => conn.execute(
                "INSERT INTO kv_store (wallet_id, key, value) VALUES (NULL, ?1, ?2) \
                 ON CONFLICT(key) WHERE wallet_id IS NULL \
                 DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
                params![key, value],
            ),
            Some(wid) => conn.execute(
                "INSERT INTO kv_store (wallet_id, key, value) VALUES (?1, ?2, ?3) \
                 ON CONFLICT(wallet_id, key) WHERE wallet_id IS NOT NULL \
                 DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
                params![wid.as_slice(), key, value],
            ),
        };
        res.map(|_| ())
            .map_err(|e| classify_put_error(e, wallet_id))
    }

    fn delete(&self, wallet_id: Option<&WalletId>, key: &str) -> Result<(), KvError> {
        validate_key(key)?;
        let conn = self.conn().map_err(KvError::from)?;
        match wallet_id {
            None => conn.execute(
                "DELETE FROM kv_store WHERE wallet_id IS NULL AND key = ?1",
                params![key],
            )?,
            Some(wid) => conn.execute(
                "DELETE FROM kv_store WHERE wallet_id = ?1 AND key = ?2",
                params![wid.as_slice(), key],
            )?,
        };
        Ok(())
    }

    fn list_keys(
        &self,
        wallet_id: Option<&WalletId>,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, KvError> {
        let conn = self.conn().map_err(KvError::from)?;
        let escaped = prefix.map(escape_like_prefix);
        let pattern = escaped.as_ref().map(|p| format!("{p}%"));
        let collect = |mut stmt: rusqlite::Statement<'_>,
                       params: &[&dyn rusqlite::ToSql]|
         -> Result<Vec<String>, rusqlite::Error> {
            let rows = stmt.query_map(params, |row| row.get::<_, String>(0))?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        };
        let rows = match (wallet_id, pattern.as_deref()) {
            (None, None) => {
                let stmt = conn
                    .prepare("SELECT key FROM kv_store WHERE wallet_id IS NULL ORDER BY key ASC")?;
                collect(stmt, &[])?
            }
            (None, Some(p)) => {
                let stmt = conn.prepare(
                    "SELECT key FROM kv_store \
                     WHERE wallet_id IS NULL AND key LIKE ?1 ESCAPE '\\' \
                     ORDER BY key ASC",
                )?;
                collect(stmt, &[&p])?
            }
            (Some(wid), None) => {
                let stmt =
                    conn.prepare("SELECT key FROM kv_store WHERE wallet_id = ?1 ORDER BY key ASC")?;
                collect(stmt, &[&wid.as_slice()])?
            }
            (Some(wid), Some(p)) => {
                let stmt = conn.prepare(
                    "SELECT key FROM kv_store \
                     WHERE wallet_id = ?1 AND key LIKE ?2 ESCAPE '\\' \
                     ORDER BY key ASC",
                )?;
                collect(stmt, &[&wid.as_slice(), &p])?
            }
        };
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_persister() -> (SqlitePersister, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("wallet.db");
        let cfg = crate::sqlite::config::SqlitePersisterConfig::new(&path);
        let p = SqlitePersister::open(cfg).expect("open persister");
        (p, tmp)
    }

    fn seed_wallet(p: &SqlitePersister, id: u8) -> WalletId {
        use rusqlite::params;
        let wid: WalletId = [id; 32];
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT OR IGNORE INTO wallet_metadata (wallet_id, network, birth_height) \
             VALUES (?1, 'testnet', 0)",
            params![wid.as_slice()],
        )
        .expect("seed wallet_metadata");
        wid
    }

    #[test]
    fn escape_like_prefix_escapes_metachars() {
        assert_eq!(escape_like_prefix("a%b_c\\d"), "a\\%b\\_c\\\\d");
        assert_eq!(escape_like_prefix("plain.key"), "plain.key");
    }

    #[test]
    fn global_round_trip() {
        let (p, _tmp) = open_persister();
        assert_eq!(p.get(None, "k").unwrap(), None);
        p.put(None, "k", b"v1").unwrap();
        assert_eq!(p.get(None, "k").unwrap().as_deref(), Some(&b"v1"[..]));
        p.put(None, "k", b"v2").unwrap();
        assert_eq!(p.get(None, "k").unwrap().as_deref(), Some(&b"v2"[..]));
        p.delete(None, "k").unwrap();
        assert_eq!(p.get(None, "k").unwrap(), None);
    }

    #[test]
    fn per_wallet_round_trip() {
        let (p, _tmp) = open_persister();
        let wid = seed_wallet(&p, 1);
        p.put(Some(&wid), "k", b"v1").unwrap();
        assert_eq!(p.get(Some(&wid), "k").unwrap().as_deref(), Some(&b"v1"[..]));
        p.delete(Some(&wid), "k").unwrap();
        assert_eq!(p.get(Some(&wid), "k").unwrap(), None);
    }

    #[test]
    fn null_scope_uniqueness_via_partial_index() {
        // Direct INSERTs (no ON CONFLICT) must be rejected because the
        // partial index enforces global-key uniqueness.
        let (p, _tmp) = open_persister();
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT INTO kv_store (wallet_id, key, value) VALUES (NULL, 'k', X'01')",
            [],
        )
        .expect("first insert");
        let err = conn
            .execute(
                "INSERT INTO kv_store (wallet_id, key, value) VALUES (NULL, 'k', X'02')",
                [],
            )
            .expect_err("second insert must fail");
        match err {
            rusqlite::Error::SqliteFailure(ffi, _) => {
                assert_eq!(ffi.extended_code, rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE);
            }
            other => panic!("expected SQLITE_CONSTRAINT_UNIQUE, got {other:?}"),
        }
    }

    #[test]
    fn per_wallet_uniqueness_via_partial_index() {
        let (p, _tmp) = open_persister();
        let wid = seed_wallet(&p, 7);
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT INTO kv_store (wallet_id, key, value) VALUES (?1, 'k', X'01')",
            params![wid.as_slice()],
        )
        .expect("first insert");
        let err = conn
            .execute(
                "INSERT INTO kv_store (wallet_id, key, value) VALUES (?1, 'k', X'02')",
                params![wid.as_slice()],
            )
            .expect_err("second insert must fail");
        match err {
            rusqlite::Error::SqliteFailure(ffi, _) => {
                assert_eq!(ffi.extended_code, rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE);
            }
            other => panic!("expected SQLITE_CONSTRAINT_UNIQUE, got {other:?}"),
        }
    }

    #[test]
    fn get_rejects_oversized_value_before_materialising() {
        // CMT-006: a row larger than MAX_VALUE_LEN (planted via direct
        // SQL — bypassing `put`'s implicit cap) must surface as
        // ValueTooLarge instead of OOMing the process.
        use rusqlite::params;
        let (p, _tmp) = open_persister();
        let oversize = vec![0u8; MAX_VALUE_LEN + 1];
        {
            let conn = p.lock_conn_for_test();
            conn.execute(
                "INSERT INTO kv_store (wallet_id, key, value) VALUES (NULL, ?1, ?2)",
                params!["huge", oversize.as_slice()],
            )
            .expect("seed oversize row");
        }
        match p.get(None, "huge") {
            Err(KvError::ValueTooLarge { found, max }) => {
                assert_eq!(found, MAX_VALUE_LEN + 1);
                assert_eq!(max, MAX_VALUE_LEN);
            }
            other => panic!("expected ValueTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn cascade_on_wallet_delete() {
        use rusqlite::params;
        let (p, _tmp) = open_persister();
        let wid = seed_wallet(&p, 5);
        p.put(None, "global", b"survives").unwrap();
        p.put(Some(&wid), "scoped", b"cascades").unwrap();
        {
            let conn = p.lock_conn_for_test();
            conn.execute(
                "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
                params![wid.as_slice()],
            )
            .expect("delete wallet");
        }
        assert_eq!(p.get(Some(&wid), "scoped").unwrap(), None);
        assert_eq!(
            p.get(None, "global").unwrap().as_deref(),
            Some(&b"survives"[..])
        );
    }
}
