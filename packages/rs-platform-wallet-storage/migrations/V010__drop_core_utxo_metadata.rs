//! Remove `core_utxos` metadata that is not part of production rehydration.

pub fn migration() -> String {
    "\
DROP TRIGGER setnull_core_utxos_on_tx_delete;

ALTER TABLE core_utxos DROP COLUMN account_index;
ALTER TABLE core_utxos DROP COLUMN spent_in_txid;
"
    .to_string()
}
