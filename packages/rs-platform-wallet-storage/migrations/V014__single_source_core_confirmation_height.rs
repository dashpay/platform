//! Single-source UTXO confirmation height in `core_transactions` (#4178).
//!
//! Rebuilding `core_transactions` relaxes `record_blob NOT NULL`. A height-only
//! row carries a recordless UTXO's confirmation height without block context.
//! Legacy writers used height zero as the unconfirmed sentinel, so only positive
//! heights are safe to backfill; post-migration rows use `NULL` for unconfirmed.

// INTENTIONAL(outpoint-txid-prefix): `substr(u.outpoint, 2, 32)` lifts the txid
// out of an encoded outpoint by skipping a single length-prefix byte ahead of
// the 32 txid bytes. `encode_outpoint_txid_occupies_bytes_two_to_thirty_three`
// pins that layout, so a change in the encoding fails a test here rather than
// silently backfilling 32 bytes of the wrong field.

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

-- Orphan policy: a row whose wallet was deleted while FK enforcement happened
-- to be off is unreachable garbage (every read path keys through `wallets`),
-- but copying it into the FK-declared twin under PRAGMA foreign_keys = ON
-- aborts this whole migration with 'FOREIGN KEY constraint failed'. Drop such
-- rows explicitly -- the same outcome the declared ON DELETE CASCADE would
-- have produced had enforcement been on when the wallet was deleted. Both
-- source tables need it: `core_utxos` feeds the height-only backfill below.
DELETE FROM core_transactions WHERE wallet_id NOT IN (SELECT wallet_id FROM wallets);
DELETE FROM core_utxos WHERE wallet_id NOT IN (SELECT wallet_id FROM wallets);

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

DROP INDEX idx_core_utxos_unmaterialized;
ALTER TABLE core_utxos ADD COLUMN is_sweep_placeholder INTEGER NOT NULL DEFAULT 0;
UPDATE core_utxos SET is_sweep_placeholder = 1 WHERE height IS NULL;
ALTER TABLE core_utxos DROP COLUMN height;
CREATE INDEX idx_core_utxos_unmaterialized ON core_utxos(wallet_id, winner_mined_height)
    WHERE is_sweep_placeholder = 1;
CREATE TRIGGER setnull_core_utxos_on_tx_delete AFTER DELETE ON core_transactions
BEGIN
    UPDATE core_utxos SET spent_in_txid = NULL
    WHERE wallet_id = OLD.wallet_id AND spent_in_txid = OLD.txid;
END;
"
    .to_string()
}
