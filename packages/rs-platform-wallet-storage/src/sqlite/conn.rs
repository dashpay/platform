//! Single connection-open choke-point.
//!
//! `PRAGMA foreign_keys` is per-connection, defaults to OFF on every open,
//! and silently no-ops on a SQLite built without FK support — so every
//! writer connection must enable it and read it back to prove it took.
//! All library opens route through [`open_conn`]; the CLI's read-only
//! `peek_schema_version` probe opens directly (no mutations, and
//! `open_conn` is `pub(crate)`, unreachable from the bin target).

use rusqlite::{Connection, OpenFlags};
use std::path::Path;

use crate::sqlite::error::WalletStorageError;

/// Magic stamped into the SQLite header `application_id` (offset 68) by
/// `V001__initial`. ASCII `"PLWT"` (Platform Wallet) big-endian. A
/// refinery-versioned DB whose `application_id` does not equal this is a
/// foreign SQLite database, not a wallet-storage DB.
pub(crate) const APPLICATION_ID: i32 = 0x504C_5754;

/// Read the header `application_id` and assert it equals
/// [`APPLICATION_ID`]. Returns [`WalletStorageError::NotAWalletDb`] on
/// mismatch. The caller decides WHEN to run this — `open()` runs it
/// pre-migration on a refinery-versioned DB; `restore_from` runs it on
/// the staged copy. A brand-new (unmigrated) DB reports `0` and is the
/// caller's responsibility to skip (V001 stamps the real value).
pub(crate) fn assert_wallet_application_id(conn: &Connection) -> Result<(), WalletStorageError> {
    let found: i32 = conn.pragma_query_value(None, "application_id", |row| row.get(0))?;
    if found != APPLICATION_ID {
        return Err(WalletStorageError::NotAWalletDb {
            expected: APPLICATION_ID,
            found,
        });
    }
    Ok(())
}

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
/// `SQLITE_OPEN_READ_ONLY` and performs no pragma. URI filename parsing is
/// deliberately left off so a path can't smuggle query parameters (e.g.
/// `?mode=rwc`) that defeat the read-only intent.
pub(crate) fn open_conn(path: &Path, access: Access) -> Result<Connection, WalletStorageError> {
    let conn = match access {
        Access::ReadWrite => Connection::open(path)?,
        Access::ReadOnly => Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?,
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

    /// The bundled build can't produce a FK-less SQLite, so assert the
    /// read-back error variant at least renders its intended message.
    #[test]
    fn foreign_keys_not_enforced_variant_renders() {
        let err = WalletStorageError::ForeignKeysNotEnforced;
        assert!(format!("{err}").contains("foreign-key enforcement"));
    }
}
