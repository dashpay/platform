//! `asset_locks` table writer + reader.
//!
//! Each row stores the lifecycle status as a string column for direct
//! SQL queries, plus a bincode-serde encoded `AssetLockEntry` in the
//! `lifecycle_blob` column.

use std::collections::BTreeMap;

use dashcore::OutPoint;
use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::{AssetLockChangeSet, AssetLockEntry};
use platform_wallet::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &AssetLockChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.asset_locks.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
                status = excluded.status, \
                account_index = excluded.account_index, \
                identity_index = excluded.identity_index, \
                amount_duffs = excluded.amount_duffs, \
                lifecycle_blob = excluded.lifecycle_blob",
        )?;
        for (op, entry) in &cs.asset_locks {
            let op_bytes = blob::encode_outpoint(op);
            let lifecycle_blob = blob::encode(entry)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                &op_bytes[..],
                status_str(&entry.status),
                i64::from(entry.account_index),
                i64::from(entry.identity_index),
                crate::sqlite::util::safe_cast::u64_to_i64(
                    "asset_locks.amount_duffs",
                    entry.amount_duffs,
                )?,
                lifecycle_blob,
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        let mut stmt =
            tx.prepare_cached("DELETE FROM asset_locks WHERE wallet_id = ?1 AND outpoint = ?2")?;
        for op in &cs.removed {
            let op_bytes = blob::encode_outpoint(op);
            stmt.execute(params![wallet_id.as_slice(), &op_bytes[..]])?;
        }
    }
    Ok(())
}

fn status_str(s: &AssetLockStatus) -> &'static str {
    match s {
        AssetLockStatus::Built => "built",
        AssetLockStatus::Broadcast => "broadcast",
        AssetLockStatus::InstantSendLocked => "is_locked",
        AssetLockStatus::ChainLocked => "chain_locked",
        AssetLockStatus::Consumed => "consumed",
    }
}

/// Per-wallet asset-lock slice as returned by the readers — outer-keyed
/// by `account_index`, inner-keyed by outpoint.
pub type AssetLocksByAccount = BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>;

/// Decode one raw `(outpoint_bytes, account_index, lifecycle_blob)`
/// tuple into the typed `(account_index, OutPoint, TrackedAssetLock)`
/// triple that [`load_state`] consumes.
///
/// Hard-fail behaviour: a malformed outpoint, blob, or out-of-range
/// account index returns a typed [`WalletStorageError`]. Every caller
/// propagates that error — corruption is never silently skipped.
fn decode_row(
    op_bytes: &[u8],
    account_index: i64,
    blob_bytes: &[u8],
) -> Result<(u32, OutPoint, TrackedAssetLock), WalletStorageError> {
    let outpoint = blob::decode_outpoint(op_bytes)?;
    let entry: AssetLockEntry = blob::decode(blob_bytes)?;
    let account_index =
        u32::try_from(account_index).map_err(|_| WalletStorageError::IntegerOverflow {
            field: "asset_locks.account_index",
            value: account_index as u64,
            target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
        })?;
    // CMT-007: typed-column vs blob cross-check, symmetric with
    // IdentityKeyEntryMismatch. A torn write / partial migration /
    // restored corruption that passes PRAGMA integrity_check would
    // otherwise silently mis-bucket the lock into the wrong account or
    // report a different outpoint than the indexed column it was
    // selected by.
    if entry.out_point != outpoint || entry.account_index != account_index {
        return Err(WalletStorageError::AssetLockEntryMismatch {
            typed_outpoint: outpoint.to_string(),
            blob_outpoint: entry.out_point.to_string(),
            typed_account_index: account_index,
            blob_account_index: entry.account_index,
        });
    }
    let tracked = TrackedAssetLock {
        out_point: entry.out_point,
        transaction: entry.transaction,
        account_index: entry.account_index,
        funding_type: entry.funding_type,
        identity_index: entry.identity_index,
        amount: entry.amount_duffs,
        status: entry.status,
        proof: entry.proof,
    };
    Ok((account_index, outpoint, tracked))
}

/// Build the per-wallet asset-lock slice for `ClientStartState` from
/// the `asset_locks` table, bucketed by account index. Every status
/// variant the changeset writes is considered "active": consumed
/// locks leave the table via [`AssetLockChangeSet::removed`], so a
/// row present here is by definition still in play. Any row that
/// fails to read or decode is a hard error — corruption is never
/// silently dropped.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, account_index, lifecycle_blob \
         FROM asset_locks WHERE wallet_id = ?1",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let account_index: i64 = row.get(1)?;
        let blob_bytes: Vec<u8> = row.get(2)?;
        Ok((op_bytes, account_index, blob_bytes))
    })?;
    let mut out: AssetLocksByAccount = BTreeMap::new();
    for r in rows {
        let (op_bytes, account_index, blob_bytes) = r?;
        let (acct, outpoint, tracked) = decode_row(&op_bytes, account_index, &blob_bytes)?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}

/// The status-filtered rehydration feed: every asset lock for the
/// wallet **except** terminal `Consumed` rows, bucketed by account
/// index.
///
/// `consume_asset_lock` upserts a row with `status = 'consumed'` and
/// never `DELETE`s it (post-#3634 the row persists forever for
/// history). Feeding `Consumed` rows back into `unused_asset_locks`
/// would resurrect a spent one-shot lock as actionable (A04/A08), so
/// rehydration must read through this filter. The exclusion is at the
/// SQL level (`status NOT IN ('consumed')`, `status` indexed — no
/// full-scan regression); the historical rows stay on disk and remain
/// visible via [`load_state`].
///
/// Hard-fail on the first decode error — like [`load_state`], a
/// corrupt row aborts the read with a typed [`WalletStorageError`].
pub fn load_unconsumed(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, account_index, lifecycle_blob \
         FROM asset_locks WHERE wallet_id = ?1 AND status NOT IN ('consumed')",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let account_index: i64 = row.get(1)?;
        let blob_bytes: Vec<u8> = row.get(2)?;
        Ok((op_bytes, account_index, blob_bytes))
    })?;
    let mut out: AssetLocksByAccount = BTreeMap::new();
    for r in rows {
        let (op_bytes, account_index, blob_bytes) = r?;
        let (acct, outpoint, tracked) = decode_row(&op_bytes, account_index, &blob_bytes)?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}

/// Return every asset lock for the wallet, bucketed by account index,
/// **including** terminal `Consumed`.
///
/// Use [`load_unconsumed`] for the rehydration feed — this unfiltered
/// view is for history / inspection only. (A `Consumed` lock survives
/// consumption permanently on disk: `consume_asset_lock` upserts
/// `status = 'consumed'` and does *not* route the entry through
/// `AssetLockChangeSet::removed`, so it is never `DELETE`d.)
///
/// Hard-fail on the first decode error — like [`load_state`], a
/// corrupt row aborts the read with a typed [`WalletStorageError`].
pub fn list_active(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, account_index, lifecycle_blob \
         FROM asset_locks WHERE wallet_id = ?1",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let account_index: i64 = row.get(1)?;
        let blob_bytes: Vec<u8> = row.get(2)?;
        Ok((op_bytes, account_index, blob_bytes))
    })?;
    let mut out: AssetLocksByAccount = BTreeMap::new();
    for r in rows {
        let (op_bytes, account_index, blob_bytes) = r?;
        let (acct, outpoint, tracked) = decode_row(&op_bytes, account_index, &blob_bytes)?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}
