//! Typed half of V020: index every input of each kept unmined record.
//!
//! SQL cannot decode a record body, so the SQL half can only name the coins a
//! kept record already holds the claim on. A sweep that withholds an input
//! hands it to a surviving spender found through this index; a survivor
//! missing from it loses the claim to the winner, and the coin stays held
//! after the survivor is defeated. Every input is therefore indexed here.
//!
//! Record codec: this step decodes `TransactionRecord` bodies with the live
//! record codec (`core_state::decode_transaction_record`), not a frozen copy,
//! so the live codec must keep decoding every stored body. Two checks catch a
//! break: the golden rows in `tests/integration/persisted_core_transactions.rs`
//! fail in CI when a codec or dependency change stops decoding them, and under
//! [`LoadPolicy::Strict`] this step fails the upgrade on a body it cannot read
//! instead of silently leaving its inputs unindexed. Migrations here and later
//! may therefore call the live codec; a change that trips either check has to
//! restore compatibility.

use dashcore::hashes::Hash;
use rusqlite::Transaction;

use crate::sqlite::config::LoadPolicy;
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::{blob, core_history, core_state, id32};

/// Index the complete input set of every kept record body.
///
/// A body this cannot read (oversize, undecodable, or stored under another
/// txid) follows `policy`: [`LoadPolicy::Strict`] fails the upgrade, which
/// rolls back to V019, so a codec regression surfaces at once instead of as
/// coins stuck later. [`LoadPolicy::Recovery`] logs and skips it; its wallet
/// then degrades at load, where the same body fails to decode again.
pub(super) fn index_kept_inputs(
    tx: &Transaction<'_>,
    policy: LoadPolicy,
) -> Result<(), WalletStorageError> {
    let mut stmt = tx.prepare(
        "SELECT wallet_id, txid, length(record_blob), record_blob FROM core_transactions
         WHERE height IS NULL AND record_blob IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let wallet_id = id32("core_transactions.wallet_id", &row.get::<_, Vec<u8>>(0)?)?;
        let txid: Vec<u8> = row.get(1)?;
        let record = blob::check_size(row.get(2)?)
            .and_then(|()| core_state::decode_transaction_record(&row.get::<_, Vec<u8>>(3)?))
            .and_then(|record| {
                if record.txid.as_byte_array().as_slice() == txid.as_slice() {
                    Ok(record)
                } else {
                    Err(WalletStorageError::blob_decode(
                        "kept transaction record is stored under another txid",
                    ))
                }
            });
        match (record, policy) {
            (Ok(record), _) => core_history::index_record(tx, &wallet_id, &record)?,
            (Err(error), LoadPolicy::Strict) => return Err(error),
            (Err(error), LoadPolicy::Recovery) => tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                txid = %hex::encode(&txid),
                %error,
                "V020 skipped a kept transaction record it cannot read; its inputs stay unindexed"
            ),
        }
    }
    Ok(())
}
