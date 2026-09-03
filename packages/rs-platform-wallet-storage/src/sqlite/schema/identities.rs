//! `identities` table writer.

use std::collections::{BTreeMap, HashMap, HashSet};

use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use platform_wallet::{ContactChangeSet, IdentityKeysChangeSet, ManagedIdentity};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{
    changeset::{IdentityChangeSet, IdentityEntry},
    IdentityManagerStartState,
};

use super::wallet_id_to_param;
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite, SiteCoords};
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: identity snapshot reaching the `entry_blob` column.
impl_persistable_blob!(IdentityEntry);

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
        // COALESCE keeps an already-parented row's wallet_id on re-upsert
        // (excluded fills only when on-disk is NULL): the orphan → parented
        // promotion path. The all-zero sentinel stores NULL (no parent).
        let scope_is_sentinel = wallet_id.iter().all(|b| *b == 0);
        // The DO UPDATE WHERE keeps a wallet-B flush from overwriting wallet
        // A's row: it fires only when the on-disk row is unowned (orphan →
        // parented promotion) or already owned by the incoming scope. A
        // cross-wallet write becomes a no-op (SQLite skips a false-WHERE
        // upsert without erroring), preserving the resident blob, index, and
        // tombstone. `IS` is the NULL-safe match for the nullable column.
        let mut stmt = tx.prepare_cached(
            "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, ?3, ?4, 0) \
             ON CONFLICT(identity_id) DO UPDATE SET \
                wallet_id = COALESCE(identities.wallet_id, excluded.wallet_id), \
                identity_index = excluded.identity_index, \
                entry_blob = excluded.entry_blob, \
                tombstoned = 0 \
             WHERE identities.wallet_id IS NULL OR identities.wallet_id IS excluded.wallet_id",
        )?;
        let wallet_id_param = wallet_id_to_param(wallet_id);
        for (id, entry) in &cs.identities {
            // Typed id column and blob must name the same identity; reject
            // before encoding so the two can never diverge on disk.
            if entry.id != *id {
                return Err(WalletStorageError::IdentityEntryIdMismatch);
            }
            // The entry's wallet_id (when set) must match the flush scope;
            // sentinel scope requires it to be `None`, else a real wallet's
            // identity would land in the orphan slot.
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
        // Carry the promoted identities' NULL-scoped keys into the same
        // scope, inside the promotion's own transaction. A key left
        // NULL-scoped under a now-owned identity is the state the
        // `identity_keys_null_scope_*` triggers forbid, and it drops out of
        // the owner's scoped read. The `EXISTS` clause is what makes this
        // safe to run for every flushed id: it fires only where the identity
        // row really is owned by this scope, so a cross-wallet upsert that
        // the `DO UPDATE WHERE` turned into a no-op moves no keys either.
        if !scope_is_sentinel {
            let mut rescope = tx.prepare_cached(
                "UPDATE identity_keys SET wallet_id = ?1 \
                 WHERE identity_id = ?2 AND wallet_id IS NULL \
                   AND EXISTS (SELECT 1 FROM identities i \
                               WHERE i.identity_id = ?2 AND i.wallet_id IS ?1)",
            )?;
            for id in cs.identities.keys() {
                rescope.execute(params![wallet_id_param, id.as_slice()])?;
            }
        }
    }
    if !cs.removed.is_empty() {
        // Scope the tombstone to the flush wallet (NULL-safe `IS`) so wallet
        // A's `removed` set can't tombstone wallet B's identity; the sentinel
        // scope maps to NULL and tombstones only orphan rows.
        let wallet_id_param = wallet_id_to_param(wallet_id);
        let mut stmt = tx.prepare_cached(
            "UPDATE identities SET tombstoned = 1 WHERE identity_id = ?1 AND wallet_id IS ?2",
        )?;
        for id in &cs.removed {
            stmt.execute(params![id.as_slice(), wallet_id_param])?;
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
/// tombstone `UPDATE` leaves `identity_index` populated, and counting
/// those would refuse legitimate slot reuse.
///
/// The judgement is on the state the changeset ENDS in, never the one it
/// starts from: an on-disk occupant that `cs` itself rewrites to another
/// index — or to none at all — has vacated the slot as surely as a
/// tombstoned one, so "A moves to 2, B takes 1" and a two-way swap are
/// both legal. `identities` carries no `(wallet_id, identity_index)`
/// UNIQUE index, so [`apply`]'s row-at-a-time upserts pass straight
/// through the transient double-claim a swap goes through.
///
/// A pre-existing on-disk duplicate (written before this check existed)
/// makes both of its slot-mates unwritable here. Refusing to extend the
/// contradiction is the point: a strict load rejects that wallet outright
/// and a `Recovery` load counts the collision and drops one identity from
/// the slot, so the duplicate costs the user an identity either way.
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
         WHERE wallet_id IS ?1 AND identity_index = ?2 AND tombstoned = 0 \
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
        let occupant_id = Identifier::from(occupant);
        if cs.removed.contains(&occupant_id) {
            continue;
        }
        // The occupant is rewritten by this same changeset, somewhere
        // other than here: `apply` moves it out of this slot (to another
        // index, or to none at all) in the same transaction, so by the
        // time the changeset lands it holds no claim on `index`.
        if cs
            .identities
            .get(&occupant_id)
            .is_some_and(|entry| entry.identity_index != Some(index))
        {
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
    // Scope to the caller's wallet (NULL-safe `IS`) so a peer wallet sharing
    // the identity-id row can't leak through; sentinel matches orphan rows.
    let wallet_id_param = wallet_id_to_param(wallet_id);
    let mut stmt = conn.prepare(
        "SELECT length(entry_blob), entry_blob, tombstoned FROM identities \
         WHERE identity_id = ?1 AND wallet_id IS ?2",
    )?;
    let mut rows = stmt.query(params![&identity_id[..], wallet_id_param])?;
    match rows.next()? {
        None => Ok(None),
        Some(row) => {
            blob::check_size(row.get::<_, i64>(0)?)?;
            let payload: Vec<u8> = row.get(1)?;
            let tombstoned: i64 = row.get(2)?;
            Ok(Some((blob::decode(&payload)?, tombstoned != 0)))
        }
    }
}

/// Build an [`IdentityManagerStartState`](platform_wallet::changeset::IdentityManagerStartState)
/// for one wallet. Tombstoned rows are skipped; a row that fails to decode is
/// a hard error (corruption is never silently dropped). Rows with
/// `identity_index = Some(_)` bucket into `wallet_identities`, `None` into
/// `out_of_wallet_identities`.
/// Strict-policy [`load_state_with_ctx`], for the tests that read identity
/// state directly rather than through `load_prekeyed`.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    // `LoadCtx::strict()` is cfg-gated to test/helper builds; the policy
    // constructor is not, so the plain library build keeps compiling.
    load_state_with_ctx(
        conn,
        wallet_id,
        &LoadCtx::new(crate::sqlite::config::LoadPolicy::Strict),
    )
}

/// [`load_state`] under an explicit load policy.
///
/// Only the duplicate-slot case consults `ctx`: a second live row claiming
/// an `identity_index` this wallet has already filled is fatal under
/// `Strict` and a counted, logged drop under `Recovery`. The typed-column
/// cross-checks stay unconditional — a row whose columns contradict its own
/// blob is corruption, not a recoverable projection. Rows are read by
/// `identity_id` ascending, so the lexicographically higher id wins a collision.
pub fn load_state_with_ctx(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    use platform_wallet::changeset::IdentityManagerStartState;

    // Per-wallet loader. NULL-safe `IS` so the scope reads exactly the way
    // it writes: a real wallet id behaves identically to `=` and never sees
    // unowned rows, while the all-zero sentinel maps to NULL and reads the
    // unowned bucket. A plain `=` could not express the second case at all.
    let wallet_id_param = wallet_id_to_param(wallet_id);
    let mut stmt = conn.prepare(
        "SELECT identity_id, length(entry_blob), entry_blob, tombstoned, identity_index \
         FROM identities WHERE wallet_id IS ?1 ORDER BY identity_id",
    )?;
    // The ignored-senders TABLE is the authoritative ignore record (every
    // ignore/un-ignore maintains it transactionally); the `entry_blob`'s
    // snapshot copy can be stale — see `contacts::load_ignored_senders`.
    let mut ignored_by_owner =
        crate::sqlite::schema::contacts::load_ignored_senders(conn, wallet_id)?;
    let mut state = IdentityManagerStartState::default();
    let mut rows = stmt.query(params![wallet_id_param])?;
    while let Some(row) = rows.next()? {
        let identity_id_bytes: Vec<u8> = row.get(0)?;
        blob::check_size(row.get::<_, i64>(1)?)?;
        let payload: Vec<u8> = row.get(2)?;
        let tombstoned: i64 = row.get(3)?;
        let typed_identity_index: Option<i64> = row.get(4)?;
        if tombstoned != 0 {
            continue;
        }
        let entry: IdentityEntry = blob::decode(&payload)?;
        // Cross-check the decoded blob against the typed columns it was
        // selected by (mirrors the accounts / identity_keys readers): the
        // blob must name the same identity, and its own wallet_id (when set)
        // must match the wallet scope, else the row is corrupt / mis-filed.
        let typed_id = super::id32("identities.identity_id", &identity_id_bytes)?;
        if entry.id != dpp::prelude::Identifier::from(typed_id) {
            return Err(WalletStorageError::IdentityEntryIdMismatch);
        }
        if let Some(entry_wallet_id) = entry.wallet_id {
            if entry_wallet_id != *wallet_id {
                return Err(WalletStorageError::WalletIdMismatch {
                    expected: *wallet_id,
                    found: entry_wallet_id,
                });
            }
        }
        // The blob's index is what the wallet runs on; the column is what
        // the write-path slot guard (`check_index_conflicts`) reasons about.
        // Divergence means the guard has been policing a value the runtime
        // never sees, so the two must be proven equal on the way in.
        let typed_identity_index = typed_identity_index
            .map(|raw| crate::sqlite::util::safe_cast::i64_to_u32("identities.identity_index", raw))
            .transpose()?;
        if entry.identity_index != typed_identity_index {
            return Err(WalletStorageError::IdentityEntryIdMismatch);
        }
        let ignored = ignored_by_owner.remove(&entry.id).unwrap_or_default();
        let managed = managed_identity_from_entry(&entry, wallet_id, ignored);
        match entry.identity_index {
            Some(idx) => {
                let entry_id = entry.id;
                if let Some(displaced) = state
                    .wallet_identities
                    .entry(*wallet_id)
                    .or_default()
                    .insert(idx, managed)
                {
                    // `identities` carries no `(wallet_id, identity_index)`
                    // UNIQUE index, so a pre-guard duplicate can still be on
                    // disk. Nothing in the persisted rows establishes which
                    // identity genuinely holds `idx`, so the slot winner is
                    // arbitrary on the merits — which is exactly why the loser
                    // MUST NOT be dropped. Park it in
                    // `out_of_wallet_identities` (the same bucket a NULL
                    // `identity_index` row lands in) so a Recovery load hands
                    // the caller every identity it read, slot or no slot. The
                    // persister is read-only in Recovery, so anything dropped
                    // here could never be recovered from a later flush.
                    let displaced_identity_id = displaced.identity.id();
                    state
                        .out_of_wallet_identities
                        .insert(displaced_identity_id, displaced);
                    ctx.tolerate(
                        LoadSite::IdentityIndexCollision,
                        WalletStorageError::IdentityIndexConflict {
                            wallet_id: *wallet_id,
                            identity_index: idx,
                            existing: displaced_identity_id.to_buffer(),
                            incoming: entry_id.to_buffer(),
                        },
                    )?;
                }
            }
            None => {
                state.out_of_wallet_identities.insert(entry.id, managed);
            }
        }
    }
    Ok(state)
}

/// Build a fully pre-keyed
/// [`IdentityManagerStartState`](platform_wallet::changeset::IdentityManagerStartState)
/// for one wallet: read the identities, then fold this wallet's persisted
/// identity keys and contacts onto them so every `ManagedIdentity` carries
/// its own `public_keys` and contact maps at load time — no separate
/// changeset layered on afterwards. Fail-hard on a corrupt row (inherited
/// from the three underlying readers) and on any merged key / contact entry
/// whose owner is absent for a reason other than a known tombstone; a
/// tombstoned owner's orphaned rows are skipped with a summary log (see
/// [`merge_contacts_and_keys`]).
pub fn load_prekeyed(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    let mut state = load_state_with_ctx(conn, wallet_id, ctx)?;
    let identity_keys = crate::sqlite::schema::identity_keys::load_state(conn, wallet_id)?;
    let records = crate::sqlite::schema::contacts::load_state(conn, wallet_id)?;
    // Ignored senders restore in `load_state` from the authoritative
    // `ignored_senders` table, so only the request / established maps ride
    // this changeset; `removed_*` / `ignored` / `unignored` stay empty.
    let contacts = platform_wallet::changeset::ContactChangeSet {
        sent_requests: records.sent_requests,
        incoming_requests: records.incoming_requests,
        established: records.established,
        ..Default::default()
    };
    let tombstoned = load_tombstoned_ids(conn, wallet_id)?;
    merge_contacts_and_keys(
        &mut state,
        contacts,
        identity_keys,
        &tombstoned,
        *wallet_id,
        ctx,
    )?;
    // The scan verdict rides the same per-wallet start state the identities
    // do, because it is the fact the startup sequence weighs against them:
    // "we already have one" is not evidence we have them all unless the scan
    // behind it answered every index (dashpay/platform#4365).
    if let Some(scan_state) =
        crate::sqlite::schema::identity_scan_states::load_for_wallet(conn, wallet_id, ctx)?
    {
        state.scan_states.insert(*wallet_id, scan_state);
    }
    Ok(state)
}

/// The set of identity ids tombstoned (logically deleted) for this wallet.
/// A rehydration-merge entry whose owner is in this set is an expected
/// logical-delete orphan — safe to skip; an owner absent for any other
/// reason is a hard error.
fn load_tombstoned_ids(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<HashSet<Identifier>, WalletStorageError> {
    // NULL-safe `IS`, matching `load_state`: a tombstoned UNOWNED identity
    // must be recognised as tombstoned too, else its leftover key rows are
    // treated as inexplicable orphans and hard-error the read.
    let wallet_id_param = wallet_id_to_param(wallet_id);
    let mut stmt = conn
        .prepare("SELECT identity_id FROM identities WHERE wallet_id IS ?1 AND tombstoned = 1")?;
    let mut rows = stmt.query(params![wallet_id_param])?;
    let mut out = HashSet::new();
    while let Some(row) = rows.next()? {
        let id_bytes: Vec<u8> = row.get(0)?;
        let id32 = super::id32("identities.identity_id", &id_bytes)?;
        out.insert(Identifier::from(id32));
    }
    Ok(out)
}

/// Reconstruct a [`ManagedIdentity`] from a persisted [`IdentityEntry`]
/// using a freshly minted V0 [`Identity`] for `(id, balance, revision)`.
/// Live runtime fields (contacts maps, public-key derivations) are
/// recovered separately via the contacts / identity_keys readers.
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
    // Fall back to the reading scope, EXCEPT for the sentinel: an identity
    // read from the unowned bucket must come back with `wallet_id: None`,
    // not `Some([0; 32])`. The sentinel is a storage spelling of "no
    // wallet", never a wallet id, and handing it out would put a value
    // that looks like an owner onto an identity that has none.
    managed.wallet_id = entry
        .wallet_id
        .or_else(|| (!wallet_id.iter().all(|b| *b == 0)).then_some(*wallet_id));
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

/// Insert a stub identity row (test helper) so identity_keys /
/// dashpay_profiles can reference it via their FK. The stub carries a
/// `null`-encoded `IdentityEntry` so `entry_blob` always decodes; real data
/// overwrites via [`apply`].
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
            (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, NULL, ?3, 0)",
        params![&identity_id[..], wallet_id_param, payload],
    )?;
    Ok(())
}

