//! Index raw inputs so late output ownership can repair the spending history,
//! keep each record's pre-repair blob so every repair stays reversible, and
//! rebuild the mined Core history stored before both by rescanning it.
//!
//! Records stored before V020 may carry wrong input details, net amounts and
//! directions, and their raw inputs were never indexed. Rather than repair
//! them in place, V020 drops each wallet's mined Core history and rewinds its
//! filter checkpoint to just below its birth height, so the next SPV start
//! rescans it from birth and records it again through the current accounting.
//!
//! A block rescan cannot redeliver what was never mined, so V020 keeps the
//! unmined history as evidence: unconfirmed transactions with their instant
//! locks and outputs, and every spent row whose claim no mined transaction
//! stands behind. A kept coin's mined funding is reduced to a height-only row
//! the rescan records again. Every input of each kept record is indexed, not
//! only the coins it holds the claim on, so a sweep can hand a withheld input
//! to a kept survivor; SQL cannot read a record body, so the typed half in
//! `src/sqlite/migrations/v020.rs` does that after this SQL. The loader then
//! authenticates every kept claim before replay, as it does any stored one.
//!
//! Kept records carry their possibly wrong stored accounting until the wallet
//! records them again, at the latest when they mine. SQL cannot recompute it,
//! and a pending record that shows a wrong amount for a while costs less than
//! a lost reservation, which lets the wallet spend the coin again.
//!
//! The SPV header/filter store, the address pools, asset locks, identities,
//! DashPay and metadata rows are untouched; `last_processed_height` and the
//! chainlock are chain facts, not coverage, and stay.

// INTENTIONAL(outpoint-txid-prefix): `substr(outpoint, 2, 32)` is the txid of
// an encoded outpoint; `encode_outpoint_txid_occupies_bytes_two_to_thirty_three`
// pins that layout.

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
    );
    DELETE FROM core_utxos WHERE NOT (
        EXISTS (
            SELECT 1 FROM core_transactions t
            WHERE t.wallet_id = core_utxos.wallet_id
              AND t.txid = substr(core_utxos.outpoint, 2, 32) AND t.height IS NULL
        )
        OR (
            spent = 1 AND winner_mined_height IS NULL AND NOT EXISTS (
                SELECT 1 FROM core_transactions t
                WHERE t.wallet_id = core_utxos.wallet_id
                  AND t.txid = core_utxos.spent_in_txid AND t.height IS NOT NULL
            )
        )
    );
    DELETE FROM core_instant_locks WHERE NOT EXISTS (
        SELECT 1 FROM core_transactions t
        WHERE t.wallet_id = core_instant_locks.wallet_id
          AND t.txid = core_instant_locks.txid AND t.height IS NULL
    );
    DELETE FROM core_transactions WHERE height IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM core_utxos u
        WHERE u.wallet_id = core_transactions.wallet_id
          AND substr(u.outpoint, 2, 32) = core_transactions.txid
    );
    UPDATE core_transactions
        SET block_hash = NULL, block_time = NULL, finalized = 0, record_blob = NULL
        WHERE height IS NOT NULL;
    UPDATE core_sync_state SET synced_height = (
        SELECT MIN(COALESCE(core_sync_state.synced_height, MAX(w.birth_height - 1, 0)),
                   MAX(w.birth_height - 1, 0))
        FROM wallets w WHERE w.wallet_id = core_sync_state.wallet_id
    );"
    .to_owned()
}
