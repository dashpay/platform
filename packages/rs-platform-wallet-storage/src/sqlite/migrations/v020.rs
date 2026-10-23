//! Typed half of V020: index every input of each kept unmined record.
//!
//! SQL cannot decode a record body, so the SQL half can only name the coins a
//! kept record already holds the claim on. A sweep that withholds an input
//! hands it to a surviving spender found through this index; a survivor
//! missing from it loses the claim to the winner, and the coin stays held
//! after the survivor is defeated. Every input is therefore indexed here.
//!
//! CODEC PIN: this step decodes `TransactionRecord` bodies with the CURRENT
//! record codec (`core_state::decode_transaction_record`), and V020 is the
//! last migration allowed to depend on it. A later change to that codec must
//! keep every body V020 can meet decodable: one it cannot read is skipped,
//! silently leaving its inputs unindexed. The golden rows in
//! `tests/integration/persisted_core_transactions.rs` guard that decoding
//! across dependency upgrades. A later migration that needs record contents
//! must freeze its own decoder instead of calling the live one.

use dashcore::hashes::Hash;
use rusqlite::Transaction;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{blob, core_history, core_state, id32};

/// Index the complete input set of every kept record body.
///
/// A body this cannot read, or one stored under another txid, is skipped
/// rather than failing the upgrade: the loader judges it under its load
/// policy, and without a readable body there are no inputs to index.
pub(super) fn index_kept_inputs(tx: &Transaction<'_>) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare(
        "SELECT wallet_id, txid, length(record_blob), record_blob FROM core_transactions
         WHERE height IS NULL AND record_blob IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let wallet_id = id32("core_transactions.wallet_id", &row.get::<_, Vec<u8>>(0)?)?;
        let txid: Vec<u8> = row.get(1)?;
        if blob::check_size(row.get(2)?).is_err() {
            continue;
        }
        let Ok(record) = core_state::decode_transaction_record(&row.get::<_, Vec<u8>>(3)?) else {
            continue;
        };
        if record.txid.as_byte_array().as_slice() != txid.as_slice() {
            continue;
        }
        core_history::index_record(tx, &wallet_id, &record)?;
    }
    Ok(())
}