/// Fold persisted PUBLIC keys and contact state onto the already-built
/// managed identities so `Identity.public_keys` and the contact maps
/// are populated at load time — the FFI persister's pre-keyed shape,
/// with no separate changeset layered on afterwards.
///
/// Entries route by owner `identity_id` across BOTH buckets. Only key
/// `upserts` and the `sent` / `incoming` / `established` maps are routed;
/// `removed_*` (insert-only feed) and `ignored` / `unignored` (restored in
/// the identity reader from the `ignored_senders` table) are skipped. No
/// `Network` needed — key insert is network-independent.
///
/// # Errors
///
/// [`WalletStorageError::OrphanedIdentityEntry`] when an entry's owner is
/// absent from the loaded set. A known-tombstoned owner is the one case
/// [`LoadPolicy::Recovery`](crate::LoadPolicy) forgives: its rows are
/// logical-delete leftovers, skipped and summarised once per collection.
/// Under `Strict` even those abort, because "the owner is gone" is exactly
/// the state that silently drops live key / contact material.
pub fn merge_contacts_and_keys(
    state: &mut IdentityManagerStartState,
    contacts: ContactChangeSet,
    identity_keys: IdentityKeysChangeSet,
    tombstoned: &HashSet<Identifier>,
    wallet_id: WalletId,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    // One transient id → &mut ManagedIdentity view over both buckets so
    // routing is O(1) per entry rather than a per-entry bucket scan. The
    // two buckets are disjoint fields, so their mutable borrows coexist.
    let mut by_id: HashMap<Identifier, &mut ManagedIdentity> = HashMap::new();
    for managed in state.out_of_wallet_identities.values_mut() {
        by_id.insert(managed.identity.id(), managed);
    }
    for inner in state.wallet_identities.values_mut() {
        for managed in inner.values_mut() {
            by_id.insert(managed.identity.id(), managed);
        }
    }

    route_by_owner(
        identity_keys
            .upserts
            .into_values()
            .map(|entry| (entry.identity_id, entry.public_key)),
        &mut by_id,
        tombstoned,
        wallet_id,
        ctx,
        "identity_keys",
        |managed, key| managed.identity.add_public_key(key),
    )?;
    route_by_owner(
        contacts
            .sent_requests
            .into_iter()
            .map(|(key, entry)| (key.owner_id, entry.request)),
        &mut by_id,
        tombstoned,
        wallet_id,
        ctx,
        "sent_contact_requests",
        |managed, request| managed.apply_sent_contact_request(request),
    )?;
    route_by_owner(
        contacts
            .incoming_requests
            .into_iter()
            .map(|(key, entry)| (key.owner_id, entry.request)),
        &mut by_id,
        tombstoned,
        wallet_id,
        ctx,
        "incoming_contact_requests",
        |managed, request| managed.apply_incoming_contact_request(request),
    )?;
    route_by_owner(
        contacts
            .established
            .into_iter()
            .map(|(key, established)| (key.owner_id, established)),
        &mut by_id,
        tombstoned,
        wallet_id,
        ctx,
        "established_contacts",
        |managed, established| managed.apply_established_contact(established),
    )?;

    Ok(())
}

