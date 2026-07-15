//! `asset_locks` table writer + reader.
//!
//! Each row stores the lifecycle status as a string column for direct
//! SQL queries, plus a bincode-serde encoded [`AssetLockEntryWire`] in the
//! `lifecycle_blob` column.
//!
//! `AssetLockEntry.proof`'s `#[serde(tag = "$type")]` enum is rejected by
//! bincode-serde (needs `deserialize_any`), so [`AssetLockEntryWire`]
//! pre-encodes the proof with bincode's native `Encode`/`Decode` and rides
//! the surrounding fields on the serde encoder — mirroring `IdentityKeyWire`
//! in `identity_keys.rs`.

use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};

use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
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

/// On-disk wire shape for [`AssetLockEntry`]. `proof` rides as a natively
/// bincode-encoded `Option<Vec<u8>>` rather than routing the internally-tagged
/// [`AssetLockProof`] enum through the `bincode::serde` bridge — that bridge
/// cannot service the `deserialize_any` an internally-tagged serde enum needs,
/// which made a `proof: Some(..)` row write-once/read-never (issue #4133).
///
/// Fields 1–7 preserve the exact order and serde encodings of `AssetLockEntry`
/// (including the shared `asset_lock_funding_type` adapter) so a pre-fix
/// `proof: None` row decodes byte-identically under this type — no migration
/// needed for those rows.
#[derive(Serialize, Deserialize)]
struct AssetLockEntryWire {
    out_point: OutPoint,
    transaction: dashcore::Transaction,
    account_index: u32,
    #[serde(with = "platform_wallet::changeset::serde_adapters::asset_lock_funding_type")]
    funding_type: AssetLockFundingType,
    identity_index: u32,
    amount_duffs: u64,
    status: AssetLockStatus,
    /// Natively bincode-encoded [`AssetLockProof`]; see the type docs.
    proof: Option<Vec<u8>>,
}

// PUBLIC material only: asset-lock lifecycle reaching `lifecycle_blob`.
impl_persistable_blob!(AssetLockEntryWire);

/// Encode an [`AssetLockEntry`] to its on-disk `lifecycle_blob` form via the
/// wire type. Test-only seam so integration tests can forge a lifecycle blob
/// without naming the crate-internal [`AssetLockEntryWire`].
#[cfg(any(test, feature = "__test-helpers"))]
pub fn encode_entry_for_test(entry: &AssetLockEntry) -> Result<Vec<u8>, WalletStorageError> {
    blob::encode(&AssetLockEntryWire::from_entry(entry)?)
}

impl AssetLockEntryWire {
    /// Project an [`AssetLockEntry`] onto its wire shape, pre-encoding the
    /// proof natively (see the type docs).
    fn from_entry(entry: &AssetLockEntry) -> Result<Self, WalletStorageError> {
        let proof = entry
            .proof
            .as_ref()
            .map(|p| bincode::encode_to_vec(p, blob::bounded_config()))
            .transpose()?;
        Ok(Self {
            out_point: entry.out_point,
            transaction: entry.transaction.clone(),
            account_index: entry.account_index,
            funding_type: entry.funding_type,
            identity_index: entry.identity_index,
            amount_duffs: entry.amount_duffs,
            status: entry.status.clone(),
            proof,
        })
    }

