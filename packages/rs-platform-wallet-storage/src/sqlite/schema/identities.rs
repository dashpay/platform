//! `identities` table writer.

use std::collections::BTreeMap;

use dpp::prelude::Identifier;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use platform_wallet::changeset::IdentityChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

// Imports used only by the test-gated readers below.
#[cfg(any(test, feature = "__test-helpers"))]
use platform_wallet::changeset::IdentityEntry;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityChangeSet,
) -> Result<(), WalletStorageError> {
    // `store` checks the merged buffer before a changeset joins it; this
    // checks what actually reaches disk. Disk state can still move under
    // a buffered changeset — a sibling persister on the same file — and
    // `delete_wallet`'s pre-flush never passes through `store` at all.
    check_index_conflicts(tx, wallet_id, cs)?;
    if !cs.identities.is_empty() {
        // PK is `identity_id` alone; `wallet_id` is nullable and links
        // the identity to its parent wallet for cascade. The all-zero
        // wallet id is treated as "no parent wallet known" and stored
        // as NULL so the FK to `wallet_metadata` doesn't activate.
        //
        // COALESCE order — `COALESCE(identities.wallet_id,
        // excluded.wallet_id)` — preserves an already-parented row's
        // wallet_id on re-upsert; the excluded value only fills when
        // the on-disk row is still NULL. This is the orphan → parented
        // promotion path; the reverse (mismatched re-parent) is caught
        // by the per-entry cross-check below.
        let scope_is_sentinel = wallet_id.iter().all(|b| *b == 0);
        let mut stmt = tx.prepare_cached(
            "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, ?3, ?4, 0) \
             ON CONFLICT(identity_id) DO UPDATE SET \
                wallet_id = COALESCE(identities.wallet_id, excluded.wallet_id), \
                wallet_index = excluded.wallet_index, \
                entry_blob = excluded.entry_blob, \
                tombstoned = 0",
        )?;
        let wallet_id_param = wallet_id_to_param(wallet_id);
        for (id, entry) in &cs.identities {
            // The map key is bound into the `identity_id` column while
            // `entry` is what the serialized blob carries; a disagreement
            // would persist a row whose typed id names a different
            // identity than its blob. Reject before encoding so the two
            // representations can never diverge on disk.
            if entry.id != *id {
                return Err(WalletStorageError::IdentityEntryIdMismatch);
            }
            // Cross-check: the entry's own wallet_id (when set) must
            // agree with the flush scope so the typed columns and the
            // serialized blob describe the same parenting. Sentinel
            // scope ("no parent wallet known") requires the entry's
            // wallet_id to also be `None` — otherwise a real wallet's
            // identity would be written under the orphan slot.
            if let Some(entry_wallet_id) = entry.wallet_id {
                if scope_is_sentinel {
                    return Err(WalletStorageError::WalletIdMismatch {
                        expected: [0u8; 32],
                        found: entry_wallet_id,
                    });
                }
                if entry_wallet_id != *wallet_id {
                    return Err(WalletStorageError::WalletIdMismatch {
                        expected: *wallet_id,
                        found: entry_wallet_id,
                    });
                }
            }
            let payload = blob::encode(entry)?;
            stmt.execute(params![
                id.as_slice(),
                wallet_id_param,
                entry.identity_index.map(i64::from),
                payload,
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        let mut stmt =
            tx.prepare_cached("UPDATE identities SET tombstoned = 1 WHERE identity_id = ?1")?;
        for id in &cs.removed {
            stmt.execute(params![id.as_slice()])?;
        }
    }
    Ok(())
}

/// Refuse a changeset that would put two identities in one wallet's
/// derivation slot, or hand a wallet-less identity a slot at all.
///
/// `identity_index` is an HD derivation-path component, so
/// `(wallet_id, identity_index)` names exactly one identity. A duplicate
/// that reaches disk leaves the displaced identity's keys and contacts
/// without an owner, which the next `load()` reports as fatal for the
/// whole wallet — so the write is rejected while it can still be
/// attributed to the caller that made it.
///
/// Occupancy is keyed on the FLUSH SCOPE, not on the incoming row's
/// stored `wallet_id`: [`apply`]'s upsert promotes a NULL `wallet_id`
/// into the flush scope, so the scope is the slot the write actually
/// lands in. Ids in `cs.removed` hold no slot — [`apply`] inserts before
/// it tombstones, so "tombstone A@N + insert B@N" in one changeset has a
/// legal final state. Tombstoned rows are likewise transparent: the
/// tombstone `UPDATE` leaves `wallet_index` populated, and counting
/// those would refuse legitimate slot reuse.
///
/// A pre-existing on-disk duplicate (written before this check existed)
/// makes both of its slot-mates unwritable here. That wallet already
/// fails to load; refusing to extend the contradiction is the point.
///
/// Scope-keying also covers the promotion case (an existing NULL-parented
/// row being pulled into a wallet that already fills the slot) at no
/// extra cost. That case is defensive only: no production write creates a
/// `wallet_id IS NULL` `identities` row, since the flush scope is always
/// the persister's bound wallet id.
///
/// # Errors
///
/// - [`WalletStorageError::WalletlessIdentityIndex`] — sentinel scope
///   (a NULL `wallet_id` row) combined with an index.
/// - [`WalletStorageError::IdentityIndexConflict`] — the slot is held by
///   a different live identity, on disk or elsewhere in `cs`.
pub(crate) fn check_index_conflicts(
    conn: &Connection,
    wallet_id: &WalletId,
    cs: &IdentityChangeSet,
) -> Result<(), WalletStorageError> {
    if cs.identities.is_empty() {
        return Ok(());
    }
    let scope_is_sentinel = wallet_id.iter().all(|b| *b == 0);
    let wallet_id_param = wallet_id_to_param(wallet_id);
    let mut stmt = conn.prepare_cached(
        "SELECT identity_id FROM identities \
         WHERE wallet_id IS ?1 AND wallet_index = ?2 AND tombstoned = 0 \
           AND identity_id != ?3",
    )?;
    let mut claimed: BTreeMap<u32, Identifier> = BTreeMap::new();
    for (id, entry) in &cs.identities {
        // A removed id is tombstoned by the end of the same `apply`, so
        // whatever it claims here it does not keep.
        if cs.removed.contains(id) {
            continue;
        }
        let Some(index) = entry.identity_index else {
            continue;
        };
        if scope_is_sentinel {
            return Err(WalletStorageError::WalletlessIdentityIndex {
                identity_id: id.to_buffer(),
                identity_index: index,
            });
        }
        if let Some(other) = claimed.insert(index, *id) {
            return Err(WalletStorageError::IdentityIndexConflict {
                wallet_id: *wallet_id,
                identity_index: index,
                existing: other.to_buffer(),
                incoming: id.to_buffer(),
            });
        }
        let occupant: Option<Vec<u8>> = stmt
            .query_row(
                params![wallet_id_param, i64::from(index), id.as_slice()],
                |row| row.get(0),
            )
            .optional()?;
        let Some(occupant) = occupant else { continue };
        let occupant: [u8; 32] = occupant.try_into().map_err(|_| {
            WalletStorageError::blob_decode("identities.identity_id is not 32 bytes")
        })?;
        if cs.removed.contains(&Identifier::from(occupant)) {
            continue;
        }
        return Err(WalletStorageError::IdentityIndexConflict {
            wallet_id: *wallet_id,
            identity_index: index,
            existing: occupant,
            incoming: id.to_buffer(),
        });
    }
    Ok(())
}

/// Map the caller-supplied `WalletId` (32 bytes) to the nullable
/// `identities.wallet_id` column: the all-zero id is treated as "no
/// parent wallet" and stored as NULL so the FK doesn't activate.
fn wallet_id_to_param(wallet_id: &WalletId) -> Option<&[u8]> {
    if wallet_id.iter().all(|b| *b == 0) {
        None
    } else {
        Some(wallet_id.as_slice())
    }
}

/// Decode a single `identities` row into `(entry, tombstoned)`.
///
/// Returns `Ok(None)` if no row matches. The `tombstoned` flag is
/// returned alongside the entry so the caller can decide whether to skip
/// a logically deleted identity rather than having to consult
/// [`load_state`] separately.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn fetch(
    conn: &Connection,
    wallet_id: &WalletId,
    identity_id: &[u8; 32],
) -> Result<Option<(IdentityEntry, bool)>, WalletStorageError> {
    use rusqlite::OptionalExtension;
    // Scope the lookup to the caller's wallet so a peer wallet that
    // happens to share the identity-id row can never leak through.
    // The sentinel WalletId (all zeros) matches orphan rows (NULL
    // wallet_id); a real WalletId matches only that wallet's rows.
    // `IS` is NULL-safe equality so the NULL branch works uniformly.
    let wallet_id_param = wallet_id_to_param(wallet_id);
    let row: Option<(Vec<u8>, i64)> = conn
        .query_row(
            "SELECT entry_blob, tombstoned FROM identities \
             WHERE identity_id = ?1 AND wallet_id IS ?2",
            params![&identity_id[..], wallet_id_param],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((payload, tombstoned)) => Ok(Some((blob::decode(&payload)?, tombstoned != 0))),
    }
}

/// Build a [`platform_wallet::changeset::IdentityManagerStartState`]
/// for one wallet from the `identities` table. Tombstoned rows are skipped (a logical delete,
/// not corruption); any row that fails to decode is a hard error —
/// corruption is never silently dropped.
///
/// The bucket selection mirrors `IdentityManager`'s layout:
/// rows with `IdentityEntry.identity_index = Some(_)` go into
/// `wallet_identities[wallet_id]`; rows with `None` go into
/// `out_of_wallet_identities`.
///
/// Retained for this crate's integration tests until the
/// `Wallet::from_persisted` rehydration path consumes it in `load()`.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    use platform_wallet::changeset::IdentityManagerStartState;

    // `identities.wallet_id` is nullable; this load path wants only the
    // rows belonging to the wallet the caller asked for, so the WHERE
    // clause matches by wallet_id (orphan identities — wallet_id NULL —
    // are out of scope for this per-wallet loader).
    let mut stmt = conn.prepare(
        "SELECT identity_id, entry_blob, tombstoned FROM identities WHERE wallet_id = ?1",
    )?;
    // The ignored-senders TABLE is the authoritative ignore record (every
    // ignore/un-ignore maintains it transactionally); the `entry_blob`'s
    // snapshot copy can be stale — see `contacts::load_ignored_senders`.
    let mut ignored_by_owner =
        crate::sqlite::schema::contacts::load_ignored_senders(conn, wallet_id)?;
    let mut state = IdentityManagerStartState::default();
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let _identity_id: Vec<u8> = row.get(0)?;
        let payload: Vec<u8> = row.get(1)?;
        let tombstoned: i64 = row.get(2)?;
        if tombstoned != 0 {
            continue;
        }
        let entry: IdentityEntry = blob::decode(&payload)?;
        let ignored = ignored_by_owner.remove(&entry.id).unwrap_or_default();
        let managed = managed_identity_from_entry(&entry, wallet_id, ignored);
        match entry.identity_index {
            Some(idx) => {
                state
                    .wallet_identities
                    .entry(*wallet_id)
                    .or_default()
                    .insert(idx, managed);
            }
            None => {
                state.out_of_wallet_identities.insert(entry.id, managed);
            }
        }
    }
    Ok(state)
}

