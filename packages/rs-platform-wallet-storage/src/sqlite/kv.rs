//! SQLite-backed [`KvStore`] implementation for [`SqlitePersister`].
//!
//! One dedicated table per [`ObjectId`] variant (`meta_global`,
//! `meta_wallet`, `meta_identity`, `meta_token`, `meta_contact`,
//! `meta_platform_address`). Each table has a composite PRIMARY KEY of
//! its id column(s) plus `key`, so uniqueness comes straight from the PK
//! — no partial indexes, no nullable scope column. None of the tables
//! carry an FK: a `put` succeeds before its parent object exists. The
//! `AFTER DELETE` triggers in `V001__initial.rs` clean a scope's
//! metadata up when its parent is deleted.
//!
//! `match scope` resolves each operation to its table and id-column
//! bindings; the SQL body (length-precheck read, upsert, delete,
//! prefix-list) is factored once per op and parameterised by table name
//! and id predicate. Table and column names come from the matched
//! variant — compile-time constants, never caller input — so they are
//! `format!`-spliced; the id *values* are bound parameters.
//!
//! All operations reuse `SqlitePersister`'s single `Mutex<Connection>`
//! via the crate-private `conn()` accessor; no separate connection is
//! opened.

use rusqlite::{OptionalExtension, ToSql};

use crate::kv::{validate_key, KvError, KvStore, ObjectId, MAX_VALUE_LEN};
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

/// Resolved table + id-column bindings for one [`ObjectId`] scope. The
/// `table` name and `id_cols` are compile-time constants from the
/// matched variant; `id_vals` are the owned byte buffers bound as
/// parameters. `key` (and `prefix`, where used) bind after the id
/// values, in the order the SQL placeholders appear.
struct ScopeSql<'a> {
    table: &'static str,
    id_cols: &'static [&'static str],
    id_vals: Vec<&'a [u8]>,
}

impl<'a> ScopeSql<'a> {
    /// Resolve a scope to its table and id bindings.
    fn resolve(scope: &'a ObjectId) -> Self {
        match scope {
            ObjectId::Global => ScopeSql {
                table: "meta_global",
                id_cols: &[],
                id_vals: Vec::new(),
            },
            ObjectId::Wallet(wallet_id) => ScopeSql {
                table: "meta_wallet",
                id_cols: &["wallet_id"],
                id_vals: vec![wallet_id.as_slice()],
            },
            ObjectId::Identity(identity_id) => ScopeSql {
                table: "meta_identity",
                id_cols: &["identity_id"],
                id_vals: vec![identity_id.as_slice()],
            },
            ObjectId::Token {
                identity_id,
                token_id,
            } => ScopeSql {
                table: "meta_token",
                id_cols: &["identity_id", "token_id"],
                id_vals: vec![identity_id.as_slice(), token_id.as_slice()],
            },
            ObjectId::Contact {
                wallet_id,
                owner_id,
                contact_id,
            } => ScopeSql {
                table: "meta_contact",
                id_cols: &["wallet_id", "owner_id", "contact_id"],
                id_vals: vec![
                    wallet_id.as_slice(),
                    owner_id.as_slice(),
                    contact_id.as_slice(),
                ],
            },
            ObjectId::PlatformAddress { wallet_id, address } => ScopeSql {
                table: "meta_platform_address",
                id_cols: &["wallet_id", "address"],
                id_vals: vec![wallet_id.as_slice(), address.as_slice()],
            },
        }
    }

