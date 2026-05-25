//! Single connection-open choke-point.
//!
//! `PRAGMA foreign_keys` is per-connection and resets to OFF on every
//! open — it is not persisted in the database file, and no compile-time
//! knob in `libsqlite3-sys`'s bundled build forces it on. Enforcement is
//! therefore a runtime discipline: every connection that mutates rows
//! must enable it, and we must *prove* it took, because the pragma
//! silently no-ops on a SQLite built without FK support.
//!
//! Every library connection-open site routes through [`open_conn`] so
//! there is exactly one place that owns flags + FK enforcement. The CLI
//! binary's read-only `peek_schema_version` probe opens directly — it
//! never mutates rows, so FK enforcement is moot, and `open_conn` is
//! `pub(crate)` (not reachable from the separate bin target).

use rusqlite::{Connection, OpenFlags};
use std::path::Path;

use crate::sqlite::error::WalletStorageError;

/// How the opened connection will be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Access {
    /// Read-write writer connection. Enables `foreign_keys` and asserts
    /// the read-back equals 1.
    ReadWrite,
    /// Read-only handle (backup source, restore probe, CLI peek). FK
    /// enforcement is irrelevant — no mutations flow through it — so the
    /// pragma + read-back are skipped.
    ReadOnly,
}

/// Open a SQLite connection through the crate's single choke-point.
///
/// For [`Access::ReadWrite`], enables `PRAGMA foreign_keys = ON` and
/// reads it back, returning [`WalletStorageError::ForeignKeysNotEnforced`]
/// if the result is not `1`. For [`Access::ReadOnly`], opens with
/// `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_URI` and performs no pragma.
pub(crate) fn open_conn(path: &Path, access: Access) -> Result<Connection, WalletStorageError> {
    let conn = match access {
        Access::ReadWrite => Connection::open(path)?,
        Access::ReadOnly => Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
        )?,
    };
    if access == Access::ReadWrite {
        enforce_foreign_keys(&conn)?;
    }
    Ok(conn)
}

/// Enable `foreign_keys` and assert via read-back. Separated so the
/// writer can call it after re-opening through other paths if needed.
pub(crate) fn enforce_foreign_keys(conn: &Connection) -> Result<(), WalletStorageError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let on: i64 = conn.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
    if on != 1 {
        return Err(WalletStorageError::ForeignKeysNotEnforced);
    }
    Ok(())
}

/// Flip `PRAGMA foreign_keys` on or off explicitly. Used by the
/// migration runner to disable FK enforcement around the V002 schema
/// rewrite (DROP TABLE on a parent fires ON DELETE CASCADE on its
/// children otherwise, wiping the rows the migration is trying to
/// preserve). The pragma cannot be flipped inside an open
/// transaction, so the caller must invoke this before BEGIN.
pub(crate) fn set_foreign_keys(conn: &Connection, on: bool) -> Result<(), WalletStorageError> {
    conn.pragma_update(None, "foreign_keys", if on { "ON" } else { "OFF" })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A read-write open enables FK and the read-back confirms it — the
    /// assertion path that guards against a silently no-op pragma.
    #[test]
    fn read_write_open_enforces_and_reads_back_foreign_keys() {
        let conn = Connection::open_in_memory().expect("in-memory open");
        enforce_foreign_keys(&conn).expect("FK enforcement");
        let on: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .expect("read-back");
        assert_eq!(on, 1, "read-back must observe FK enforcement is on");
    }

    /// The hard-error variant the read-back returns when the pragma is a
    /// no-op is wired and reachable. We can't build a FK-less SQLite in
    /// the bundled build, so assert the typed error renders the intended
    /// message rather than truncating the contract to "untestable".
    #[test]
    fn foreign_keys_not_enforced_variant_renders() {
        let err = WalletStorageError::ForeignKeysNotEnforced;
        assert!(format!("{err}").contains("foreign-key enforcement"));
    }
}
