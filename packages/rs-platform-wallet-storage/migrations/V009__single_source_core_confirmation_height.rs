//! Single-source UTXO confirmation height in `core_transactions` (#4178).

pub fn migration() -> String {
    "\
DROP INDEX idx_core_transactions_height;

CREATE TABLE core_transactions_new (
    wallet_id BLOB NOT NULL,
    txid BLOB NOT NULL,
    height INTEGER,
    block_hash BLOB,
    block_time INTEGER,
    finalized INTEGER NOT NULL,
    record_blob BLOB,
    PRIMARY KEY (wallet_id, txid),
    FOREIGN KEY (wallet_id) REFERENCES wallets(wallet_id) ON DELETE CASCADE
);

INSERT INTO core_transactions_new
    (wallet_id, txid, height, block_hash, block_time, finalized, record_blob)
SELECT wallet_id, txid, height, block_hash, block_time, finalized, record_blob
FROM core_transactions;

DROP TABLE core_transactions;
ALTER TABLE core_transactions_new RENAME TO core_transactions;

CREATE INDEX idx_core_transactions_height ON core_transactions(wallet_id, height);

ALTER TABLE core_utxos DROP COLUMN height;
"
    .to_string()
}
