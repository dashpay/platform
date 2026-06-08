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

use {
    dashcore::OutPoint, platform_wallet::changeset::AssetLockEntry,
    platform_wallet::wallet::asset_lock::tracked::TrackedAssetLock, rusqlite::Connection,
    std::collections::BTreeMap,
};

use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: asset-lock lifecycle reaching `lifecycle_blob`.
impl_persistable_blob!(AssetLockEntry);

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

/// Single source of truth for the `asset_locks.status` TEXT-column
/// domain.
///
/// Mirrors every variant of
/// [`platform_wallet::wallet::asset_lock::tracked::AssetLockStatus`]
/// (writer side: [`status_str`]). The migration in
/// `migrations/V001__initial.rs` interpolates this array into the
/// `CHECK (status IN (...))` clause so an unknown label is rejected at
/// insert time rather than landing as silent garbage. The
/// `asset_lock_status_labels_match_enum` unit test below enforces
/// set-equality between this array and the writer's output — drift (a
/// renamed/added variant) becomes a failing test, not a runtime
/// divergence between Rust and SQLite.
pub(crate) const ASSET_LOCK_STATUS_LABELS: &[&str] = &[
    "built",
    "broadcast",
    "is_locked",
    "chain_locked",
    "consumed",
];

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
        crate::sqlite::util::safe_cast::i64_to_u32("asset_locks.account_index", account_index)?;
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
/// the `asset_locks` table, bucketed by account index — including
/// terminal `Consumed` rows. Consumed locks are RETAINED permanently
/// with `status='consumed'` (they are never deleted), so this reader
/// returns the full history. The rehydration feed
/// [`load_unconsumed`](Self::load_unconsumed) filters `Consumed` rows at
/// the SQL layer so a spent lock is never resurrected into the live set.
/// Any row that fails to read or decode is a hard error — corruption is
/// never silently dropped. Retained as the full-history inspection
/// reader for this crate's integration tests.
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
#[cfg(any(test, feature = "__test-helpers"))]
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
        ];
        for v in &variants {
            match v {
                AssetLockStatus::Built
                | AssetLockStatus::Broadcast
                | AssetLockStatus::InstantSendLocked
                | AssetLockStatus::ChainLocked
                | AssetLockStatus::Consumed => {}
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
}
