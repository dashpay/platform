//! Single connection-open choke-point.
//!
//! `PRAGMA foreign_keys` and `PRAGMA recursive_triggers` are both
//! per-connection and reset to OFF on every open — neither is persisted
//! in the database file, and no compile-time knob in `libsqlite3-sys`'s
//! bundled build forces them on. Enforcement is therefore a runtime
//! discipline: every connection that mutates rows must enable them, and
//! we must *prove* each took, because the pragma silently no-ops on a
//! SQLite built without the corresponding support. `recursive_triggers`
//! is required so the `meta_*` soft-cascade `AFTER DELETE` triggers fire
//! for parents removed via an FK cascade (e.g. `delete_wallet` →
//! `identities` cascade → `meta_identity` trigger), not just for
//! directly-deleted parents.
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
    /// Read-write writer connection. Enables `foreign_keys` and
    /// `recursive_triggers` and asserts each read-back equals 1.
    ReadWrite,
    /// Read-only handle (backup source, restore probe, CLI peek). FK and
    /// trigger enforcement is irrelevant — no mutations flow through it —
    /// so the pragmas + read-backs are skipped.
    ReadOnly,
}

/// Open a SQLite connection through the crate's single choke-point.
///
/// For [`Access::ReadWrite`], enables `PRAGMA foreign_keys = ON` and
/// `PRAGMA recursive_triggers = ON` and reads each back, returning
/// [`WalletStorageError::ForeignKeysNotEnforced`] /
/// [`WalletStorageError::RecursiveTriggersNotEnforced`] if a result is
/// not `1`. For [`Access::ReadOnly`], opens with
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
        enforce_recursive_triggers(&conn)?;
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

/// Enable `recursive_triggers` and assert via read-back. Required so the
/// `meta_*` soft-cascade triggers fire for parents removed by an FK
/// cascade, not only directly-deleted parents.
pub(crate) fn enforce_recursive_triggers(conn: &Connection) -> Result<(), WalletStorageError> {
    conn.pragma_update(None, "recursive_triggers", "ON")?;
    let on: i64 = conn.pragma_query_value(None, "recursive_triggers", |r| r.get(0))?;
    if on != 1 {
        return Err(WalletStorageError::RecursiveTriggersNotEnforced);
    }
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

    /// A read-write open enables recursive_triggers and the read-back
    /// confirms it — the assertion path that lets the meta_* soft-cascade
    /// triggers fire through FK cascades.
    #[test]
    fn read_write_open_enforces_and_reads_back_recursive_triggers() {
        let conn = Connection::open_in_memory().expect("in-memory open");
        enforce_recursive_triggers(&conn).expect("recursive_triggers enforcement");
        let on: i64 = conn
            .pragma_query_value(None, "recursive_triggers", |r| r.get(0))
            .expect("read-back");
        assert_eq!(on, 1, "read-back must observe recursive_triggers is on");
    }

    #[test]
    fn recursive_triggers_not_enforced_variant_renders() {
        let err = WalletStorageError::RecursiveTriggersNotEnforced;
        assert!(format!("{err}").contains("recursive-trigger enforcement"));
    }
}
