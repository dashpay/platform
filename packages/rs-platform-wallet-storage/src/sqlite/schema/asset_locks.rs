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

use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use dpp::prelude::AssetLockProof;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use platform_wallet::changeset::{AssetLockChangeSet, AssetLockPersistenceAction};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::blob;

use {
    dashcore::{OutPoint, Txid},
    platform_wallet::changeset::AssetLockEntry,
    platform_wallet::wallet::asset_lock::tracked::TrackedAssetLock,
    rusqlite::Connection,
    std::collections::{BTreeMap, HashSet},
};

use crate::sqlite::schema::blob::impl_persistable_blob;

/// On-disk wire shape for [`AssetLockEntry`]. `proof` rides as a natively
/// bincode-encoded `Option<Vec<u8>>` rather than routing the internally-tagged
/// [`AssetLockProof`] enum through the `bincode::serde` bridge — that bridge
/// cannot service the `deserialize_any` an internally-tagged serde enum needs,
/// which made a `proof: Some(..)` row write-once/read-never (issue #4133).
///
/// Fields 1–7 preserve the historical `AssetLockEntry` order and serde encodings
/// (including the shared `asset_lock_funding_type` adapter) so a pre-fix
/// `proof: None` row decodes byte-identically under this type — no migration
/// needed for those rows. The later operation UUID lives in a separate nullable
/// column so adding it never changes this published bincode layout.
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

/// Nullable recovery columns read independently of the frozen lifecycle blob.
struct AssetLockRecoveryRow {
    operation_id: Option<Vec<u8>>,
    lifecycle_blob: Vec<u8>,
    operation_revision: i64,
}

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
            operation_id: None,
            operation_revision: 0,
            expected_operation_revision: None,
        })
    }
}

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &AssetLockChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.asset_locks.is_empty() {
        // The wallet owner decides UUID, revision, and terminal-state policy
        // before this backend mutates either lifecycle or recovery columns.
        // SQL also protects a consumed column if its blob has drifted.
        let mut stmt = tx.prepare_cached(
            "INSERT INTO asset_locks \
                (wallet_id, outpoint, status, account_index, identity_index, amount_duffs, lifecycle_blob, operation_id, operation_revision) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
                status = excluded.status, \
                account_index = excluded.account_index, \
                identity_index = excluded.identity_index, \
                amount_duffs = excluded.amount_duffs, \
                lifecycle_blob = excluded.lifecycle_blob, \
                operation_id = COALESCE(excluded.operation_id, asset_locks.operation_id), \
                operation_revision = excluded.operation_revision \
             WHERE asset_locks.status != 'consumed' OR excluded.status = 'consumed'",
        )?;
        for (op, entry) in &cs.asset_locks {
            let op_bytes = blob::encode_outpoint(op)?;
            let lifecycle_blob = blob::encode(&AssetLockEntryWire::from_entry(entry)?)?;
            let existing: Option<AssetLockRecoveryRow> = tx.query_row(
                "SELECT operation_id, lifecycle_blob, operation_revision FROM asset_locks WHERE wallet_id = ?1 AND outpoint = ?2",
                params![wallet_id.as_slice(), &op_bytes[..]],
                |row| Ok(AssetLockRecoveryRow {
                    operation_id: row.get(0)?,
                    lifecycle_blob: row.get(1)?,
                    operation_revision: row.get(2)?,
                }),
            ).optional()?;
            let previous = existing
                .map(|stored| -> Result<_, WalletStorageError> {
                    let old: AssetLockEntryWire = blob::decode(&stored.lifecycle_blob)?;
                    let mut previous = old.into_entry()?;
                    previous.operation_revision = u64::try_from(stored.operation_revision)
                        .map_err(|_| {
                            WalletStorageError::blob_decode("negative operation revision")
                        })?;
                    previous.operation_id = stored
                        .operation_id
                        .map(|id| {
                            id.try_into().map_err(|_| {
                                WalletStorageError::blob_decode("invalid operation UUID")
                            })
                        })
                        .transpose()?;
                    Ok(previous)
                })
                .transpose()?;
            match entry
                .persistence_action(previous.as_ref())
                .map_err(|error| WalletStorageError::blob_decode(error.reason()))?
            {
                AssetLockPersistenceAction::Apply => {}
                AssetLockPersistenceAction::Ignore => continue,
            }
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
                entry.operation_id.as_ref().map(|id| id.as_slice()),
                crate::sqlite::util::safe_cast::u64_to_i64(
                    "asset_locks.operation_revision",
                    entry.operation_revision
                )?,
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        // Only unbound locks and known-unsent Built operation rows may be
        // removed. Delayed cleanup cannot erase a dispatched or revised UUID.
        // This mirrors AssetLockStatus::permits_removal in the owner.
        let mut stmt = tx.prepare_cached(
            "DELETE FROM asset_locks \
             WHERE wallet_id = ?1 AND outpoint = ?2 AND status != 'consumed' AND operation_revision = 0 AND (operation_id IS NULL OR status = 'built')",
        )?;
        for op in &cs.removed {
            let op_bytes = blob::encode_outpoint(op)?;
            stmt.execute(params![wallet_id.as_slice(), &op_bytes[..]])?;
        }
    }
    Ok(())
}

