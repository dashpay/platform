//! Additive V003 migration: per-row manifest integrity checksum (#3968).
//!
//! Adds a nullable `checksum BLOB` to `account_registrations`. Each row's
//! checksum is `SHA-256(wallet_id ‖ account_xpub_bytes)`, bound by the writer
//! and by a one-time app-side backfill in `SqlitePersister::open`. SQLite has
//! no SHA-256 builtin (and no constant-literal default can carry a per-row
//! digest), so the column is added nullable at DDL time and made effectively
//! non-null at rest by that backfill; verify-at-load treats a surviving NULL
//! on a V003+ store as tamper-evidence.
//!
//! Additive-only: V001/V002 stay byte-identical so refinery's
//! applied-migration checksum chain for versions 1 and 2 never diverges on an
//! existing store. `max_supported_version()` lifts 2 to 3 automatically (the
//! value is derived from the embedded list's `MAX(version)`).

pub fn migration() -> String {
    "ALTER TABLE account_registrations ADD COLUMN checksum BLOB;\n".to_string()
}
