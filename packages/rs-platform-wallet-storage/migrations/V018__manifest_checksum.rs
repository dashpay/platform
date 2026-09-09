//! Bind account manifest blobs to their wallet with SHA-256 checksums.
//! The migration transaction backfills existing rows before committing.

pub fn migration() -> String {
    "ALTER TABLE account_registrations ADD COLUMN checksum BLOB;\n".to_string()
}
