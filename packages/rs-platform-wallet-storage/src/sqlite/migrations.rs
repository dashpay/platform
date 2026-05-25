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

/// Apply migrations on behalf of [`crate::sqlite::persister::SqlitePersister::open`],
/// re-classifying the V002 sentinel-row CHECK failure into a typed
/// [`WalletStorageError::MigrationRequiresManualCleanup`] so operators
/// see what the migration refused on instead of a raw rusqlite error.
///
/// V002 reshapes identity-owned tables, which involves
/// `DROP TABLE identities` while child tables (`identity_keys`,
/// `dashpay_profiles`, etc.) carry `ON DELETE CASCADE` FK clauses
/// pointing at it. SQLite's drop-of-parent semantics fire those
/// CASCADE clauses when FK enforcement is on, wiping the rows we're
/// trying to migrate. FK enforcement cannot be toggled inside a
/// transaction, so the pragma is flipped off here for the migration
/// window and re-enabled (with assertion) right after — matching
/// `open_conn`'s normal invariant.
pub(crate) fn run_for_open(
    conn: &mut rusqlite::Connection,
) -> Result<refinery::Report, WalletStorageError> {
    crate::sqlite::conn::set_foreign_keys(conn, false)?;
    let result = run(conn);
    // Always restore FK enforcement, even on migration error, so the
    // caller's connection is in the documented state.
    crate::sqlite::conn::enforce_foreign_keys(conn)?;
    match result {
        Ok(r) => Ok(r),
        Err(e) => {
            // The V002 guard uses a CHECK on the temp table
            // `_v002_sentinel_rows_must_be_zero`; SQLite surfaces that
            // as a ConstraintViolation whose message names the table.
            if migration_failure_is_sentinel(&e) {
                // Re-query the live `token_balances` table for the
                // count so the typed error carries the actual number
                // of offending rows. The CHECK message itself does not
                // expose it.
                let count: i64 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM token_balances \
                         WHERE wallet_id = X'0000000000000000000000000000000000000000000000000000000000000000'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap_or(0);
                return Err(WalletStorageError::MigrationRequiresManualCleanup {
                    table: "token_balances",
                    count,
                });
            }
            Err(WalletStorageError::Migration(e))
        }
    }
}

/// Recognise the V002 sentinel-guard failure by walking the error
/// chain for a `ConstraintViolation` whose message names the guard
/// table.
fn migration_failure_is_sentinel(err: &refinery::Error) -> bool {
    let mut source: Option<&dyn std::error::Error> = Some(err);
    while let Some(s) = source {
        // SQLite surfaces the failing CHECK by predicate text
        // (`CHECK constraint failed: sentinel_count = 0`), not by table
        // name. `sentinel_count` is unique to the V002 guard temp
        // table; matching on the column name keeps the detector tight
        // without false positives.
        if let Some(rusqlite::Error::SqliteFailure(code, msg)) = s.downcast_ref::<rusqlite::Error>()
        {
            if matches!(code.code, rusqlite::ErrorCode::ConstraintViolation)
                && msg.as_deref().is_some_and(|m| m.contains("sentinel_count"))
            {
                return true;
            }
        }
        source = s.source();
    }
    false
}

/// Return a fresh refinery [`Runner`](refinery::Runner) seeded with the
/// embedded migration list. Used by tests that need to apply a subset
/// of migrations via [`refinery::Runner::set_target`].
pub fn runner() -> refinery::Runner {
    migrations::runner()
}

/// Highest migration version this binary knows how to apply. Used by
/// both `SqlitePersister::open` (CMT-005) and `backup::restore_from`
/// (CMT-001 / CMT-010) to refuse forward-version databases.
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
/// the migration-drift hash check (TC-029).
pub fn embedded_migrations() -> Vec<(i32, String)> {
    migrations::runner()
        .get_migrations()
        .iter()
        .map(|m| (m.version(), m.name().to_string()))
        .collect()
}

/// SHA-256 over `(version, name)` of every embedded migration in version
/// order. Pinning this in tests catches edits to committed migrations
/// (forbidden by NFR-8 append-only policy).
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

    /// TC-CODE-027-1: helper returns false on a brand-new in-memory DB
    /// (no `refinery_schema_history`), and true after the table is
    /// created.
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
}
