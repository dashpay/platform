//! `dashpay_backfill` writer + reader — the durable record of the DashPay
//! coreHeight backfill, one row per wallet.
//!
//! See [`DashPayBackfillRecord`] for what a record claims. Every write carries
//! the whole record, so the writer replaces rather than folds, and an empty
//! record is an invalidation: it clears whatever is on file. The cover set is
//! stored in the record's own flat encoding
//! ([`covered_bytes`](DashPayBackfillRecord::covered_bytes)).

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::DashPayBackfillRecord;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::util::safe_cast::i64_to_u32;

/// Replace this wallet's backfill record with `record`; an empty record (no
/// covered receival accounts) only deletes the stored one. It is how upstream
/// retracts coverage it can no longer vouch for, and a record with nothing
/// covered claims nothing anyway.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    record: &DashPayBackfillRecord,
) -> Result<(), WalletStorageError> {
    tx.prepare_cached("DELETE FROM dashpay_backfill WHERE wallet_id = ?1")?
        .execute(params![wallet_id.as_slice()])?;
    if record.is_empty() {
        return Ok(());
    }
    tx.prepare_cached(
        "INSERT INTO dashpay_backfill (wallet_id, floor, rewound_from, covered) \
         VALUES (?1, ?2, ?3, ?4)",
    )?
    .execute(params![
        wallet_id.as_slice(),
        i64::from(record.floor),
        i64::from(record.rewound_from),
        record.covered_bytes(),
    ])?;
    Ok(())
}

/// This wallet's backfill record, or the empty record when none is on file.
///
/// Absence reads as "no backfill recorded", which upstream answers by
/// re-running the backfill — slow, never lossy — so a wallet that predates
/// this table is left on exactly the behaviour it had. A cover set that is not
/// a whole number of entries reads the same way, as
/// [`from_parts`](DashPayBackfillRecord::from_parts) prescribes: a contact
/// missing from the set is re-armed, one wrongly present never would be.
///
/// # Errors
///
/// An oversized blob or a height that does not narrow to `u32` is structural
/// and fatal in every load policy; the table's CHECK constraints make either
/// one a sign the file was written with them bypassed.
pub fn load_for_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<DashPayBackfillRecord, WalletStorageError> {
    let mut stmt = conn.prepare_cached(
        "SELECT floor, rewound_from, length(covered), covered \
         FROM dashpay_backfill WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let Some(row) = rows.next()? else {
        return Ok(DashPayBackfillRecord::default());
    };
    let floor = i64_to_u32("dashpay_backfill.floor", row.get(0)?)?;
    let rewound_from = i64_to_u32("dashpay_backfill.rewound_from", row.get(1)?)?;
    // Gate before materializing, like every other blob reader.
    blob::check_size(row.get(2)?)?;
    let covered: Vec<u8> = row.get(3)?;
    Ok(DashPayBackfillRecord::from_parts(floor, rewound_from, &covered).unwrap_or_default())
}
