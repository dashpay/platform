//! Index raw inputs so late output ownership can repair the spending history.

pub fn migration() -> String {
    "CREATE TABLE core_transaction_inputs (
        wallet_id BLOB NOT NULL,
        txid BLOB NOT NULL,
        outpoint BLOB NOT NULL,
        PRIMARY KEY (wallet_id, txid, outpoint),
        FOREIGN KEY (wallet_id, txid) REFERENCES core_transactions(wallet_id, txid) ON DELETE CASCADE
    );
    CREATE INDEX idx_core_transaction_inputs_outpoint
        ON core_transaction_inputs(wallet_id, outpoint);"
        .to_owned()
}
