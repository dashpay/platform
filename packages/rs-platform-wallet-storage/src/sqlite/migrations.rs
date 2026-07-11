//! Schema migration plumbing.
//!
//! Embeds every Rust migration under `migrations/` at compile time
//! (see `refinery::embed_migrations!`). The `run` function applies any
//! pending migrations to the supplied connection.

use rusqlite::OptionalExtension;

use crate::sqlite::error::WalletStorageError;

// `embed_migrations!` generates a `migrations` module with a `runner()`
// function. The path is relative to the crate root (where `Cargo.toml`
// lives).
refinery::embed_migrations!("./migrations");

/// Apply every pending migration to `conn`.
pub fn run(conn: &mut rusqlite::Connection) -> Result<refinery::Report, refinery::Error> {
    migrations::runner().run(conn)
}

/// Apply migrations on behalf of [`crate::sqlite::persister::SqlitePersister::open`].
///
/// Plain wrapper today — V001 ships the final identity-cascade shape so
/// there is no FK-toggle dance or sentinel re-classification needed.
/// Kept as a typed-error chokepoint so future migrations that DO need
/// to re-classify a refinery error have a single entry point.
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

/// Refuse to operate on a database whose `refinery_schema_history`
/// MAX(version) exceeds [`max_supported_version`]. Returns
/// [`WalletStorageError::SchemaVersionUnsupported`] in that case.
///
/// Quietly succeeds when the table is absent (caller decides whether a
/// missing schema-history is itself an error — `restore_from` rejects
/// it, `open` treats it as "brand-new DB about to be migrated").
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
/// order. Pinning this in tests catches edits to committed migrations
/// (forbidden by the append-only migration policy).
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

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
    /// these migrations), so V001 is edited in place rather than amended by a
    /// follow-on migration — no real database has ever applied it. This test
    /// pins that the objects exist after the (only) migration runs.
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
