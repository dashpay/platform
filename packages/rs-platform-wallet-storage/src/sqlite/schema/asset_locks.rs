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
    }
}

/// Per-wallet asset-lock slice as returned by the readers — outer-keyed
/// by `account_index`, inner-keyed by outpoint.
pub type AssetLocksByAccount = BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>;

/// Decode one raw `(outpoint_bytes, account_index, lifecycle_blob)`
/// tuple into the typed `(account_index, OutPoint, TrackedAssetLock)`
/// triple that [`list_active`], [`load_state`], and [`load_all`]
/// consume.
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
    let account_index =
        u32::try_from(account_index).map_err(|_| WalletStorageError::IntegerOverflow {
            field: "asset_locks.account_index",
            value: account_index as u64,
            target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
        })?;
    Ok((account_index, outpoint, tracked))
}

/// Build the per-wallet asset-lock slice for `ClientStartState` from
/// the `asset_locks` table. Any row that fails to read or decode is a
/// hard error — corruption is never silently dropped.
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

/// Bulk reader for `load()`: one [`load_state`] call per wallet id
/// listed in `wallet_metadata`. Constant-query w.r.t. the number of
/// wallets touched per call site (FR-P4-6).
///
/// Driven by [`wallet_meta::list_ids`](crate::sqlite::schema::wallet_meta::list_ids):
/// orphaned `asset_locks` rows whose `wallet_id` is absent from
/// `wallet_metadata` are intentionally NOT surfaced. FK triggers
/// prevent such orphans; a future re-wire that needs them must restore
/// the id-union over the area table.
pub fn load_all(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, AssetLocksByAccount>, WalletStorageError> {
    let mut out: BTreeMap<WalletId, AssetLocksByAccount> = BTreeMap::new();
    for wallet_id in crate::sqlite::schema::wallet_meta::list_ids(conn)? {
        out.insert(wallet_id, load_state(conn, &wallet_id)?);
    }
    Ok(out)
}

/// Return non-`Used` asset locks per wallet, bucketed by account
/// index. Every status variant the changeset writes is considered
/// "active": consumed locks leave via [`AssetLockChangeSet::removed`].
///
/// Hard-fail on the first decode error — like [`load_state`] and
/// [`load_all`], a corrupt row aborts the read with a typed
/// [`WalletStorageError`].
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
