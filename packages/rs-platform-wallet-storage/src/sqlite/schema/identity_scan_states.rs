//! `identity_scan_states` writer + reader — the verdict of the last
//! gap-limit identity scan, one row per wallet.
//!
//! The entry is all-primitive apart from its list of unanswered indices, so
//! every field maps to an explicit column and the list lives in the
//! `identity_scan_failed_indices` child table. See
//! [`IdentityScanStateEntry`] for what each field means and why `complete`
//! and `unlocated_gap` are stored rather than derived.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use platform_wallet::changeset::IdentityScanStateEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::wallet_id_to_param;
use crate::sqlite::util::safe_cast::i64_to_u32;

/// Persist `incoming` as this wallet's scan verdict, folded over whatever is
/// already on record.
///
/// Folding rather than overwriting is what keeps the durable record from
/// losing a gap. In-process the manager has already folded — `superseding` is
/// idempotent against the same previous verdict, so that costs nothing — but
/// one database file may be open to several processes, and a peer holding a
/// staler view would otherwise publish a clean suffix scan over a gap it
/// never probed. That is dashpay/platform#4365 reached from the other side,
/// and it is the one direction this row must never move in.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    incoming: &IdentityScanStateEntry,
) -> Result<(), WalletStorageError> {
    let folded = match read(tx, wallet_id)? {
        Some(previous) => incoming.clone().superseding(&previous),
        None => incoming.clone(),
    };

    // Named so a field added to `IdentityScanStateEntry` is a compile error
    // here rather than a column that silently stops being written.
    let IdentityScanStateEntry {
        complete,
        probed_from,
        probed_through,
        ref failed_indices,
        unlocated_gap,
    } = folded;

    tx.prepare_cached(
        "INSERT INTO identity_scan_states \
            (wallet_id, complete, probed_from, probed_through, unlocated_gap) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            complete = excluded.complete, \
            probed_from = excluded.probed_from, \
            probed_through = excluded.probed_through, \
            unlocated_gap = excluded.unlocated_gap",
    )?
    .execute(params![
        wallet_id.as_slice(),
        i64::from(complete),
        i64::from(probed_from),
        i64::from(probed_through),
        i64::from(unlocated_gap),
    ])?;

    // The list is replaced wholesale: `folded` already carries every index
    // still outstanding, so a survivor of the old row that is missing here
    // has been answered.
    tx.prepare_cached("DELETE FROM identity_scan_failed_indices WHERE wallet_id = ?1")?
        .execute(params![wallet_id.as_slice()])?;
    let mut insert = tx.prepare_cached(
        "INSERT INTO identity_scan_failed_indices (wallet_id, failed_index) VALUES (?1, ?2)",
    )?;
    for index in failed_indices {
        insert.execute(params![wallet_id.as_slice(), i64::from(*index)])?;
    }
    Ok(())
}

/// This wallet's scan verdict, or `None` when no scan has ever published one.
///
/// Absence is deliberately not completeness: upstream reads a missing entry
/// as "keep the existing warm-launch behaviour", so a wallet that predates
/// this bookkeeping is left exactly where it was.
///
/// # Errors
///
/// Returns [`WalletStorageError::IdentityScanStateContradiction`] under
/// [`LoadPolicy::Strict`](crate::sqlite::config::LoadPolicy::Strict) when the
/// row claims a complete scan while unanswered indices sit beside it — a
/// state no fold can produce. Under
/// [`LoadPolicy::Recovery`](crate::sqlite::config::LoadPolicy::Recovery) the
/// contradiction is counted and the verdict clamped to incomplete: the cost
/// of being wrong that way is one extra scan, and the cost of being wrong the
/// other way is an identity that never reappears.
pub fn load_for_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<Option<IdentityScanStateEntry>, WalletStorageError> {
    let Some(mut entry) = read(conn, wallet_id)? else {
        return Ok(None);
    };
    if entry.complete && !entry.failed_indices.is_empty() {
        ctx.tolerate(
            LoadSite::IdentityScanStateContradiction,
            WalletStorageError::IdentityScanStateContradiction {
                wallet_id: *wallet_id,
                failed_indices: entry.failed_indices.len(),
            },
        )?;
        entry.complete = false;
    }
    Ok(Some(entry))
}

/// Read the row and its indices verbatim, without judging them. Shared by the
/// write-side fold (which must see exactly what is stored) and the read path
/// (which validates on top).
///
/// Takes `&Connection` so a `Transaction`'s deref covers the writer too.
fn read(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Option<IdentityScanStateEntry>, WalletStorageError> {
    // NULL-safe `IS`, matching the identity readers: the all-zero unowned
    // sentinel maps to NULL, which a NOT NULL primary key can never hold, so
    // the unowned scope correctly finds nothing instead of matching by luck.
    let wallet_id_param = wallet_id_to_param(wallet_id);
    let row = conn
        .prepare_cached(
            "SELECT complete, probed_from, probed_through, unlocated_gap \
             FROM identity_scan_states WHERE wallet_id IS ?1",
        )?
        .query_row(params![wallet_id_param], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .optional()?;
    let Some((complete, probed_from, probed_through, unlocated_gap)) = row else {
        return Ok(None);
    };

    let mut stmt = conn.prepare_cached(
        "SELECT failed_index FROM identity_scan_failed_indices \
         WHERE wallet_id IS ?1 ORDER BY failed_index",
    )?;
    let mut rows = stmt.query(params![wallet_id_param])?;
    let mut failed_indices = Vec::new();
    while let Some(row) = rows.next()? {
        failed_indices.push(i64_to_u32(
            "identity_scan_failed_indices.failed_index",
            row.get(0)?,
        )?);
    }

    Ok(Some(IdentityScanStateEntry {
        complete: complete != 0,
        probed_from: i64_to_u32("identity_scan_states.probed_from", probed_from)?,
        probed_through: i64_to_u32("identity_scan_states.probed_through", probed_through)?,
        failed_indices,
        unlocated_gap: unlocated_gap != 0,
    }))
}
