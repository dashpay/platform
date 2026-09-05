//! Schema migration plumbing.
//!
//! Embeds every Rust migration under `migrations/` at compile time
//! (see `refinery::embed_migrations!`). The `run` function applies any
//! pending migrations to the supplied connection.

use rusqlite::OptionalExtension;

use crate::sqlite::error::WalletStorageError;

// Generates a `migrations` module with `runner()`; path is relative to
// the crate root.
refinery::embed_migrations!("./migrations");

/// Apply every pending migration to `conn`.
pub fn run(conn: &mut rusqlite::Connection) -> Result<refinery::Report, refinery::Error> {
    migrations::runner().run(conn)
}

/// Apply migrations on behalf of [`crate::sqlite::persister::SqlitePersister::open`].
///
/// A typed-error chokepoint: a single entry point for any future
/// migration that needs to re-classify a refinery error.
pub(crate) fn run_for_open(
    conn: &mut rusqlite::Connection,
) -> Result<refinery::Report, WalletStorageError> {
    run(conn).map_err(WalletStorageError::Migration)
}

/// Return a fresh refinery [`Runner`](refinery::Runner) seeded with the
/// embedded migration list. Used by tests that need to apply a subset
/// of migrations via [`refinery::Runner::set_target`].
#[cfg(any(test, feature = "__test-helpers"))]
pub fn runner() -> refinery::Runner {
    migrations::runner()
}

/// Highest migration version this binary knows how to apply. Used by
/// both `SqlitePersister::open` and `backup::restore_from` to refuse
/// forward-version databases.
pub fn max_supported_version() -> i64 {
    embedded_migrations()
        .iter()
        .map(|(v, _)| *v as i64)
        .max()
        .unwrap_or(0)
}

/// Returns true if the `refinery_schema_history` table exists on this
/// connection. Used by `open`, `restore_from`, and `count_pending` to
/// distinguish "fresh DB" (no migrations applied yet) from
/// "pre-existing DB" (carries refinery history).
pub(crate) fn has_schema_history(conn: &rusqlite::Connection) -> Result<bool, WalletStorageError> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(exists)
}

/// Returns true if the database already carries any schema object (table,
/// index, view, or trigger). `open` uses it to tell a brand-new empty DB
/// apart from a pre-existing NON-wallet SQLite file that lacks
/// `refinery_schema_history`: the former is migrated, the latter rejected
/// rather than silently grafting wallet tables onto a foreign schema.
pub(crate) fn db_has_objects(conn: &rusqlite::Connection) -> Result<bool, WalletStorageError> {
    let exists = conn
        .query_row("SELECT 1 FROM sqlite_master LIMIT 1", [], |_| Ok(()))
        .optional()?
        .is_some();
    Ok(exists)
}

/// Refuse to operate on a DB whose `refinery_schema_history` MAX(version)
/// exceeds [`max_supported_version`], returning
/// [`WalletStorageError::SchemaVersionUnsupported`]. This is a forward-only
/// gate — it refuses a newer DB but never migrates it down (SQLite
/// migrations are one-directional).
///
/// Quietly succeeds when the table is absent; the caller decides what a
/// missing schema-history means (`restore_from` rejects it, `open` treats
/// it as a brand-new DB).
pub fn assert_schema_version_supported(
    conn: &rusqlite::Connection,
) -> Result<(), WalletStorageError> {
    if !has_schema_history(conn)? {
        return Ok(());
    }
    let source_version: Option<i64> = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let max_supported = max_supported_version();
    if let Some(v) = source_version {
        if v > max_supported {
            return Err(WalletStorageError::SchemaVersionUnsupported {
                found: v,
                max_supported,
            });
        }
    }
    Ok(())
}

fn refinery_timestamp_is_valid(value: &str) -> bool {
    let bytes = value.as_bytes();
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    let shape_is_valid = bytes.len() >= 20
        && bytes.len() <= 30
        && digits(0..4)
        && bytes[4] == b'-'
        && digits(5..7)
        && bytes[7] == b'-'
        && digits(8..10)
        && bytes[10] == b'T'
        && digits(11..13)
        && bytes[13] == b':'
        && digits(14..16)
        && bytes[16] == b':'
        && digits(17..19)
        && bytes[17] <= b'5'
        && bytes[bytes.len() - 1] == b'Z'
        && (bytes.len() == 20 || (bytes[19] == b'.' && digits(20..bytes.len().saturating_sub(1))));

    shape_is_valid && chrono::DateTime::parse_from_rfc3339(value).is_ok()
}

/// Probe `refinery_schema_history` rows BEFORE handing the connection to
/// refinery, which parses `applied_on` (RFC3339) and `checksum` (`u64`)
/// with `unwrap()` — a malformed value would abort the process. Surfaces
/// a typed [`WalletStorageError::SchemaHistoryMalformed`] instead.
/// Quietly succeeds when the table is absent.
pub(crate) fn assert_schema_history_well_formed(
    conn: &rusqlite::Connection,
) -> Result<(), WalletStorageError> {
    if !has_schema_history(conn)? {
        return Ok(());
    }
    let mut stmt = conn.prepare("SELECT applied_on, checksum FROM refinery_schema_history")?;
    let rows = stmt.query_map([], |row| {
        let applied_on: String = row.get(0)?;
        let checksum: String = row.get(1)?;
        Ok((applied_on, checksum))
    })?;
    for row in rows {
        let (applied_on, checksum) = row?;
        // Deliberately NARROWER than refinery's own reader, which accepts any
        // RFC3339 value (`drivers/rusqlite.rs` parses with `time::Rfc3339`).
        // We can be narrower because refinery only ever WRITES the canonical
        // UTC shape: `runner.rs` stamps `OffsetDateTime::now_utc()` and
        // `traits/mod.rs` formats it with `time::Rfc3339`, which emits a
        // trailing `Z` for a UTC offset rather than `+00:00`. So every value
        // refinery produced passes this gate, while a hand-written `+00:00`
        // is refused — a typed error, which is the point: the panic this
        // guard replaces is unrecoverable.
        if !refinery_timestamp_is_valid(&applied_on) {
            return Err(WalletStorageError::SchemaHistoryMalformed {
                reason: "applied_on is not a valid RFC3339 timestamp",
            });
        }
        if checksum.parse::<u64>().is_err() {
            return Err(WalletStorageError::SchemaHistoryMalformed {
                reason: "checksum is not a valid u64",
            });
        }
    }
    Ok(())
}

