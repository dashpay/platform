//! Keep UTXO confirmation height single-sourced in `core_transactions` (#4178).

pub fn migration() -> String {
    "ALTER TABLE core_utxos DROP COLUMN height;".to_string()
}