/// Block bound replay losers and remove unbound ones in the Core repair transaction.
pub(crate) fn remove_swept(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    txids: &HashSet<Txid>,
) -> Result<(), WalletStorageError> {
    let mut changes = AssetLockChangeSet::default();
    for mut lock in load_unconsumed(tx, wallet_id, &LoadCtx::strict())?
        .into_values()
        .flat_map(BTreeMap::into_values)
    {
        if !txids.contains(&lock.out_point.txid)
            || lock.status.is_consumed()
            || lock.status == AssetLockStatus::Conflicted
        {
            continue;
        }
        if lock.operation_id.is_some() {
            lock.status = AssetLockStatus::Conflicted;
            lock.proof = None;
            let mut entry = AssetLockEntry::from(&lock);
            entry.advance_operation_revision();
            changes.asset_locks.insert(lock.out_point, entry);
        } else {
            changes.removed.insert(lock.out_point);
        }
    }
    apply(tx, wallet_id, &changes)
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
/// five labels; `V022__asset_lock_conflict_evidence.rs` rebuilt the
/// table with the current seven.
///
/// Two unit tests below keep the three copies honest:
/// - `asset_lock_status_labels_match_enum` — this array ⇔ the writer's
///   codomain ([`status_str`]);
/// - `asset_lock_status_labels_frozen_in_latest_migration` — this array
///   ⇔ the latest migration's frozen list, so ADDING a variant fails
///   with instructions to append a new table-rebuild migration (V018+)
///   instead of editing a shipped one.
#[cfg(test)]
pub(crate) const ASSET_LOCK_STATUS_LABELS: &[&str] = &[
    "built",
    "broadcast",
    "is_locked",
    "chain_locked",
    "consumed",
    "recovered_from_chain",
    "conflicted",
];

fn status_str(s: &AssetLockStatus) -> &'static str {
    match s {
        AssetLockStatus::Built => "built",
        AssetLockStatus::Broadcast => "broadcast",
        AssetLockStatus::InstantSendLocked => "is_locked",
        AssetLockStatus::ChainLocked => "chain_locked",
        AssetLockStatus::Consumed | AssetLockStatus::DrainCompleted => "consumed",
        AssetLockStatus::RecoveredFromChain => "recovered_from_chain",
        AssetLockStatus::Conflicted => "conflicted",
    }
}

/// Per-wallet asset-lock slice as returned by the readers — outer-keyed
/// by `account_index`, inner-keyed by outpoint.
pub type AssetLocksByAccount = BTreeMap<u32, BTreeMap<OutPoint, TrackedAssetLock>>;