/// List `(version, name)` of every embedded migration. Used by tests and
/// the migration-drift hash check.
pub fn embedded_migrations() -> Vec<(i32, String)> {
    migrations::runner()
        .get_migrations()
        .iter()
        .map(|m| (m.version(), m.name().to_string()))
        .collect()
}

/// SHA-256 over `(version, name)` of every embedded migration in version
/// order. Deliberately content-blind: it hashes the migration set's
/// identity, not the SQL bodies, so it catches an added/removed/renamed
/// migration but ignores in-place DDL edits (a content-pinning guard
/// belongs with the schema freeze at release).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_fingerprint() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut entries = embedded_migrations();
    entries.sort_by_key(|(v, _)| *v);
    let mut hasher = Sha256::new();
    for (v, name) in entries {
        hasher.update(v.to_be_bytes());
        hasher.update([0u8]);
        hasher.update(name.as_bytes());
        hasher.update([0u8]);
    }
    hasher.finalize().into()
}

/// SHA-256 over `(version, name, rendered SQL)` of every embedded migration
/// in version order. Unlike [`embedded_migrations_fingerprint`] this is
/// content-level: it pins each migration's SQL body, so an in-place DDL edit
/// (e.g. renaming a table inside a same-named file) breaks the golden test.
/// This is the guard the D0 schema freeze relies on; the identity-only
/// fingerprint cannot catch a same-name body edit.
///
/// The SQL *text* is deterministic even where a value is generated at run
/// time (`randomblob(16)`): the literal string is hashed, not the runtime
/// bytes.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_sql_fingerprint() -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut migrations = migrations::runner().get_migrations().clone();
    migrations.sort_by_key(|m| m.version());
    let mut hasher = Sha256::new();
    for m in &migrations {
        hasher.update((m.version() as u32).to_be_bytes());
        hasher.update([0u8]);
        hasher.update(m.name().as_bytes());
        hasher.update([0u8]);
        let sql = m
            .sql()
            .expect("embedded migrations always carry rendered SQL");
        hasher.update(sql.as_bytes());
        hasher.update([0u8]);
    }
    hasher.finalize().into()
}

/// Rendered SQL of every embedded migration, in version order. Used by the
/// schema-freeze grep guard to scan for retired table names.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn embedded_migrations_sql() -> Vec<String> {
    let mut migrations = migrations::runner().get_migrations().clone();
    migrations.sort_by_key(|m| m.version());
    migrations
        .iter()
        .map(|m| {
            m.sql()
                .expect("embedded migrations always carry rendered SQL")
                .to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{params, Connection};

    #[test]
    fn schema_history_guard_rejects_timestamps_refinery_rejects() {
        for applied_on in ["2024-01-01T00:00:00−00:00", "2024-01-01T00:00:60Z"] {
            let conn = Connection::open_in_memory().unwrap();
            conn.execute_batch(
                "CREATE TABLE refinery_schema_history (
                    applied_on TEXT NOT NULL,
                    checksum TEXT NOT NULL
                )",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO refinery_schema_history (applied_on, checksum) VALUES (?1, '0')",
                params![applied_on],
            )
            .unwrap();

            assert!(matches!(
                assert_schema_history_well_formed(&conn),
                Err(WalletStorageError::SchemaHistoryMalformed { .. })
            ));
        }
    }

    /// The helper returns false on a brand-new in-memory DB (no
    /// `refinery_schema_history`), and true after the table is created.
    #[test]
    fn has_schema_history_distinguishes_fresh_vs_migrated() {
        let conn = Connection::open_in_memory().unwrap();
        assert!(
            !has_schema_history(&conn).unwrap(),
            "fresh in-memory DB has no schema-history table"
        );
        conn.execute(
            "CREATE TABLE refinery_schema_history (version INTEGER PRIMARY KEY)",
            [],
        )
        .unwrap();
        assert!(
            has_schema_history(&conn).unwrap(),
            "schema-history table is present after creation"
        );
    }

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |_| Ok(()),
        )
        .is_ok()
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        cols.iter().any(|c| c == column)
    }

    /// The initial schema (V001) creates the DashPay sync-correctness
    /// objects directly — the `contacts.payment_channel_broken` column and
    /// the `ignored_senders` table. The storage crate is pre-release with no
    /// product consumers yet (nothing instantiates `SqlitePersister` or runs
    /// these migrations), so every migration remains editable in place until
    /// the crate's first release. This test pins that the objects exist after
    /// the full migration set runs.
    #[test]
    fn v001_creates_dashpay_sync_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        run(&mut conn).unwrap();

        assert!(
            table_exists(&conn, "ignored_senders"),
            "V001 must create the ignored-senders table"
        );
        assert!(
            column_exists(&conn, "contacts", "payment_channel_broken"),
            "V001 must create the contacts.payment_channel_broken column"
        );
    }
}
