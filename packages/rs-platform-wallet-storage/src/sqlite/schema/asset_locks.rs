//! `asset_locks` table writer + reader.
//!
//! Each row stores the lifecycle status as a string column for direct
//! SQL queries, plus a bincode-serde encoded `AssetLockEntry` in the
//! `lifecycle_blob` column.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::AssetLockChangeSet;
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

// Imports used only by the test-gated readers below.
#[cfg(any(test, feature = "__test-helpers"))]
use {
    dashcore::OutPoint, platform_wallet::changeset::AssetLockEntry,
    platform_wallet::wallet::asset_lock::tracked::TrackedAssetLock, rusqlite::Connection,
    std::collections::BTreeMap,
};

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &AssetLockChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.asset_locks.is_empty() {
        // The upsert's WHERE clause enforces the one terminal lifecycle
        // rule: a stored `consumed` row is never overwritten by a
        // non-consumed snapshot. Racing writers persist through
        // different paths (the wallet-event adapter's batched drain vs
        // the live flows' synchronous changeset queue), so a stale
        // reconstruction/enrichment snapshot can land AFTER the
        // consumption write — this guard makes that arrival order
        // immaterial. Every other transition is deliberately
        // last-write-wins: non-terminal statuses move both ways (live
        // advances overwrite `recovered_from_chain`, defensive resumes
        // re-enter `broadcast`), so terminality is the only ordering
        // the store can enforce without vetoing legitimate writes.
        // `AssetLockChangeSet::merge` applies the same rule when
        // batches fold before reaching the store.
        let mut stmt = tx.prepare_cached(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
                status = excluded.status, \
                account_index = excluded.account_index, \
                identity_index = excluded.identity_index, \
                amount_duffs = excluded.amount_duffs, \
                lifecycle_blob = excluded.lifecycle_blob \
             WHERE asset_locks.status != 'consumed' OR excluded.status = 'consumed'",
        )?;
        for (op, entry) in &cs.asset_locks {
            let op_bytes = blob::encode_outpoint(op)?;
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
            let op_bytes = blob::encode_outpoint(op)?;
            stmt.execute(params![wallet_id.as_slice(), &op_bytes[..]])?;
        }
    }
    Ok(())
}

/// Test-only drift guard for the `asset_locks.status` TEXT-column
/// domain **as the writer sees it** (production code never reads this
/// — the writer maps through [`status_str`] and the on-disk CHECK
/// lives frozen inside the migrations).
///
/// Mirrors every variant of
/// [`platform_wallet::wallet::asset_lock::tracked::AssetLockStatus`]
/// (writer side: [`status_str`]). The on-disk `CHECK (status IN (...))`
/// clause rejects an unknown label at insert time rather than letting
/// it land as silent garbage — but the migrations do NOT interpolate
/// this const: each migration freezes its own copy of the domain,
/// because a generated-SQL change breaks that migration's Refinery
/// checksum on every database that already applied it
/// (`abort_divergent` default). `V001__initial.rs` carries the original
/// five labels; `V004__asset_lock_recovered_status.rs` rebuilt the
/// table with the current six.
///
/// Two unit tests below keep the three copies honest:
/// - `asset_lock_status_labels_match_enum` — this array ⇔ the writer's
///   codomain ([`status_str`]);
/// - `asset_lock_status_labels_frozen_in_latest_migration` — this array
///   ⇔ the latest migration's frozen list, so ADDING a variant fails
///   with instructions to append a new table-rebuild migration (V005+)
///   instead of editing a shipped one.
#[cfg(test)]
pub(crate) const ASSET_LOCK_STATUS_LABELS: &[&str] = &[
    "built",
    "broadcast",
    "is_locked",
    "chain_locked",
    "consumed",
    "recovered_from_chain",
];

fn status_str(s: &AssetLockStatus) -> &'static str {
    match s {
        AssetLockStatus::Built => "built",
        AssetLockStatus::Broadcast => "broadcast",
        AssetLockStatus::InstantSendLocked => "is_locked",
        AssetLockStatus::ChainLocked => "chain_locked",
        AssetLockStatus::Consumed => "consumed",
        AssetLockStatus::RecoveredFromChain => "recovered_from_chain",
    }
}

/// Per-wallet asset-lock slice as returned by the readers — outer-keyed
/// by `account_index`, inner-keyed by outpoint.
#[cfg(any(test, feature = "__test-helpers"))]
pub type AssetLocksByAccount = BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>;

/// Decode one raw `(outpoint_bytes, account_index, lifecycle_blob)`
/// tuple into the typed `(account_index, OutPoint, TrackedAssetLock)`
/// triple that [`load_state`] consumes.
///
/// Hard-fail behaviour: a malformed outpoint, blob, or out-of-range
/// account index returns a typed [`WalletStorageError`]. Every caller
/// propagates that error — corruption is never silently skipped.
#[cfg(any(test, feature = "__test-helpers"))]
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
    // Typed-column vs blob cross-check, symmetric with
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
/// silently dropped. Retained for this crate's integration tests until
/// the rehydration path consumes it in `load()`.
#[cfg(any(test, feature = "__test-helpers"))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Exhaustive sample of every [`AssetLockStatus`] variant. The
    /// trailing match arm in the loop fails to compile if upstream
    /// adds a variant — forcing the developer to extend the list,
    /// `status_str`, and [`ASSET_LOCK_STATUS_LABELS`] together.
    fn all_asset_lock_status_variants() -> Vec<AssetLockStatus> {
        let variants = vec![
            AssetLockStatus::Built,
            AssetLockStatus::Broadcast,
            AssetLockStatus::InstantSendLocked,
            AssetLockStatus::ChainLocked,
            AssetLockStatus::Consumed,
            AssetLockStatus::RecoveredFromChain,
        ];
        for v in &variants {
            match v {
                AssetLockStatus::Built
                | AssetLockStatus::Broadcast
                | AssetLockStatus::InstantSendLocked
                | AssetLockStatus::ChainLocked
                | AssetLockStatus::Consumed
                | AssetLockStatus::RecoveredFromChain => {}
            }
        }
        variants
    }

    #[test]
    fn asset_lock_status_labels_match_enum() {
        let from_writer: HashSet<&'static str> = all_asset_lock_status_variants()
            .iter()
            .map(status_str)
            .collect();
        let from_const: HashSet<&'static str> = ASSET_LOCK_STATUS_LABELS.iter().copied().collect();
        assert_eq!(
            from_writer, from_const,
            "ASSET_LOCK_STATUS_LABELS ({:?}) drifted from status_str codomain ({:?})",
            from_const, from_writer
        );
    }

    /// Pins the live label set to the domain frozen in the LATEST
    /// asset-lock migration (`V004__asset_lock_recovered_status.rs`).
    /// Shipped migrations interpolate nothing — their generated SQL is
    /// checksummed by Refinery, so widening the domain means APPENDING
    /// a new table-rebuild migration (V005+) with the new frozen list
    /// and updating this pin, never editing V001/V004 in place.
    ///
    /// IF THIS FAILS: do NOT edit a shipped migration (its Refinery
    /// checksum would diverge on already-migrated databases). Append a
    /// new migration that rebuilds `asset_locks` with the widened
    /// CHECK, then update this pin to the new migration's list.
    #[test]
    fn asset_lock_status_labels_frozen_in_latest_migration() {
        let frozen_in_v004 = [
            "built",
            "broadcast",
            "is_locked",
            "chain_locked",
            "consumed",
            "recovered_from_chain",
        ];
        assert_eq!(ASSET_LOCK_STATUS_LABELS, &frozen_in_v004);
    }
}
