//! Exact signed funding transitions live outside the frozen lifecycle codec.
pub fn migration() -> String {
    "ALTER TABLE asset_locks ADD COLUMN shielded_funding_attempts_blob BLOB \
     CHECK (shielded_funding_attempts_blob IS NULL OR typeof(shielded_funding_attempts_blob) = 'blob');".to_owned()
}