/// Decode one raw `(outpoint_bytes, account_index, lifecycle_blob, status)`
/// tuple into the typed `(account_index, OutPoint, TrackedAssetLock)`
/// triple that the reader functions consume.
///
/// Malformed fields and entry mismatches are always fatal. Status drift is
/// policy-controlled: Recovery uses the blob status with consumption sticky.
fn decode_row(
    op_bytes: &[u8],
    account_index: i64,
    blob_bytes: &[u8],
    typed_status: &str,
    operation_id: Option<Vec<u8>>,
    operation_revision: i64,
    ctx: &LoadCtx,
) -> Result<(u32, OutPoint, TrackedAssetLock), WalletStorageError> {
    let outpoint = blob::decode_outpoint(op_bytes)?;
    let wire: AssetLockEntryWire = blob::decode(blob_bytes)?;
    let mut entry = wire.into_entry()?;
    entry.operation_id = operation_id
        .map(|bytes| {
            bytes.try_into().map_err(|_| {
                WalletStorageError::blob_decode("asset_locks.operation_id must contain 16 bytes")
            })
        })
        .transpose()?;
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
    let operation_revision = u64::try_from(operation_revision)
        .map_err(|_| WalletStorageError::blob_decode("negative operation revision"))?;
    entry.operation_revision = operation_revision;
    if (entry.status == AssetLockStatus::Conflicted || operation_revision > 0)
        && (entry.operation_id.is_none() || operation_revision == 0)
    {
        return Err(WalletStorageError::blob_decode(
            "conflict evidence lacks an operation binding or revision",
        ));
    }
    let blob_status = status_str(&entry.status);
    if blob_status != typed_status
        && (typed_status == "conflicted" || entry.status == AssetLockStatus::Conflicted)
    {
        return Err(WalletStorageError::blob_decode(
            "asset lock conflict evidence disagrees with status column",
        ));
    }
    if blob_status != typed_status {
        let consumed = typed_status == "consumed" || entry.status.is_consumed();
        ctx.tolerate(
            LoadSite::AssetLockStatusDrift,
            WalletStorageError::AssetLockStatusMismatch {
                outpoint: outpoint.to_string(),
                typed_status: typed_status.to_owned(),
                blob_status: blob_status.to_owned(),
            },
        )?;
        // `load_unconsumed` pre-filters typed `consumed` rows, so the blob can
        // only withdraw a lock there. Sticky consumption also keeps the
        // unfiltered test/inspection reader conservative.
        if consumed && entry.status != AssetLockStatus::DrainCompleted {
            entry.status = AssetLockStatus::Consumed;
        }
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
        operation_id: entry.operation_id,
        operation_revision: entry.operation_revision,
    };
    Ok((account_index, outpoint, tracked))
}

/// Full-history asset-lock slice bucketed by account index, **including**
/// terminal `Consumed` rows (inspection reader for this crate's tests). Use
/// [`load_unconsumed`] for the rehydration feed. Status drift is fatal under
/// Strict; Recovery uses the blob status with consumption kept sticky.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT length(outpoint), outpoint, account_index, length(lifecycle_blob), lifecycle_blob, status, operation_id, operation_revision \
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
        let operation_id: Option<Vec<u8>> = row.get(6)?;
        let (acct, outpoint, tracked) = decode_row(
            &op_bytes,
            account_index,
            &blob_bytes,
            &status,
            operation_id,
            row.get(7)?,
            ctx,
        )?;
        out.entry(acct).or_default().insert(outpoint, tracked);
    }
    Ok(out)
}

