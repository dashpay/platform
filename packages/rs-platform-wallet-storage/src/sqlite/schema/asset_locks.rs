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

use crate::sqlite::error::SqlitePersisterError;
use crate::sqlite::schema::blob;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &AssetLockChangeSet,
) -> Result<(), SqlitePersisterError> {
    for (op, entry) in &cs.asset_locks {
        let op_bytes = blob::encode_outpoint(op);
        let lifecycle_blob = blob::encode(entry)?;
        tx.execute(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
                status = excluded.status, \
                account_index = excluded.account_index, \
                identity_index = excluded.identity_index, \
                amount_duffs = excluded.amount_duffs, \
                lifecycle_blob = excluded.lifecycle_blob",
            params![
                wallet_id.as_slice(),
                &op_bytes[..],
                status_str(&entry.status),
                entry.account_index as i64,
                entry.identity_index as i64,
                entry.amount_duffs as i64,
                lifecycle_blob,
            ],
        )?;
    }
    for op in &cs.removed {
        let op_bytes = blob::encode_outpoint(op);
        tx.execute(
            "DELETE FROM asset_locks WHERE wallet_id = ?1 AND outpoint = ?2",
            params![wallet_id.as_slice(), &op_bytes[..]],
        )?;
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

/// Return non-`Used` asset locks per wallet, bucketed by account
/// index. Every status variant the changeset writes is considered
/// "active": consumed locks leave via [`AssetLockChangeSet::removed`].
pub fn list_active(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>, SqlitePersisterError> {
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
        out.entry(account_index as u32)
            .or_default()
            .insert(outpoint, tracked);
    }
    Ok(out)
}