/// Apply one collection's entries to their owning identity.
///
/// An entry whose owner is loaded is applied. An entry whose owner is
/// tombstoned is counted and, if the policy allows it, skipped — decided
/// once after the walk rather than per entry, so a wallet with thousands of
/// leftovers produces one log line, not thousands, while the per-site tally
/// still counts every skipped row. Any other missing owner is fatal in both
/// policies.
fn route_by_owner<T>(
    entries: impl IntoIterator<Item = (Identifier, T)>,
    by_id: &mut HashMap<Identifier, &mut ManagedIdentity>,
    tombstoned: &HashSet<Identifier>,
    wallet_id: WalletId,
    ctx: &LoadCtx,
    collection: &'static str,
    apply: impl Fn(&mut ManagedIdentity, T),
) -> Result<(), WalletStorageError> {
    let mut skipped = 0usize;
    let mut first_skipped_owner: Option<Identifier> = None;
    for (owner, payload) in entries {
        match by_id.get_mut(&owner) {
            Some(managed) => apply(managed, payload),
            None if tombstoned.contains(&owner) => {
                skipped += 1;
                first_skipped_owner.get_or_insert(owner);
            }
            None => {
                return Err(WalletStorageError::OrphanedIdentityEntry {
                    owner: owner.to_buffer(),
                })
            }
        }
    }
    if let Some(owner) = first_skipped_owner {
        ctx.tolerate_at(
            LoadSite::TombstonedIdentityOrphan,
            SiteCoords {
                wallet_id: Some(wallet_id),
                account_type: &"identity",
                affected: skipped,
                detail: Some(&collection),
            },
            WalletStorageError::OrphanedIdentityEntry {
                owner: owner.to_buffer(),
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use platform_wallet::changeset::IdentityChangeSet;
    use platform_wallet::wallet::identity::IdentityStatus;

    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn
    }

    fn insert_wallet(conn: &Connection, wallet: &[u8; 32]) {
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet[..]],
        )
        .unwrap();
    }

    fn entry(
        id: [u8; 32],
        wallet_id: Option<[u8; 32]>,
        balance: u64,
        index: Option<u32>,
    ) -> IdentityEntry {
        IdentityEntry {
            id: Identifier::from(id),
            balance,
            revision: 0,
            identity_index: index,
            last_updated_balance_block_time: None,
            last_synced_keys_block_time: None,
            dpns_names: Vec::new(),
            contested_dpns_names: Vec::new(),
            status: IdentityStatus::Unknown,
            wallet_id,
            dashpay_profile: None,
            dashpay_payments: Default::default(),
            contact_profiles: Default::default(),
            ignored_senders: Default::default(),
        }
    }

    fn apply_in_tx(conn: &mut Connection, scope: &[u8; 32], cs: &IdentityChangeSet) {
        let tx = conn.transaction().unwrap();
        apply(&tx, scope, cs).unwrap();
        tx.commit().unwrap();
    }

    /// A wallet-B flush naming an identity already owned by wallet A must NOT
    /// overwrite A's blob / index or clear A's tombstone — the DO UPDATE WHERE
    /// scopes the overwrite to the owning wallet, so the cross-wallet write is
    /// a no-op.
    #[test]
    fn cross_wallet_upsert_does_not_overwrite_resident_row() {
        let mut conn = migrated_conn();
        let a = [0xA1u8; 32];
        let b = [0xB2u8; 32];
        let x = [0x01u8; 32];
        insert_wallet(&conn, &a);
        insert_wallet(&conn, &b);

        // A registers X (balance 1000, index 5), then tombstones it.
        let mut cs_a = IdentityChangeSet::default();
        cs_a.identities
            .insert(Identifier::from(x), entry(x, Some(a), 1000, Some(5)));
        apply_in_tx(&mut conn, &a, &cs_a);
        let mut cs_a_remove = IdentityChangeSet::default();
        cs_a_remove.removed.insert(Identifier::from(x));
        apply_in_tx(&mut conn, &a, &cs_a_remove);

        // B flushes X (balance 2000, index 9, unowned blob). Must be a no-op.
        let mut cs_b = IdentityChangeSet::default();
        cs_b.identities
            .insert(Identifier::from(x), entry(x, None, 2000, Some(9)));
        apply_in_tx(&mut conn, &b, &cs_b);

        let (resident, tombstoned) = fetch(&conn, &a, &x).unwrap().expect("A still owns the row");
        assert_eq!(resident.balance, 1000, "A's blob must survive B's write");
        assert_eq!(resident.identity_index, Some(5), "A's index must survive");
        assert!(tombstoned, "A's tombstone must not be reset by B");
        assert!(
            fetch(&conn, &b, &x).unwrap().is_none(),
            "B must not have taken ownership"
        );
    }

    /// The WHERE still permits the orphan → parented promotion path: an
    /// unowned (NULL wallet_id) row is claimed by the first wallet to flush it.
    #[test]
    fn orphan_promotion_still_applies() {
        let mut conn = migrated_conn();
        let a = [0xA1u8; 32];
        let y = [0x02u8; 32];
        insert_wallet(&conn, &a);

        // Orphan Y under the sentinel scope (NULL wallet_id).
        let mut cs_orphan = IdentityChangeSet::default();
        cs_orphan
            .identities
            .insert(Identifier::from(y), entry(y, None, 10, None));
        apply_in_tx(&mut conn, &[0u8; 32], &cs_orphan);
        assert!(
            fetch(&conn, &a, &y).unwrap().is_none(),
            "Y starts unowned by A"
        );

        // A claims Y (balance 500, index 3).
        let mut cs_a = IdentityChangeSet::default();
        cs_a.identities
            .insert(Identifier::from(y), entry(y, Some(a), 500, Some(3)));
        apply_in_tx(&mut conn, &a, &cs_a);

        let (claimed, _) = fetch(&conn, &a, &y).unwrap().expect("A claimed Y");
        assert_eq!(claimed.balance, 500, "promotion applies the new blob");
        assert_eq!(claimed.identity_index, Some(3));
    }

    /// `load_prekeyed` folds each identity's persisted keys onto it across
    /// BOTH buckets — a wallet-owned identity (`identity_index = Some`) and
    /// an out-of-wallet one (`identity_index = None`) each receive their own
    /// key, with no cross-attribution.
    #[test]
    fn load_prekeyed_populates_keys_in_both_buckets() {
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use platform_wallet::changeset::{IdentityKeyEntry, IdentityKeysChangeSet};

        let mut conn = migrated_conn();
        let w = [0x0Au8; 32];
        insert_wallet(&conn, &w);

        let wallet_owned = Identifier::from([0x11u8; 32]);
        let out_of_wallet = Identifier::from([0x22u8; 32]);

        let mut ids = IdentityChangeSet::default();
        ids.identities
            .insert(wallet_owned, entry([0x11; 32], Some(w), 100, Some(0)));
        ids.identities
            .insert(out_of_wallet, entry([0x22; 32], Some(w), 200, None));

        let key = |id: Identifier, byte: u8| IdentityKeyEntry {
            identity_id: id,
            key_id: 0,
            public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 0,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![byte; 33]),
                disabled_at: None,
            }),
            public_key_hash: [byte; 20],
            wallet_id: None,
            derivation_indices: None,
        };
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts
            .insert((wallet_owned, 0), key(wallet_owned, 0xA1));
        keys.upserts
            .insert((out_of_wallet, 0), key(out_of_wallet, 0xB2));

        let tx = conn.transaction().unwrap();
        apply(&tx, &w, &ids).unwrap();
        crate::sqlite::schema::identity_keys::apply(&tx, &w, &keys).unwrap();
        tx.commit().unwrap();

        let state = load_prekeyed(&conn, &w, &LoadCtx::strict()).unwrap();
        let wo = &state.wallet_identities[&w][&0];
        assert_eq!(
            wo.identity.public_keys()[&0].data().as_slice(),
            &[0xA1; 33],
            "wallet-owned identity carries its own key"
        );
        let oow = &state.out_of_wallet_identities[&out_of_wallet];
        assert_eq!(
            oow.identity.public_keys()[&0].data().as_slice(),
            &[0xB2; 33],
            "out-of-wallet identity carries its own key"
        );
    }

    /// Promoting an orphan identity into a wallet must carry its
    /// NULL-scoped `identity_keys` rows across in the same transaction.
    /// Left behind, those keys stay NULL-scoped while their identity is
    /// wallet-owned — a state the `identity_keys_null_scope_*` triggers
    /// exist to forbid, and one that hides the keys from the promoted
    /// identity's own scoped read.
    #[test]
    fn orphan_promotion_rescopes_the_identitys_null_scoped_keys() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let a = [0xA1u8; 32];
        let y = [0x02u8; 32];
        insert_wallet(&conn, &a);

        // Orphan Y under the sentinel scope, then give it a NULL-scoped key.
        let mut cs_orphan = IdentityChangeSet::default();
        cs_orphan
            .identities
            .insert(Identifier::from(y), entry(y, None, 10, None));
        apply_in_tx(&mut conn, &[0u8; 32], &cs_orphan);

        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert(
            (Identifier::from(y), 0),
            sample_key_entry(Identifier::from(y), 0x09),
        );
        let tx = conn.transaction().unwrap();
        crate::sqlite::schema::identity_keys::apply(&tx, &[0u8; 32], &keys).unwrap();
        tx.commit().unwrap();

        // A claims Y.
        let mut cs_a = IdentityChangeSet::default();
        cs_a.identities
            .insert(Identifier::from(y), entry(y, Some(a), 500, Some(3)));
        apply_in_tx(&mut conn, &a, &cs_a);

        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys \
                 WHERE identity_id = ?1 AND wallet_id IS NULL",
                params![&y[..]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            orphaned, 0,
            "the promoted identity's keys must not stay NULL-scoped"
        );

        let rescoped: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys \
                 WHERE identity_id = ?1 AND wallet_id IS ?2",
                params![&y[..], &a[..]],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rescoped, 1, "the key must move to the promoting wallet");
    }

    /// A NULL-scoped key must name an identity that actually exists. V001's
    /// guard only rejected keys whose identity was wallet-OWNED, so one
    /// naming no identity at all slipped through — MATCH SIMPLE leaves both
    /// foreign keys dormant on a NULL-scoped row, making the trigger the
    /// only guard there is. Closed by V014.
    #[test]
    fn null_scoped_key_is_rejected_for_a_missing_identity() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let ghost = Identifier::from([0x5Eu8; 32]); // never written to `identities`

        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts
            .insert((ghost, 0), sample_key_entry(ghost, 0x5E));

        let tx = conn.transaction().unwrap();
        let err = crate::sqlite::schema::identity_keys::apply(&tx, &[0u8; 32], &keys)
            .expect_err("a NULL-scoped key naming no identity must be rejected");
        assert!(
            matches!(err, WalletStorageError::IdentityKeyWalletMismatch { .. }),
            "expected IdentityKeyWalletMismatch from the NULL-scope trigger, got {err:?}"
        );
    }

    /// Two live rows of one wallet claiming the same `identity_index` must
    /// not resolve by silently dropping whichever the scan reached first.
    /// `identities` has no `(wallet_id, identity_index)` UNIQUE index, so a
    /// duplicate written before the write-path slot guard existed is still
    /// reachable on disk.
    #[test]
    fn load_state_reports_a_duplicate_identity_index_instead_of_dropping_one() {
        let conn = migrated_conn();
        let wallet = [7u8; 32];
        insert_wallet(&conn, &wallet);
        for id_byte in [0xAAu8, 0xBBu8] {
            let e = entry([id_byte; 32], Some(wallet), 100, Some(1));
            let payload = blob::encode(&e).unwrap();
            conn.execute(
                "INSERT INTO identities \
                 (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
                 VALUES (?1, ?2, 1, ?3, 0)",
                params![&[id_byte; 32][..], &wallet[..], payload],
            )
            .unwrap();
        }

        // Strict: the contradiction is fatal rather than silently resolved.
        let err = load_state_with_ctx(&conn, &wallet, &LoadCtx::strict())
            .expect_err("a duplicated derivation slot must fail a strict load");
        assert!(
            matches!(err, WalletStorageError::IdentityIndexConflict { .. }),
            "expected IdentityIndexConflict, got {err:?}"
        );

        // Recovery: still degraded, but counted and logged rather than silent.
        let ctx = LoadCtx::recovery();
        let state = load_state_with_ctx(&conn, &wallet, &ctx)
            .expect("recovery tolerates the duplicate to get the wallet open");
        assert_eq!(
            state.wallet_identities.get(&wallet).map(|m| m.len()),
            Some(1),
            "one identity still loses the slot — the point is that it is reported"
        );
        let degradation = ctx.degradation();
        assert!(degradation.degraded);
        assert_eq!(
            degradation.by_site.get(&LoadSite::IdentityIndexCollision),
            Some(&1),
            "the collision must be counted against its own site"
        );
    }

    /// The `identity_index` COLUMN and the index inside `entry_blob` are two
    /// copies of one fact. The write-path slot guard polices the column while
    /// the wallet runs on the blob, so a load must refuse a row where they
    /// disagree rather than let the guard protect a value nothing reads.
    #[test]
    fn load_state_rejects_identity_index_column_drift() {
        let conn = migrated_conn();
        let wallet = [8u8; 32];
        insert_wallet(&conn, &wallet);
        let e = entry([0xC1u8; 32], Some(wallet), 100, Some(2));
        let payload = blob::encode(&e).unwrap();
        conn.execute(
            "INSERT INTO identities \
             (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, 9, ?3, 0)",
            params![&[0xC1u8; 32][..], &wallet[..], payload],
        )
        .unwrap();

        let err =
            load_state(&conn, &wallet).expect_err("column/blob index drift must fail the load");
        assert!(
            matches!(err, WalletStorageError::IdentityEntryIdMismatch),
            "expected IdentityEntryIdMismatch, got {err:?}"
        );
    }

    fn sample_key_entry(id: Identifier, byte: u8) -> platform_wallet::changeset::IdentityKeyEntry {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        platform_wallet::changeset::IdentityKeyEntry {
            identity_id: id,
            key_id: 0,
            public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: 0,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                contract_bounds: None,
                key_type: KeyType::ECDSA_SECP256K1,
                read_only: false,
                data: BinaryData::new(vec![byte; 33]),
                disabled_at: None,
            }),
            public_key_hash: [byte; 20],
            wallet_id: None,
            derivation_indices: None,
        }
    }

    /// An `identity_keys` write naming an identity owned by a DIFFERENT
    /// wallet is rejected at write time by the compound FK. This is where
    /// the guarantee now lives: the unreadable row never reaches disk, so
    /// the load-time orphan check can't be reached by this route.
    #[test]
    fn identity_key_write_is_rejected_for_a_non_owning_wallet() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let a = [0xA1u8; 32];
        let b = [0xB2u8; 32];
        insert_wallet(&conn, &a);
        insert_wallet(&conn, &b);

        // Identity X is parented to wallet B.
        let x = Identifier::from([0x33u8; 32]);
        let mut ids_b = IdentityChangeSet::default();
        ids_b
            .identities
            .insert(x, entry([0x33; 32], Some(b), 100, Some(0)));
        apply_in_tx(&mut conn, &b, &ids_b);

        // Filing X's key under wallet A must fail — A does not own X.
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((x, 0), sample_key_entry(x, 0xC3));
        {
            let tx = conn.transaction().unwrap();
            let err = crate::sqlite::schema::identity_keys::apply(&tx, &a, &keys)
                .expect_err("a key for a non-owning wallet must be rejected");
            assert!(
                matches!(
                    err,
                    WalletStorageError::IdentityKeyWalletMismatch {
                        wallet_id, identity_id, ..
                    } if wallet_id == a && identity_id == x.to_buffer()
                ),
                "expected IdentityKeyWalletMismatch naming wallet A and identity X, got {err:?}"
            );
        }

        // Nothing was written: A's load is clean rather than fatally orphaned.
        let state =
            load_prekeyed(&conn, &a, &LoadCtx::strict()).expect("no orphan row was ever created");
        assert!(state
            .wallet_identities
            .get(&a)
            .is_none_or(|inner| inner.is_empty()));
    }

    /// The reported top-up corruption, at the storage seam: identity owned
    /// by wallet A, its keys flushed under wallet B's scope. Before the
    /// compound FK these rows landed silently and bricked the next load;
    /// now the write is refused and the file stays loadable.
    #[test]
    fn cross_wallet_key_flush_cannot_brick_a_wallet_file() {
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let owner = [0xFAu8; 32];
        let payer = [0xC5u8; 32];
        insert_wallet(&conn, &owner);
        insert_wallet(&conn, &payer);

        let identity = Identifier::from([0xA7u8; 32]);
        let mut ids = IdentityChangeSet::default();
        ids.identities
            .insert(identity, entry([0xA7; 32], Some(owner), 500, Some(0)));
        let mut owner_keys = IdentityKeysChangeSet::default();
        owner_keys
            .upserts
            .insert((identity, 0), sample_key_entry(identity, 0x11));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &owner, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &owner, &owner_keys).unwrap();
            tx.commit().unwrap();
        }

        // The payer wallet re-files the same keys under its own scope.
        let mut payer_keys = IdentityKeysChangeSet::default();
        payer_keys
            .upserts
            .insert((identity, 0), sample_key_entry(identity, 0x11));
        {
            let tx = conn.transaction().unwrap();
            let err = crate::sqlite::schema::identity_keys::apply(&tx, &payer, &payer_keys)
                .expect_err("the paying wallet does not own the identity");
            assert!(
                matches!(err, WalletStorageError::IdentityKeyWalletMismatch { .. }),
                "expected IdentityKeyWalletMismatch, got {err:?}"
            );
        }

        // Both wallets still load; the owner keeps its key.
        let owner_state =
            load_prekeyed(&conn, &owner, &LoadCtx::strict()).expect("owner wallet still loads");
        assert_eq!(
            owner_state.wallet_identities[&owner][&0]
                .identity
                .public_keys()[&0]
                .data()
                .as_slice(),
            &[0x11; 33]
        );
        load_prekeyed(&conn, &payer, &LoadCtx::strict()).expect("payer wallet still loads");
    }

    /// The cross-wallet write refused above must stay refused on the
    /// UPSERT path specifically. With `PRIMARY KEY (identity_id, key_id)`
    /// a foreign wallet's write for an existing key no longer arrives as
    /// an INSERT — it collides and resolves to `DO UPDATE`, and an UPDATE
    /// that leaves `wallet_id` alone violates no foreign key. The upsert
    /// therefore assigns `wallet_id = excluded.wallet_id` so the mismatch
    /// still trips the compound FK. Drop that one clause and this test
    /// fails: the write returns `Ok` and the payer's key material
    /// silently replaces the owner's under the owner's own scope.
    #[test]
    fn identity_key_upsert_cannot_overwrite_another_wallets_key() {
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let owner = [0x0Eu8; 32];
        let payer = [0x0Fu8; 32];
        insert_wallet(&conn, &owner);
        insert_wallet(&conn, &payer);

        // Owner holds identity X and its key 0, carrying `0x11` material.
        let x = Identifier::from([0xE1u8; 32]);
        let mut ids = IdentityChangeSet::default();
        ids.identities
            .insert(x, entry([0xE1; 32], Some(owner), 700, Some(0)));
        let mut owner_keys = IdentityKeysChangeSet::default();
        owner_keys.upserts.insert((x, 0), sample_key_entry(x, 0x11));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &owner, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &owner, &owner_keys).unwrap();
            tx.commit().unwrap();
        }

        // The payer re-files THE SAME `(identity_id, key_id)` with
        // different material — the exact key collision the narrowed
        // primary key turns into an update rather than an insert.
        let mut payer_keys = IdentityKeysChangeSet::default();
        payer_keys.upserts.insert((x, 0), sample_key_entry(x, 0x22));
        {
            let tx = conn.transaction().unwrap();
            let err = crate::sqlite::schema::identity_keys::apply(&tx, &payer, &payer_keys)
                .expect_err("an upsert onto another wallet's key must be rejected");
            assert!(
                matches!(
                    err,
                    WalletStorageError::IdentityKeyWalletMismatch {
                        wallet_id, identity_id, ..
                    } if wallet_id == payer && identity_id == x.to_buffer()
                ),
                "expected IdentityKeyWalletMismatch naming the payer and identity X, got {err:?}"
            );
        }

        // Distinct material on each side, so this assertion catches a
        // silent overwrite as well as a missing error.
        let owner_state =
            load_prekeyed(&conn, &owner, &LoadCtx::strict()).expect("owner wallet still loads");
        assert_eq!(
            owner_state.wallet_identities[&owner][&0]
                .identity
                .public_keys()[&0]
                .data()
                .as_slice(),
            &[0x11; 33],
            "the owner's key material must survive the payer's upsert"
        );
    }

    /// The unowned scope round-trips: a key written under the all-zero
    /// sentinel lands with a genuine SQL NULL `wallet_id` (not 32 zero
    /// bytes) against an identity that is itself unowned. Both foreign
    /// keys are dormant here — MATCH SIMPLE skips enforcement once any
    /// child key column is NULL — so this is the case the guards must
    /// permit rather than the case they catch.
    #[test]
    fn null_scoped_key_is_accepted_for_an_unowned_identity() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let unowned = [0u8; 32];

        // Identity Z exists with no owning wallet.
        let z = Identifier::from([0x5Au8; 32]);
        let mut ids = IdentityChangeSet::default();
        ids.identities.insert(z, entry([0x5A; 32], None, 10, None));
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((z, 0), sample_key_entry(z, 0x5B));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &unowned, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &keys)
                .expect("an unowned key on an unowned identity is legitimate");
            tx.commit().unwrap();
        }

        // NULL, not a 32-byte zero blob: the distinction the readers and
        // both guards key on.
        let is_null: bool = conn
            .query_row(
                "SELECT wallet_id IS NULL FROM identity_keys WHERE identity_id = ?1",
                params![&z.to_buffer()[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert!(is_null, "the sentinel scope must store SQL NULL");

        // Re-saving the same key exercises the DO UPDATE path, which the
        // BEFORE UPDATE trigger also inspects — it must stay permitted.
        let mut resave = IdentityKeysChangeSet::default();
        resave.upserts.insert((z, 0), sample_key_entry(z, 0x5C));
        {
            let tx = conn.transaction().unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &resave)
                .expect("re-saving an unowned key must stay permitted");
            tx.commit().unwrap();
        }
    }

    /// The NULL door into the corruption the compound FK closed: with a
    /// NULL `wallet_id` both FKs go dormant, so nothing at the FK level
    /// stops an unowned key from naming a WALLET-OWNED identity — a row
    /// no per-wallet reader can resolve. The trigger pair is what rejects
    /// it, and it must surface as the same typed error as the FK path.
    ///
    /// Both statement paths are exercised: the INSERT trigger, and the
    /// UPDATE trigger reached by an upsert colliding on an existing
    /// `(identity_id, key_id)`. Only the second catches a re-save, which
    /// is the shape ordinary writes take.
    #[test]
    fn null_scoped_key_is_rejected_for_a_wallet_owned_identity() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let owner = [0xD1u8; 32];
        let unowned = [0u8; 32];
        insert_wallet(&conn, &owner);

        // Identity X is owned by a real wallet.
        let x = Identifier::from([0xD2u8; 32]);
        let mut ids = IdentityChangeSet::default();
        ids.identities
            .insert(x, entry([0xD2; 32], Some(owner), 300, Some(0)));
        apply_in_tx(&mut conn, &owner, &ids);

        // Both FKs are dormant for a NULL scope, so a rejection here can
        // only have come from the trigger. Assert that positively via
        // the extended result code (1811 = SQLITE_CONSTRAINT_TRIGGER)
        // rather than inferring it, so the test still proves the trigger
        // fired if some future guard starts rejecting earlier.
        let assert_raised_by_trigger = |err: &WalletStorageError| match err {
            WalletStorageError::IdentityKeyWalletMismatch { source, .. } => match source.as_ref() {
                rusqlite::Error::SqliteFailure(e, _) => assert_eq!(
                    e.extended_code, 1811,
                    "rejection must come from the NULL-scope trigger, not an FK"
                ),
                other => panic!("expected a SqliteFailure source, got {other:?}"),
            },
            other => panic!("expected IdentityKeyWalletMismatch, got {other:?}"),
        };

        // INSERT path: no row for (X, 0) yet, so this reaches the
        // BEFORE INSERT trigger.
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((x, 0), sample_key_entry(x, 0xD3));
        {
            let tx = conn.transaction().unwrap();
            let err = crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &keys)
                .expect_err("an unowned key may not name a wallet-owned identity");
            assert!(
                matches!(
                    err,
                    WalletStorageError::IdentityKeyWalletMismatch {
                        wallet_id, identity_id, ..
                    } if wallet_id == unowned && identity_id == x.to_buffer()
                ),
                "expected IdentityKeyWalletMismatch from the INSERT trigger, got {err:?}"
            );
            assert_raised_by_trigger(&err);
        }

        // UPDATE path: stage the key legitimately under its owner first,
        // so the unowned write now COLLIDES and resolves to DO UPDATE —
        // which the BEFORE INSERT trigger never sees.
        let mut owner_keys = IdentityKeysChangeSet::default();
        owner_keys.upserts.insert((x, 0), sample_key_entry(x, 0xD4));
        {
            let tx = conn.transaction().unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &owner, &owner_keys).unwrap();
            tx.commit().unwrap();
        }
        {
            let tx = conn.transaction().unwrap();
            let err = crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &keys)
                .expect_err("the UPDATE path must be guarded too");
            assert!(
                matches!(err, WalletStorageError::IdentityKeyWalletMismatch { .. }),
                "expected IdentityKeyWalletMismatch from the UPDATE trigger, got {err:?}"
            );
            assert_raised_by_trigger(&err);
        }

        // The owner's row is untouched: still owned, still its own material.
        let (scope, blob_head): (Option<Vec<u8>>, Vec<u8>) = conn
            .query_row(
                "SELECT wallet_id, public_key_blob FROM identity_keys WHERE identity_id = ?1",
                params![&x.to_buffer()[..]],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            scope.as_deref(),
            Some(&owner[..]),
            "the owner's scope must survive the rejected unowned write"
        );
        assert!(!blob_head.is_empty());
    }

    /// A NULL-scoped key must be deletable. The delete carries a scope
    /// guard, and with a plain `wallet_id = ?1` that guard can never
    /// match NULL: the statement would succeed, remove nothing, and
    /// report `Ok` — an unowned key that no caller can ever erase.
    #[test]
    fn null_scoped_key_can_be_deleted() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let unowned = [0u8; 32];
        let z = Identifier::from([0x6Au8; 32]);

        let mut ids = IdentityChangeSet::default();
        ids.identities.insert(z, entry([0x6A; 32], None, 20, None));
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((z, 0), sample_key_entry(z, 0x6B));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &unowned, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &keys).unwrap();
            tx.commit().unwrap();
        }

        let mut removal = IdentityKeysChangeSet::default();
        removal.removed.insert((z, 0));
        {
            let tx = conn.transaction().unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &removal).unwrap();
            tx.commit().unwrap();
        }

        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
                params![&z.to_buffer()[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0, "the unowned key must actually be deleted");
    }

    /// `load_prekeyed` skips — never hard-errors on — an `identity_keys`
    /// entry whose owner is a known-tombstoned identity: those orphaned rows
    /// are the expected, self-explained fallout of a logical delete.
    #[test]
    fn load_prekeyed_skips_orphaned_keys_of_tombstoned_owner_in_recovery() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let a = [0xA7u8; 32];
        insert_wallet(&conn, &a);
        let y = Identifier::from([0x44u8; 32]);

        let mut ids = IdentityChangeSet::default();
        ids.identities
            .insert(y, entry([0x44; 32], Some(a), 50, Some(0)));
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((y, 0), sample_key_entry(y, 0xD4));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &a, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &a, &keys).unwrap();
            tx.commit().unwrap();
        }
        // Tombstone Y; its key row survives as a logical-delete orphan.
        let mut removed = IdentityChangeSet::default();
        removed.removed.insert(y);
        apply_in_tx(&mut conn, &a, &removed);

        let strict = load_prekeyed(&conn, &a, &LoadCtx::strict())
            .expect_err("a tombstoned owner's leftover key must abort a strict load");
        assert!(
            matches!(strict, WalletStorageError::OrphanedIdentityEntry { .. }),
            "expected OrphanedIdentityEntry, got {strict:?}"
        );

        let state = load_prekeyed(&conn, &a, &LoadCtx::recovery())
            .expect("tombstoned-owner orphan must be skipped in recovery, not fatal");
        assert!(
            state
                .wallet_identities
                .get(&a)
                .map(|m| m.is_empty())
                .unwrap_or(true),
            "tombstoned identity must not surface in the loaded state"
        );
    }

    /// The same logical-delete skip, for a tombstoned UNOWNED identity.
    ///
    /// Not a duplicate of the wallet-owned case above: the skip depends on
    /// `load_tombstoned_ids` recognising the owner as tombstoned, and that
    /// query is scoped by `wallet_id`. Scoped with `=` it cannot match a
    /// NULL, so an unowned tombstone is invisible, its surviving key rows
    /// look like owners that vanished for no reason, and the read fails
    /// with `OrphanedIdentityEntry` — the exact brick this whole line of
    /// work exists to prevent, re-entering through the unowned door. The
    /// NULL-safe `IS` is what closes it, and this test is what holds it
    /// closed: revert that predicate to `= ?1` and this fails.
    #[test]
    fn load_prekeyed_skips_orphaned_keys_of_tombstoned_unowned_owner_in_recovery() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let unowned = [0u8; 32];
        let z = Identifier::from([0x9Cu8; 32]);

        let mut ids = IdentityChangeSet::default();
        ids.identities.insert(z, entry([0x9C; 32], None, 30, None));
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((z, 0), sample_key_entry(z, 0x9D));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &unowned, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &unowned, &keys).unwrap();
            tx.commit().unwrap();
        }

        // Tombstone Z. Its key row survives — a logical delete tombstones
        // the identity, it does not reap the children.
        let mut removed = IdentityChangeSet::default();
        removed.removed.insert(z);
        apply_in_tx(&mut conn, &unowned, &removed);
        let surviving_keys: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1",
                params![&z.to_buffer()[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            surviving_keys, 1,
            "the orphaned key row must actually exist, or this test proves nothing"
        );

        let state = load_prekeyed(&conn, &unowned, &LoadCtx::recovery())
            .expect("a tombstoned UNOWNED owner's orphan key must be skipped in recovery");
        assert!(
            state.out_of_wallet_identities.is_empty(),
            "the tombstoned identity must not surface in the loaded state"
        );
    }

    /// The delete's scope guard, which nothing else holds in place.
    ///
    /// `(identity_id, key_id)` is the primary key, so `wallet_id` in the
    /// DELETE's WHERE is no longer needed to SELECT the row — which makes
    /// removing it look like an obvious tidy-up. It is not: it is what
    /// stops one wallet's `removed` set from deleting another wallet's
    /// key. Without it that cross-wallet delete succeeds, and a
    /// permissive deletion is a worse failure than a permissive no-op.
    #[test]
    fn identity_key_delete_cannot_reach_another_wallets_key() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let owner = [0xC1u8; 32];
        let other = [0xC2u8; 32];
        insert_wallet(&conn, &owner);
        insert_wallet(&conn, &other);

        let x = Identifier::from([0xC3u8; 32]);
        let mut ids = IdentityChangeSet::default();
        ids.identities
            .insert(x, entry([0xC3; 32], Some(owner), 600, Some(0)));
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((x, 0), sample_key_entry(x, 0xC4));
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &owner, &ids).unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &owner, &keys).unwrap();
            tx.commit().unwrap();
        }

        // The other wallet asks for the same key to be removed. Scoped
        // out, this is a clean no-op rather than an error.
        let mut removal = IdentityKeysChangeSet::default();
        removal.removed.insert((x, 0));
        {
            let tx = conn.transaction().unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &other, &removal).unwrap();
            tx.commit().unwrap();
        }

        let surviving: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1 AND key_id = 0",
                params![&x.to_buffer()[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            surviving, 1,
            "a foreign wallet's removed set must not delete the owner's key"
        );

        // ...and the owner can still delete its own.
        {
            let tx = conn.transaction().unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &owner, &removal).unwrap();
            tx.commit().unwrap();
        }
        let after_owner: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM identity_keys WHERE identity_id = ?1 AND key_id = 0",
                params![&x.to_buffer()[..]],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            after_owner, 0,
            "the owning wallet must still be able to delete"
        );
    }

    /// The converse of the NULL-scope trigger, and the reason no third
    /// trigger is needed: a WALLET-scoped key naming an UNOWNED identity.
    ///
    /// Here the child key `(wallet_id, identity_id)` is fully non-NULL, so
    /// MATCH SIMPLE leaves the compound FK live and it rejects the write
    /// on its own. Asserting the extended code keeps that claim honest —
    /// 787 proves the FK did it, rather than the trigger or one of the
    /// earlier in-Rust guards quietly covering for an unenforced FK.
    #[test]
    fn wallet_scoped_key_is_rejected_for_an_unowned_identity() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let a = [0xB7u8; 32];
        let unowned = [0u8; 32];
        insert_wallet(&conn, &a);

        // Identity Z belongs to no wallet.
        let z = Identifier::from([0xB8u8; 32]);
        let mut ids = IdentityChangeSet::default();
        ids.identities.insert(z, entry([0xB8; 32], None, 40, None));
        apply_in_tx(&mut conn, &unowned, &ids);

        // Wallet A files a key for it under its own scope.
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((z, 0), sample_key_entry(z, 0xB9));
        let tx = conn.transaction().unwrap();
        let err = crate::sqlite::schema::identity_keys::apply(&tx, &a, &keys)
            .expect_err("a wallet may not file a key for an identity it does not own");
        assert!(
            matches!(
                err,
                WalletStorageError::IdentityKeyWalletMismatch {
                    wallet_id, identity_id, ..
                } if wallet_id == a && identity_id == z.to_buffer()
            ),
            "expected IdentityKeyWalletMismatch naming wallet A and identity Z, got {err:?}"
        );
        match &err {
            WalletStorageError::IdentityKeyWalletMismatch { source, .. } => match source.as_ref() {
                rusqlite::Error::SqliteFailure(e, _) => assert_eq!(
                    e.extended_code, 787,
                    "this case must be caught by the live compound FK, not a trigger"
                ),
                other => panic!("expected a SqliteFailure source, got {other:?}"),
            },
            other => panic!("expected IdentityKeyWalletMismatch, got {other:?}"),
        }
    }

    /// `load_state` rejects a row whose decoded blob names a different
    /// `identity_id` than its typed column — corruption is a hard, typed
    /// error, never rehydrated under the wrong id.
    #[test]
    fn load_state_rejects_identity_id_column_mismatch() {
        let conn = migrated_conn();
        let a = [0xA1u8; 32];
        insert_wallet(&conn, &a);
        let typed_id = [0x01u8; 32]; // column
        let blob_id = [0x02u8; 32]; // disagreeing blob
        let payload = blob::encode(&entry(blob_id, Some(a), 100, Some(1))).unwrap();
        conn.execute(
            "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, 1, ?3, 0)",
            params![&typed_id[..], &a[..], payload],
        )
        .unwrap();

        let err = load_state(&conn, &a).expect_err("identity_id mismatch must fail");
        assert!(
            matches!(err, WalletStorageError::IdentityEntryIdMismatch),
            "expected IdentityEntryIdMismatch, got {err:?}"
        );
    }
}
