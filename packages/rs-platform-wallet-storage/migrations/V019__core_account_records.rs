//! Preserve per-account transaction slices alongside the wallet-level record.

pub fn migration() -> String {
    "ALTER TABLE core_transactions ADD COLUMN account_records_blob BLOB;".to_string()
}
