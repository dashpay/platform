//! Index raw inputs so late output ownership can repair the spending history,
//! and keep each record's pre-repair blob so every repair stays reversible.

pub fn migration() -> String {
    "CREATE TABLE core_transaction_inputs (
        wallet_id BLOB NOT NULL,
        txid BLOB NOT NULL,
        outpoint BLOB NOT NULL,
        PRIMARY KEY (wallet_id, txid, outpoint),
        FOREIGN KEY (wallet_id, txid) REFERENCES core_transactions(wallet_id, txid) ON DELETE CASCADE
    );
    CREATE INDEX idx_core_transaction_inputs_outpoint
        ON core_transaction_inputs(wallet_id, outpoint);
    CREATE TABLE core_transaction_record_originals (
        wallet_id BLOB NOT NULL,
        txid BLOB NOT NULL,
        record_blob BLOB NOT NULL,
        PRIMARY KEY (wallet_id, txid),
        FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
    );"
        .to_owned()
}