/// Status-filtered rehydration feed: every asset lock **except** terminal
/// `Consumed` rows without an operation ID, bucketed by account index.
/// Consumed wallet-drain locks are non-actionable recovery tombstones: retaining
/// their ID prevents a restart from mistaking a completed operation for a new one.
/// Legacy consumed locks remain excluded; full history is available via
/// [`load_state`]. Malformed rows remain fatal. Recovery keeps consumption
/// sticky when the typed column and blob disagree.
pub fn load_unconsumed(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<AssetLocksByAccount, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT length(outpoint), outpoint, account_index, length(lifecycle_blob), lifecycle_blob, status, operation_id, operation_revision \
         FROM asset_locks WHERE wallet_id = ?1 AND (status != 'consumed' OR operation_id IS NOT NULL)",
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
        let operation_id: Option<Vec<u8>> = row.get(6)?;
        let (acct, outpoint, tracked) = decode_row(
            &op_bytes,
            account_index,
            &blob_bytes,
            &status,
            operation_id,
            row.get(7)?,
            ctx,
        )?;
        if tracked.status.is_consumed() && tracked.operation_id.is_none() {
            continue;
        }
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
    load_state(conn, wallet_id, &LoadCtx::strict())
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
            operation_id: None,
            operation_revision: 0,
            expected_operation_revision: None,
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
        let mut locks = load_state(&conn, &w, &LoadCtx::strict()).unwrap();
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
                operation_id: tracked.operation_id,
                operation_revision: tracked.operation_revision,
                expected_operation_revision: None,
            })
            .expect("stored asset lock must load back")
    }

    #[test]
    fn should_preserve_proven_drain_completion_over_stale_consumed_snapshot() {
        let mut conn = migrated_conn();
        let wallet = [0xAA; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet[..]],
        )
        .unwrap();
        let mut entry =
            entry_with_proof(sample_outpoint(90), AssetLockStatus::DrainCompleted, None);
        entry.operation_id = Some([9; 16]);
        assert_eq!(
            roundtrip_through_db(&entry).status,
            AssetLockStatus::DrainCompleted
        );
        for status in [
            AssetLockStatus::DrainCompleted,
            AssetLockStatus::Consumed,
            AssetLockStatus::Built,
        ] {
            entry.status = status;
            let mut cs = AssetLockChangeSet::default();
            cs.asset_locks.insert(entry.out_point, entry.clone());
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet, &cs).unwrap();
            tx.commit().unwrap();
        }
        let restored = load_state(&conn, &wallet, &LoadCtx::strict()).unwrap();
        assert_eq!(
            restored[&0][&entry.out_point].status,
            AssetLockStatus::DrainCompleted
        );
        let projection: String = conn
            .query_row("SELECT status FROM asset_locks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(projection, "consumed");
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
            // uses.
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

        // Freeze the historical shape rather than following new fields on the
        // live entry. UUIDs were added later and deliberately live outside this blob.
        #[derive(Serialize)]
        struct LegacyEntry<'a> {
            out_point: OutPoint,
            transaction: &'a dashcore::Transaction,
            account_index: u32,
            #[serde(with = "platform_wallet::changeset::serde_adapters::asset_lock_funding_type")]
            funding_type: AssetLockFundingType,
            identity_index: u32,
            amount_duffs: u64,
            status: &'a AssetLockStatus,
            proof: &'a Option<AssetLockProof>,
        }
        let legacy = LegacyEntry {
            out_point: entry.out_point,
            transaction: &entry.transaction,
            account_index: entry.account_index,
            funding_type: entry.funding_type,
            identity_index: entry.identity_index,
            amount_duffs: entry.amount_duffs,
            status: &entry.status,
            proof: &entry.proof,
        };
        let old_bytes = bincode::serde::encode_to_vec(&legacy, blob::bounded_config()).unwrap();
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

    /// Strict rejects status drift with a typed error; Recovery keeps
    /// consumption sticky so a stale blob cannot resurrect a spent lock.
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
            operation_id: None,
            operation_revision: 0,
            expected_operation_revision: None,
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

        // Strict rejects the disagreement with the purpose-built error.
        let err = load_state(&conn, &w, &LoadCtx::strict())
            .expect_err("load_state must reject a status column vs blob mismatch");
        assert!(
            matches!(err, WalletStorageError::AssetLockStatusMismatch { .. }),
            "expected AssetLockStatusMismatch, got {err:?}"
        );

        // Recovery keeps consumption sticky even though the blob says Built.
        let ctx = LoadCtx::recovery();
        let state = load_state(&conn, &w, &ctx).expect("recovery tolerates status drift");
        assert_eq!(state[&0][&outpoint].status, AssetLockStatus::Consumed);
        assert_eq!(
            ctx.degradation()
                .by_site
                .get(&LoadSite::AssetLockStatusDrift),
            Some(&1)
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
            AssetLockStatus::RecoveredFromChain,
            AssetLockStatus::Conflicted,
            AssetLockStatus::DrainCompleted,
        ];
        for v in &variants {
            match v {
                AssetLockStatus::Built
                | AssetLockStatus::Broadcast
                | AssetLockStatus::InstantSendLocked
                | AssetLockStatus::ChainLocked
                | AssetLockStatus::Consumed
                | AssetLockStatus::RecoveredFromChain
                | AssetLockStatus::DrainCompleted
                | AssetLockStatus::Conflicted => {}
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
    /// asset-lock migration (`V022__asset_lock_conflict_evidence.rs`).
    /// Shipped migrations interpolate nothing — their generated SQL is
    /// checksummed by Refinery, so widening the domain means APPENDING
    /// a new table-rebuild migration (V018+) with the new frozen list
    /// and updating this pin, never editing V001/V004 in place.
    ///
    /// IF THIS FAILS: do NOT edit a shipped migration (its Refinery
    /// checksum would diverge on already-migrated databases). Append a
    /// new migration that rebuilds `asset_locks` with the widened
    /// CHECK, then update this pin to the new migration's list.
    #[test]
    fn asset_lock_status_labels_frozen_in_latest_migration() {
        let frozen_in_v022 = [
            "built",
            "broadcast",
            "is_locked",
            "chain_locked",
            "consumed",
            "recovered_from_chain",
            "conflicted",
        ];
        assert_eq!(ASSET_LOCK_STATUS_LABELS, &frozen_in_v022);
    }
}
