//! Index raw inputs so late output ownership can repair the spending history,
//! keep each record's pre-repair blob so every repair stays reversible, and
//! rebuild the Core history stored before both by rescanning it.
//!
//! Records stored before V019 may carry wrong input details, net amounts and
//! directions, and their raw inputs were never indexed. Rather than repair
//! them in place, V019 drops each wallet's Core history (transactions, UTXOs,
//! instant locks) and rewinds its filter checkpoint to just below its birth
//! height, so the next SPV start rescans it from birth and records it again
//! through the current accounting. The SPV header/filter store, the address
//! pools, asset locks, identities, DashPay and metadata rows are untouched;
//! `last_processed_height` and the chainlock are chain facts, not coverage,
//! and stay. Unconfirmed transactions are not re-delivered by the rescan and
//! return only if the network relays them again.

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
    DELETE FROM core_utxos;
    DELETE FROM core_instant_locks;
    DELETE FROM core_transactions;
    UPDATE core_sync_state SET synced_height = (
        SELECT MIN(COALESCE(core_sync_state.synced_height, MAX(w.birth_height - 1, 0)),
                   MAX(w.birth_height - 1, 0))
        FROM wallets w WHERE w.wallet_id = core_sync_state.wallet_id
    );"
    .to_owned()
}
