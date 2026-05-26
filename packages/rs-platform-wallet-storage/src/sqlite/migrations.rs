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
    let has_table = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !has_table {
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