/// Reconstruct a [`ManagedIdentity`] from a persisted [`IdentityEntry`]
/// using a freshly minted V0 [`Identity`] for `(id, balance, revision)`.
/// Live runtime fields (contacts maps, public-key derivations) are
/// recovered separately via the contacts / identity_keys readers.
#[cfg(any(test, feature = "__test-helpers"))]
fn managed_identity_from_entry(
    entry: &IdentityEntry,
    wallet_id: &WalletId,
    ignored_senders: std::collections::BTreeSet<dpp::prelude::Identifier>,
) -> platform_wallet::wallet::identity::ManagedIdentity {
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::Identity;
    use platform_wallet::wallet::identity::ManagedIdentity;
    let identity = Identity::V0(IdentityV0 {
        id: entry.id,
        public_keys: std::collections::BTreeMap::new(),
        balance: entry.balance,
        revision: entry.revision,
    });
    let mut managed = match entry.identity_index {
        Some(index) => ManagedIdentity::new(identity, index),
        None => ManagedIdentity::new_out_of_wallet(identity),
    };
    managed.last_updated_balance_block_time = entry.last_updated_balance_block_time;
    managed.last_synced_keys_block_time = entry.last_synced_keys_block_time;
    managed.status = entry.status;
    managed.dpns_names = entry.dpns_names.clone();
    managed.contested_dpns_names = entry.contested_dpns_names.clone();
    managed.wallet_id = entry.wallet_id.or(Some(*wallet_id));
    // Scalar-snapshot collections ride the identity `entry_blob`
    // (payments / profile / contact_profiles), so they restore from
    // `entry`. The relational request collections are loaded separately
    // from the `contacts` table and stay defaulted here.
    // High-water sync cursors, the per-session rescan guard, the
    // verify-failed auto-accept markers, and the deferred contact-crypto
    // queue (not persisted; a signerless sweep re-enqueues its ops on
    // load) are in-memory by design: a cold restore starts them at their
    // defaults so the next sweep re-fetches / re-evaluates safely.
    //
    // Ignored senders restore from the `ignored_senders` TABLE (passed in
    // by the loader), NOT from `entry.ignored_senders`: an un-ignore
    // deletes only the table row (no fresh identity-entry flush), and the
    // changeset merge UNIONs the blob's set across buffered snapshots —
    // so the blob copy can resurrect an un-ignored sender. The table is
    // maintained transactionally by both the ignore and un-ignore writers
    // and is therefore authoritative. The constructor starts a fresh
    // empty ignored set, so per-element apply reproduces the table's set
    // exactly.
    for sender in &ignored_senders {
        managed.apply_ignored_sender(*sender);
    }
    *managed.dashpay_profile_mut() = entry.dashpay_profile.clone();
    *managed.dashpay_payments_mut() = entry.dashpay_payments.clone();
    *managed.dashpay_contact_profiles_mut() = entry.contact_profiles.clone();
    managed
}

/// Insert a stub identity row so identity_keys / dashpay_profiles can
/// reference it via their native composite FK. Used by tests that exercise
/// identity_keys persistence without going through the full identity
/// flow. The stub row carries a `null`-encoded `IdentityEntry` so the
/// `entry_blob` column always decodes — callers wanting real data
/// overwrite via [`apply`].
#[cfg(any(test, feature = "__test-helpers"))]
pub fn ensure_exists(
    conn: &Connection,
    wallet_id: &WalletId,
    identity_id: &[u8; 32],
) -> Result<(), WalletStorageError> {
    use dpp::prelude::Identifier;
    use platform_wallet::wallet::identity::IdentityStatus;

    let stub = IdentityEntry {
        id: Identifier::from(*identity_id),
        balance: 0,
        revision: 0,
        identity_index: None,
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Unknown,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    };
    let payload = blob::encode(&stub)?;
    let wallet_id_param = wallet_id_to_param(wallet_id);
    conn.execute(
        "INSERT OR IGNORE INTO identities \
            (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, NULL, ?3, 0)",
        params![&identity_id[..], wallet_id_param, payload],
    )?;
    Ok(())
}
