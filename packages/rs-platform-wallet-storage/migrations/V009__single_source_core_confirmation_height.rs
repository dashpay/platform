//! Single-source UTXO confirmation height in `core_transactions` (#4178).
//!
//! Rebuilding `core_transactions` relaxes `record_blob NOT NULL`. A height-only
//! row carries a recordless UTXO's confirmation height without block context.
//! Legacy writers used height zero as the unconfirmed sentinel, so only positive
//! heights are safe to backfill; post-V009 rows use `NULL` for unconfirmed.

pub fn migration() -> String {
    "\
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

INSERT INTO core_transactions
    (wallet_id, txid, height, block_hash, block_time, finalized, record_blob)
SELECT u.wallet_id, substr(u.outpoint, 2, 32), u.height, NULL, NULL, 0, NULL
FROM core_utxos u
WHERE u.spent = 0 AND u.height IS NOT NULL AND u.height > 0
  AND NOT EXISTS (
      SELECT 1 FROM core_transactions t
      WHERE t.wallet_id = u.wallet_id AND t.txid = substr(u.outpoint, 2, 32)
  )
ON CONFLICT(wallet_id, txid) DO UPDATE SET height = excluded.height
WHERE core_transactions.record_blob IS NULL AND core_transactions.height IS NULL;

ALTER TABLE core_utxos DROP COLUMN height;
"
    .to_string()
}
