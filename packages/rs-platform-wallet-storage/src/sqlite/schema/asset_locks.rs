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

/// Source of truth for the `asset_locks.status` TEXT domain, mirroring
/// [`platform_wallet::wallet::asset_lock::tracked::AssetLockStatus`].
/// `migrations/V001__initial.rs` interpolates it into a `CHECK (status IN
/// (...))`; `asset_lock_status_labels_match_enum` keeps it in sync with
/// [`status_str`].
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

/// Decode one raw `(outpoint_bytes, account_index, lifecycle_blob, status)`
/// tuple into the typed `(account_index, OutPoint, TrackedAssetLock)`
/// triple that the reader functions consume.
///
/// Hard-fail behaviour: a malformed outpoint, blob, out-of-range
/// account index, or a mismatch between the typed columns and the blob
/// returns a typed [`WalletStorageError`]. Every caller propagates that
/// error — corruption is never silently skipped.
fn decode_row(
    op_bytes: &[u8],
    account_index: i64,
    blob_bytes: &[u8],
    typed_status: &str,
) -> Result<(u32, OutPoint, TrackedAssetLock), WalletStorageError> {
    let outpoint = blob::decode_outpoint(op_bytes)?;
    let entry: AssetLockEntry = blob::decode(blob_bytes)?;
    let account_index =
        crate::sqlite::util::safe_cast::i64_to_u32("asset_locks.account_index", account_index)?;
    // Typed-column vs blob cross-check: corruption that passes PRAGMA
    // integrity_check would otherwise mis-bucket the lock or report a
    // different outpoint / account index than the indexed columns it was
    // selected by.
    if entry.out_point != outpoint || entry.account_index != account_index {
        return Err(WalletStorageError::AssetLockEntryMismatch {
            typed_outpoint: outpoint.to_string(),
            blob_outpoint: entry.out_point.to_string(),
            typed_account_index: account_index,
            blob_account_index: entry.account_index,
        });
    }
    // Status cross-check: the typed `status` column drives SQL-level filters
    // (e.g. `load_unconsumed`'s `status NOT IN ('consumed')`), so a blob that
    // disagrees with the column would cause a consumed lock to re-enter the
    // live set or an active lock to be filtered out.
    if status_str(&entry.status) != typed_status {
        return Err(WalletStorageError::BlobDecode {
            reason: "asset_locks.status column disagrees with lifecycle_blob status",
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

/// Full-history asset-lock slice bucketed by account index, **including**
/// terminal `Consumed` rows (inspection reader for this crate's tests). Use
/// [`load_unconsumed`] for the rehydration feed. A row that fails to decode is
/// a hard error.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT length(outpoint), outpoint, account_index, length(lifecycle_blob), lifecycle_blob, status \
         FROM asset_locks WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out: AssetLocksByAccount = BTreeMap::new();
    while let Some(row) = rows.next()? {
        blob::check_size(row.get::<_, i64>(0)?)?;
        let op_bytes: Vec<u8> = row.get(1)?;
        let account_index: i64 = row.get(2)?;
        blob::check_size(row.get::<_, i64>(3)?)?;
        let blob_bytes: Vec<u8> = row.get(4)?;
        let status: String = row.get(5)?;
        let (acct, outpoint, tracked) = decode_row(&op_bytes, account_index, &blob_bytes, &status)?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}

/// Status-filtered rehydration feed: every asset lock **except** terminal
/// `Consumed` rows, bucketed by account index. Feeding `Consumed` locks back
/// into the live set would resurrect a spent one-shot lock as actionable
/// (A04/A08), so the exclusion is at the SQL level (`status NOT IN
/// ('consumed')`, `status` indexed); history stays visible via [`load_state`].
/// A row that fails to decode is a hard [`WalletStorageError`].
pub fn load_unconsumed(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT length(outpoint), outpoint, account_index, length(lifecycle_blob), lifecycle_blob, status \
         FROM asset_locks WHERE wallet_id = ?1 AND status NOT IN ('consumed')",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out: AssetLocksByAccount = BTreeMap::new();
    while let Some(row) = rows.next()? {
        blob::check_size(row.get::<_, i64>(0)?)?;
        let op_bytes: Vec<u8> = row.get(1)?;
        let account_index: i64 = row.get(2)?;
        blob::check_size(row.get::<_, i64>(3)?)?;
        let blob_bytes: Vec<u8> = row.get(4)?;
        let status: String = row.get(5)?;
        let (acct, outpoint, tracked) = decode_row(&op_bytes, account_index, &blob_bytes, &status)?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}

/// Every asset lock bucketed by account index, **including** terminal
/// `Consumed` — history/inspection only; use [`load_unconsumed`] for the
/// rehydration feed. A row that fails to decode is a hard
/// [`WalletStorageError`].
#[cfg(any(test, feature = "__test-helpers"))]
pub fn list_active(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT length(outpoint), outpoint, account_index, length(lifecycle_blob), lifecycle_blob, status \
         FROM asset_locks WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out: AssetLocksByAccount = BTreeMap::new();
    while let Some(row) = rows.next()? {
        blob::check_size(row.get::<_, i64>(0)?)?;
        let op_bytes: Vec<u8> = row.get(1)?;
        let account_index: i64 = row.get(2)?;
        blob::check_size(row.get::<_, i64>(3)?)?;
        let blob_bytes: Vec<u8> = row.get(4)?;
        let status: String = row.get(5)?;
        let (acct, outpoint, tracked) = decode_row(&op_bytes, account_index, &blob_bytes, &status)?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Open an in-memory connection with the full schema applied.
    fn migrated_conn() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn
    }

    /// `decode_row` (and the reader functions that call it) must reject a row
    /// whose `status` TEXT column disagrees with the `lifecycle_blob` status,
    /// returning a `BlobDecode` corrupt error rather than silently mis-bucketing
    /// a lock (e.g. treating a `Consumed` lock as `Built`).
    #[test]
    fn load_state_rejects_status_column_mismatch() {
        use dashcore::hashes::Hash;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
        use platform_wallet::changeset::AssetLockEntry;

        let mut conn = migrated_conn();
        let w = [0xAAu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();

        // Build a minimal `AssetLockEntry` with `status = Built`.
        let outpoint = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array([0x01u8; 32]),
            vout: 0,
        };
        let entry = AssetLockEntry {
            out_point: outpoint,
            // Dashcore Transaction with integer version and lock_time.
            transaction: dashcore::Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityTopUp,
            identity_index: 0,
            amount_duffs: 1000,
            status: AssetLockStatus::Built,
            proof: None,
        };
        let lifecycle_blob = blob::encode(&entry).unwrap();
        let op_bytes = blob::encode_outpoint(&outpoint).unwrap();

        // Insert with status column = 'consumed' but blob says 'built'.
        {
            let tx = conn.transaction().unwrap();
            tx.execute(
                "INSERT INTO asset_locks \
                    (wallet_id, outpoint, status, account_index, identity_index, \
                     amount_duffs, lifecycle_blob) \
                 VALUES (?1, ?2, 'consumed', 0, 0, 1000, ?3)",
                params![&w[..], &op_bytes[..], lifecycle_blob],
            )
            .unwrap();
            tx.commit().unwrap();
        }

        // load_state must fail: status column ('consumed') ≠ blob status ('built').
        let err = load_state(&conn, &w)
            .expect_err("load_state must reject a status column vs blob mismatch");
        assert!(
            matches!(err, WalletStorageError::BlobDecode { .. }),
            "expected BlobDecode for status mismatch, got {err:?}"
        );
    }

    /// Every [`AssetLockStatus`] variant; the wildcard-free match below fails
    /// to compile if upstream adds one.
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