    /// Reconstruct the [`AssetLockEntry`], natively decoding the proof.
    /// Rejects trailing bytes past the typed proof length, mirroring
    /// `IdentityKeyWire::into_entry` and the outer `blob::decode` guard.
    fn into_entry(self) -> Result<AssetLockEntry, WalletStorageError> {
        let proof = match self.proof {
            Some(bytes) => {
                let (proof, consumed): (AssetLockProof, usize) =
                    bincode::decode_from_slice(&bytes, blob::bounded_config())?;
                if consumed != bytes.len() {
                    return Err(WalletStorageError::blob_decode(
                        "unexpected trailing bytes in asset_locks proof bincode",
                    ));
                }
                Some(proof)
            }
            None => None,
        };
        Ok(AssetLockEntry {
            out_point: self.out_point,
            transaction: self.transaction,
            account_index: self.account_index,
            funding_type: self.funding_type,
            identity_index: self.identity_index,
            amount_duffs: self.amount_duffs,
            status: self.status,
            proof,
        })
    }
}

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
            let lifecycle_blob = blob::encode(&AssetLockEntryWire::from_entry(entry)?)?;
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
    let wire: AssetLockEntryWire = blob::decode(blob_bytes)?;
    let entry = wire.into_entry()?;
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

    use dashcore::hashes::Hash;

    fn sample_outpoint(b: u8) -> OutPoint {
        OutPoint {
            txid: dashcore::Txid::from_byte_array([b; 32]),
            vout: 0,
        }
    }

    fn sample_transaction() -> dashcore::Transaction {
        dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        }
    }

    /// An `AssetLockEntry` at the given status carrying `proof`.
    fn entry_with_proof(
        op: OutPoint,
        status: AssetLockStatus,
        proof: Option<AssetLockProof>,
    ) -> AssetLockEntry {
        AssetLockEntry {
            out_point: op,
            transaction: sample_transaction(),
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityRegistration,
            identity_index: 0,
            amount_duffs: 1000,
            status,
            proof,
        }
    }

    /// Store `entry` through the real `apply` writer, then read it back
    /// with `load_state`. Returns the reconstructed entry so a test can
    /// pin a full DB round-trip including the proof.
    fn roundtrip_through_db(entry: &AssetLockEntry) -> AssetLockEntry {
        let mut conn = migrated_conn();
        let w = [0xAAu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        let mut cs = AssetLockChangeSet::default();
        cs.asset_locks.insert(entry.out_point, entry.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &w, &cs).unwrap();
            tx.commit().unwrap();
        }
        let mut locks = load_state(&conn, &w).unwrap();
        locks
            .remove(&entry.account_index)
            .and_then(|mut by_op| by_op.remove(&entry.out_point))
            .map(|tracked| AssetLockEntry {
                out_point: tracked.out_point,
                transaction: tracked.transaction,
                account_index: tracked.account_index,
                funding_type: tracked.funding_type,
                identity_index: tracked.identity_index,
                amount_duffs: tracked.amount,
                status: tracked.status,
                proof: tracked.proof,
            })
            .expect("stored asset lock must load back")
    }

    /// Repro for #4133: a `proof: Some(AssetLockProof::Chain(..))` row must
    /// survive a full `apply` → `load_state` round-trip. Before the wire-type
    /// fix this failed at decode with `BincodeDecode`/`AnyNotSupported`, because
    /// the internally-tagged proof enum was routed through the `bincode::serde`
    /// bridge — write-once, read-never.
    #[test]
    fn wire_round_trips_chain_proof() {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        let op = sample_outpoint(0x31);
        let proof = AssetLockProof::Chain(ChainAssetLockProof::new(42, [0x07u8; 36]));
        let entry = entry_with_proof(op, AssetLockStatus::ChainLocked, Some(proof));
        let restored = roundtrip_through_db(&entry);
        assert_eq!(restored, entry, "Chain proof must survive the round-trip");
    }

    /// Repro for #4133 (Instant variant): a `proof:
    /// Some(AssetLockProof::Instant(..))` row must also survive the round-trip.
    #[test]
    fn wire_round_trips_instant_proof() {
        use dpp::identity::state_transition::asset_lock_proof::instant::InstantAssetLockProof;
        let op = sample_outpoint(0x32);
        // Distinct, non-default field values so the round-trip proves field
        // fidelity, not merely that `default() == default()`.
        let inner = {
            let mut p = InstantAssetLockProof::default();
            p.transaction.version = 3;
            p.transaction.lock_time = 111;
            p.output_index = 2;
            // `default()` leaves the nested `InstantLock.inputs` empty; a real
            // IS-lock always carries at least one input, so populate it to
            // exercise the length-prefixed-vec encoding path a genuine proof
            // hits (QA-004).
            p.instant_lock.inputs = vec![sample_outpoint(0xA1), sample_outpoint(0xA2)];
            p
        };
        let proof = AssetLockProof::Instant(inner);
        let entry = entry_with_proof(op, AssetLockStatus::InstantSendLocked, Some(proof));
        let restored = roundtrip_through_db(&entry);
        assert_eq!(restored, entry, "Instant proof must survive the round-trip");
    }

    /// The wire type rejects a `proof` payload whose `AssetLockProof` prefix is
    /// valid but carries trailing garbage, rather than silently dropping the
    /// tail — mirrors `into_entry_rejects_trailing_bytes_in_public_key_bincode`.
    #[test]
    fn into_entry_rejects_trailing_bytes_in_proof_bincode() {
        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        let proof = AssetLockProof::Chain(ChainAssetLockProof::new(7, [0x01u8; 36]));
        let mut proof_bincode = bincode::encode_to_vec(&proof, blob::bounded_config()).unwrap();
        proof_bincode.push(0xFF); // trailing garbage past the typed length

        let wire = AssetLockEntryWire {
            out_point: sample_outpoint(0x33),
            transaction: sample_transaction(),
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityRegistration,
            identity_index: 0,
            amount_duffs: 1000,
            status: AssetLockStatus::ChainLocked,
            proof: Some(proof_bincode),
        };
        let err = wire.into_entry().expect_err("trailing bytes must error");
        assert!(
            matches!(err, WalletStorageError::BlobDecode { .. }),
            "expected BlobDecode for trailing-byte garbage, got {err:?}"
        );
    }

    /// Compat pin: a pre-fix `proof: None` row (serialized straight from
    /// `AssetLockEntry`, the old on-disk shape) encodes byte-identically to the
    /// new `AssetLockEntryWire` and decodes back unchanged. Guarantees the
    /// common case rehydrates with no migration.
    #[test]
    fn none_proof_row_is_byte_identical_across_shapes() {
        let entry = entry_with_proof(sample_outpoint(0x34), AssetLockStatus::Built, None);

        // Old on-disk bytes: exactly what the pre-fix `blob::encode` produced
        // (`bincode::serde::encode_to_vec` over the `AssetLockEntry` itself).
        let old_bytes = bincode::serde::encode_to_vec(&entry, blob::bounded_config()).unwrap();
        let new_bytes = blob::encode(&AssetLockEntryWire::from_entry(&entry).unwrap()).unwrap();
        assert_eq!(
            old_bytes, new_bytes,
            "a None-proof row must be byte-identical old vs new"
        );

        // And the old bytes still decode under the new wire type.
        let wire: AssetLockEntryWire = blob::decode(&old_bytes).unwrap();
        assert_eq!(
            wire.into_entry().unwrap(),
            entry,
            "pre-fix None row must decode under the new wire type"
        );
    }

    /// `decode_row` (and the reader functions that call it) must reject a row
    /// whose `status` TEXT column disagrees with the `lifecycle_blob` status,
    /// returning a `BlobDecode` corrupt error rather than silently mis-bucketing
    /// a lock (e.g. treating a `Consumed` lock as `Built`).
    #[test]
    fn load_state_rejects_status_column_mismatch() {
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
        let lifecycle_blob =
            blob::encode(&AssetLockEntryWire::from_entry(&entry).unwrap()).unwrap();
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
