//! Add `core_utxos.rescan_held_until`: the sync checkpoint a coin a re-scan
//! credits at or below it is held out of the balance to (see SCHEMA.md).
//!
//! Purely additive: `NULL` on every existing row, which holds nothing.

pub fn migration() -> String {
    "ALTER TABLE core_utxos ADD COLUMN rescan_held_until INTEGER;".to_string()
}
