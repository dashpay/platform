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

/// Per-wallet asset-lock slice as returned by [`load_state`] —
/// outer-keyed by `account_index`, inner-keyed by outpoint.
pub type AssetLocksByAccount = BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>;

/// Build the per-wallet asset-lock slice for [`ClientStartState`].
/// Wraps [`list_active`] and tracks a corruption-skipped count for
/// the `load()` summary log.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<(AssetLocksByAccount, usize), WalletStorageError> {
    // `list_active` already iterates every row; corruption tolerance
    // sits inside the iteration today (see the per-row decode below).
    // Mirror the (state, skipped) shape so callers can fold this
    // reader into the summary the same way as identities/contacts.
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
    let mut out: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>> = BTreeMap::new();
    let mut skipped = 0usize;
    for r in rows {
        let (op_bytes, account_index, blob_bytes) = match r {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    table = "asset_locks",
                    error = %e,
                    "skipping unreadable asset_locks row"
                );
                skipped += 1;
                continue;
            }
        };
        let outpoint = match blob::decode_outpoint(&op_bytes) {
            Ok(o) => o,
            Err(e) => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    table = "asset_locks",
                    error = %e,
                    "skipping undecodable asset_locks outpoint"
                );
                skipped += 1;
                continue;
            }
        };
        let entry: AssetLockEntry = match blob::decode(&blob_bytes) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    table = "asset_locks",
                    error = %e,
                    "skipping undecodable asset_locks blob"
                );
                skipped += 1;
                continue;
            }
        };
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
        let account_index = match u32::try_from(account_index) {
            Ok(v) => v,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        out.entry(account_index)
            .or_default()
            .insert(outpoint, tracked);
    }
    Ok((out, skipped))
}

/// Return non-`Used` asset locks per wallet, bucketed by account
/// index. Every status variant the changeset writes is considered
/// "active": consumed locks leave via [`AssetLockChangeSet::removed`].
pub fn list_active(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>, WalletStorageError> {
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
    let mut out: BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>> = BTreeMap::new();
    for r in rows {
        let (op_bytes, account_index, blob_bytes) = r?;
        let outpoint = blob::decode_outpoint(&op_bytes)?;
        let entry: AssetLockEntry = blob::decode(&blob_bytes)?;
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
        out.entry(account_index)
            .or_default()
            .insert(outpoint, tracked);
    }
    Ok(out)
}