    /// `col1 = ?1 AND col2 = ?2 AND …` for the id columns, numbered from
    /// 1. Empty (no fragment) for the id-less global scope.
    fn id_predicate(&self) -> String {
        self.id_cols
            .iter()
            .enumerate()
            .map(|(i, col)| format!("{col} = ?{}", i + 1))
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    /// `WHERE <id_predicate> AND key = ?n` (or `WHERE key = ?1` when the
    /// scope has no id columns), with `key` bound at the next slot.
    fn where_key(&self) -> String {
        let key_ph = self.id_cols.len() + 1;
        if self.id_cols.is_empty() {
            format!("WHERE key = ?{key_ph}")
        } else {
            format!("WHERE {} AND key = ?{key_ph}", self.id_predicate())
        }
    }
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
    fn get(&self, scope: &ObjectId, key: &str) -> Result<Option<Vec<u8>>, KvError> {
        validate_key(key)?;
        let sql = ScopeSql::resolve(scope);
        let conn = self.conn().map_err(KvError::from)?;
        let where_key = sql.where_key();
        // Bind the id values then the key in placeholder order.
        let mut params: Vec<&dyn ToSql> = sql.id_vals.iter().map(|v| v as &dyn ToSql).collect();
        params.push(&key);
        // Single-snapshot read: select `length(value)` and `value` in one
        // row. The length (column 0) is checked against `MAX_VALUE_LEN`
        // before `row.get(1)` materialises the BLOB — rusqlite reads the
        // BLOB lazily on that call, so the cap gates the allocation with
        // no cross-snapshot TOCTOU window. The inner `Result` carries the
        // over-cap length out of the closure without ever touching
        // column 1.
        let row: Option<Result<Vec<u8>, usize>> = conn
            .query_row(
                &format!("SELECT length(value), value FROM {} {where_key}", sql.table),
                params.as_slice(),
                |row| {
                    let len = row.get::<_, i64>(0)?;
                    let found = usize::try_from(len.max(0)).unwrap_or(usize::MAX);
                    if found > MAX_VALUE_LEN {
                        return Ok(Err(found));
                    }
                    Ok(Ok(row.get::<_, Vec<u8>>(1)?))
                },
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some(Ok(value)) => Ok(Some(value)),
            Some(Err(found)) => Err(KvError::ValueTooLarge {
                found,
                max: MAX_VALUE_LEN,
            }),
        }
    }

    fn put(&self, scope: &ObjectId, key: &str, value: &[u8]) -> Result<(), KvError> {
        validate_key(key)?;
        // Cap the value before it reaches SQL so a `put` can never plant
        // a row that a later `get` would refuse to materialise. The read
        // path gates on the same `MAX_VALUE_LEN`.
        if value.len() > MAX_VALUE_LEN {
            return Err(KvError::ValueTooLarge {
                found: value.len(),
                max: MAX_VALUE_LEN,
            });
        }
        let sql = ScopeSql::resolve(scope);
        let conn = self.conn().map_err(KvError::from)?;
        // Column list / placeholders / conflict target all include the
        // id columns ahead of `key`; the plain composite PK is the
        // conflict target. Upsert refreshes `updated_at` on overwrite.
        let mut cols: Vec<&str> = sql.id_cols.to_vec();
        cols.push("key");
        let col_list = cols.join(", ");
        let value_phs = (1..=cols.len())
            .map(|i| format!("?{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        let conflict_target = cols.join(", ");
        let value_ph = cols.len() + 1;
        let mut params: Vec<&dyn ToSql> = sql.id_vals.iter().map(|v| v as &dyn ToSql).collect();
        params.push(&key);
        params.push(&value);
        conn.execute(
            &format!(
                "INSERT INTO {} ({col_list}, value) VALUES ({value_phs}, ?{value_ph}) \
                 ON CONFLICT({conflict_target}) \
                 DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
                sql.table
            ),
            params.as_slice(),
        )?;
        Ok(())
    }

    fn delete(&self, scope: &ObjectId, key: &str) -> Result<(), KvError> {
        validate_key(key)?;
        let sql = ScopeSql::resolve(scope);
        let conn = self.conn().map_err(KvError::from)?;
        let mut params: Vec<&dyn ToSql> = sql.id_vals.iter().map(|v| v as &dyn ToSql).collect();
        params.push(&key);
        conn.execute(
            &format!("DELETE FROM {} {}", sql.table, sql.where_key()),
            params.as_slice(),
        )?;
        Ok(())
    }

    fn list_keys(&self, scope: &ObjectId, prefix: Option<&str>) -> Result<Vec<String>, KvError> {
        let sql = ScopeSql::resolve(scope);
        let conn = self.conn().map_err(KvError::from)?;
        let pattern = prefix.map(|p| format!("{}%", escape_like_prefix(p)));
        // id predicate first; `key LIKE ?` (escaped) appended when a
        // prefix is supplied, bound at the slot after the id values.
        let id_pred = sql.id_predicate();
        let where_clause = match (&pattern, id_pred.is_empty()) {
            (None, true) => String::new(),
            (None, false) => format!("WHERE {id_pred}"),
            (Some(_), true) => format!("WHERE key LIKE ?{} ESCAPE '\\'", sql.id_cols.len() + 1),
            (Some(_), false) => {
                format!(
                    "WHERE {id_pred} AND key LIKE ?{} ESCAPE '\\'",
                    sql.id_cols.len() + 1
                )
            }
        };
        let mut params: Vec<&dyn ToSql> = sql.id_vals.iter().map(|v| v as &dyn ToSql).collect();
        if let Some(p) = pattern.as_ref() {
            params.push(p);
        }
        let mut stmt = conn.prepare(&format!(
            "SELECT key FROM {} {where_clause} ORDER BY key ASC",
            sql.table
        ))?;
        let rows = stmt.query_map(params.as_slice(), |row| row.get::<_, String>(0))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet::wallet::platform_wallet::WalletId;
    use rusqlite::params;

    fn open_persister() -> (SqlitePersister, tempfile::TempDir) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("wallet.db");
        let cfg = crate::sqlite::config::SqlitePersisterConfig::new(&path);
        let p = SqlitePersister::open(cfg).expect("open persister");
        (p, tmp)
    }

    fn seed_wallet(p: &SqlitePersister, id: u8) -> WalletId {
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
        assert_eq!(p.get(&ObjectId::Global, "k").unwrap(), None);
        p.put(&ObjectId::Global, "k", b"v1").unwrap();
        assert_eq!(
            p.get(&ObjectId::Global, "k").unwrap().as_deref(),
            Some(&b"v1"[..])
        );
        p.put(&ObjectId::Global, "k", b"v2").unwrap();
        assert_eq!(
            p.get(&ObjectId::Global, "k").unwrap().as_deref(),
            Some(&b"v2"[..])
        );
        p.delete(&ObjectId::Global, "k").unwrap();
        assert_eq!(p.get(&ObjectId::Global, "k").unwrap(), None);
    }

    #[test]
    fn per_wallet_round_trip() {
        let (p, _tmp) = open_persister();
        let scope = ObjectId::Wallet(seed_wallet(&p, 1));
        p.put(&scope, "k", b"v1").unwrap();
        assert_eq!(p.get(&scope, "k").unwrap().as_deref(), Some(&b"v1"[..]));
        p.delete(&scope, "k").unwrap();
        assert_eq!(p.get(&scope, "k").unwrap(), None);
    }

    #[test]
    fn global_composite_pk_rejects_duplicate() {
        // Direct INSERTs (no ON CONFLICT) must be rejected because the
        // composite PRIMARY KEY enforces per-key uniqueness.
        let (p, _tmp) = open_persister();
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT INTO meta_global (key, value) VALUES ('k', X'01')",
            [],
        )
        .expect("first insert");
        let err = conn
            .execute(
                "INSERT INTO meta_global (key, value) VALUES ('k', X'02')",
                [],
            )
            .expect_err("second insert must fail");
        match err {
            rusqlite::Error::SqliteFailure(ffi, _) => {
                assert_eq!(
                    ffi.extended_code,
                    rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
                );
            }
            other => panic!("expected SQLITE_CONSTRAINT_PRIMARYKEY, got {other:?}"),
        }
    }

    #[test]
    fn per_wallet_composite_pk_rejects_duplicate() {
        let (p, _tmp) = open_persister();
        let wid = seed_wallet(&p, 7);
        let conn = p.lock_conn_for_test();
        conn.execute(
            "INSERT INTO meta_wallet (wallet_id, key, value) VALUES (?1, 'k', X'01')",
            params![wid.as_slice()],
        )
        .expect("first insert");
        let err = conn
            .execute(
                "INSERT INTO meta_wallet (wallet_id, key, value) VALUES (?1, 'k', X'02')",
                params![wid.as_slice()],
            )
            .expect_err("second insert must fail");
        match err {
            rusqlite::Error::SqliteFailure(ffi, _) => {
                assert_eq!(
                    ffi.extended_code,
                    rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
                );
            }
            other => panic!("expected SQLITE_CONSTRAINT_PRIMARYKEY, got {other:?}"),
        }
    }

    #[test]
    fn get_rejects_oversized_value_before_materialising() {
        // A row larger than MAX_VALUE_LEN (planted via direct SQL —
        // bypassing `put`'s cap) must surface as ValueTooLarge instead
        // of OOMing the process.
        let (p, _tmp) = open_persister();
        let oversize = vec![0u8; MAX_VALUE_LEN + 1];
        {
            let conn = p.lock_conn_for_test();
            conn.execute(
                "INSERT INTO meta_global (key, value) VALUES (?1, ?2)",
                params!["huge", oversize.as_slice()],
            )
            .expect("seed oversize row");
        }
        match p.get(&ObjectId::Global, "huge") {
            Err(KvError::ValueTooLarge { found, max }) => {
                assert_eq!(found, MAX_VALUE_LEN + 1);
                assert_eq!(max, MAX_VALUE_LEN);
            }
            other => panic!("expected ValueTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn put_rejects_oversized_value_and_writes_nothing() {
        // A value over MAX_VALUE_LEN is refused before the INSERT, so no
        // row lands and a follow-up `get` returns None.
        let (p, _tmp) = open_persister();
        let oversize = vec![0u8; MAX_VALUE_LEN + 1];
        match p.put(&ObjectId::Global, "huge", &oversize) {
            Err(KvError::ValueTooLarge { found, max }) => {
                assert_eq!(found, MAX_VALUE_LEN + 1);
                assert_eq!(max, MAX_VALUE_LEN);
            }
            other => panic!("expected ValueTooLarge, got {other:?}"),
        }
        assert_eq!(
            p.get(&ObjectId::Global, "huge").unwrap(),
            None,
            "rejected put must not leave a row behind"
        );
    }

    #[test]
    fn cascade_on_wallet_delete() {
        let (p, _tmp) = open_persister();
        let scope = ObjectId::Wallet(seed_wallet(&p, 5));
        p.put(&ObjectId::Global, "global", b"survives").unwrap();
        p.put(&scope, "scoped", b"cascades").unwrap();
        {
            let conn = p.lock_conn_for_test();
            let ObjectId::Wallet(wid) = &scope else {
                unreachable!()
            };
            conn.execute(
                "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
                params![wid.as_slice()],
            )
            .expect("delete wallet");
        }
        assert_eq!(p.get(&scope, "scoped").unwrap(), None);
        assert_eq!(
            p.get(&ObjectId::Global, "global").unwrap().as_deref(),
            Some(&b"survives"[..])
        );
    }
}
