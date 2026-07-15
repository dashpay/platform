//! `identities` table writer.

use std::collections::{HashMap, HashSet};

use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use platform_wallet::{ContactChangeSet, IdentityKeysChangeSet, ManagedIdentity};
use rusqlite::{params, Transaction};

use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::{changeset::IdentityChangeSet, IdentityManagerStartState};

use {platform_wallet::changeset::IdentityEntry, rusqlite::Connection};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: identity snapshot reaching the `entry_blob` column.
impl_persistable_blob!(IdentityEntry);

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityChangeSet,
) -> Result<(), WalletStorageError> {
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

/// Map a `WalletId` to the nullable `identities.wallet_id` column: the
/// all-zero sentinel becomes NULL so the FK doesn't activate.
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
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    use platform_wallet::changeset::IdentityManagerStartState;

    // Per-wallet loader: match by wallet_id, so orphan rows (NULL wallet_id)
    // are out of scope.
    let mut stmt = conn.prepare(
        "SELECT identity_id, length(entry_blob), entry_blob, tombstoned \
         FROM identities WHERE wallet_id = ?1",
    )?;
    // The ignored-senders TABLE is the authoritative ignore record (every
    // ignore/un-ignore maintains it transactionally); the `entry_blob`'s
    // snapshot copy can be stale — see `contacts::load_ignored_senders`.
    let mut ignored_by_owner =
        crate::sqlite::schema::contacts::load_ignored_senders(conn, wallet_id)?;
    let mut state = IdentityManagerStartState::default();
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let identity_id_bytes: Vec<u8> = row.get(0)?;
        blob::check_size(row.get::<_, i64>(1)?)?;
        let payload: Vec<u8> = row.get(2)?;
        let tombstoned: i64 = row.get(3)?;
        if tombstoned != 0 {
            continue;
        }
        let entry: IdentityEntry = blob::decode(&payload)?;
        // Cross-check the decoded blob against the typed columns it was
        // selected by (mirrors the accounts / identity_keys readers): the
        // blob must name the same identity, and its own wallet_id (when set)
        // must match the wallet scope, else the row is corrupt / mis-filed.
        let typed_id = <[u8; 32]>::try_from(identity_id_bytes.as_slice()).map_err(|_| {
            WalletStorageError::blob_decode("identities.identity_id is not 32 bytes")
        })?;
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
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    let mut state = load_state(conn, wallet_id)?;
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
    merge_contacts_and_keys(&mut state, contacts, identity_keys, &tombstoned)?;
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
    let mut stmt =
        conn.prepare("SELECT identity_id FROM identities WHERE wallet_id = ?1 AND tombstoned = 1")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = HashSet::new();
    while let Some(row) = rows.next()? {
        let id_bytes: Vec<u8> = row.get(0)?;
        let id32 = <[u8; 32]>::try_from(id_bytes.as_slice()).map_err(|_| {
            WalletStorageError::blob_decode("identities.identity_id is not 32 bytes")
        })?;
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
/// Entries route by owner `identity_id` across BOTH buckets. An owner
/// absent from the loaded set is acceptable ONLY when it names a
/// known-tombstoned identity (`tombstoned`) — its rows are logical-delete
/// orphans, skipped and summarised once per collection. Any other miss is
/// corruption or a wallet-scope mismatch and returns
/// [`WalletStorageError::OrphanedIdentityEntry`] rather than silently
/// dropping live key / contact state. Only key `upserts` and the
/// `sent` / `incoming` / `established` maps are routed; `removed_*`
/// (insert-only feed) and `ignored` / `unignored` (restored in the
/// identity reader from the `ignored_senders` table) are skipped. No
/// `Network` needed — key insert is network-independent.
pub fn merge_contacts_and_keys(
    state: &mut IdentityManagerStartState,
    contacts: ContactChangeSet,
    identity_keys: IdentityKeysChangeSet,
    tombstoned: &HashSet<Identifier>,
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

    let mut skipped_keys = 0usize;
    for (_key, entry) in identity_keys.upserts {
        match by_id.get_mut(&entry.identity_id) {
            Some(managed) => managed.identity.add_public_key(entry.public_key),
            None if tombstoned.contains(&entry.identity_id) => skipped_keys += 1,
            None => {
                return Err(WalletStorageError::OrphanedIdentityEntry {
                    owner: entry.identity_id.to_buffer(),
                })
            }
        }
    }
    if skipped_keys > 0 {
        tracing::warn!(
            count = skipped_keys,
            "skipped identity keys of tombstoned identities during rehydration merge"
        );
    }

    let mut skipped_sent = 0usize;
    for (key, entry) in contacts.sent_requests {
        match by_id.get_mut(&key.owner_id) {
            Some(managed) => managed.apply_sent_contact_request(entry.request),
            None if tombstoned.contains(&key.owner_id) => skipped_sent += 1,
            None => {
                return Err(WalletStorageError::OrphanedIdentityEntry {
                    owner: key.owner_id.to_buffer(),
                })
            }
        }
    }
    if skipped_sent > 0 {
        tracing::warn!(
            count = skipped_sent,
            "skipped sent contact requests of tombstoned identities during rehydration merge"
        );
    }

    let mut skipped_incoming = 0usize;
    for (key, entry) in contacts.incoming_requests {
        match by_id.get_mut(&key.owner_id) {
            Some(managed) => managed.apply_incoming_contact_request(entry.request),
            None if tombstoned.contains(&key.owner_id) => skipped_incoming += 1,
            None => {
                return Err(WalletStorageError::OrphanedIdentityEntry {
                    owner: key.owner_id.to_buffer(),
                })
            }
        }
    }
    if skipped_incoming > 0 {
        tracing::warn!(
            count = skipped_incoming,
            "skipped incoming contact requests of tombstoned identities during rehydration merge"
        );
    }

    let mut skipped_established = 0usize;
    for (key, established) in contacts.established {
        match by_id.get_mut(&key.owner_id) {
            Some(managed) => managed.apply_established_contact(established),
            None if tombstoned.contains(&key.owner_id) => skipped_established += 1,
            None => {
                return Err(WalletStorageError::OrphanedIdentityEntry {
                    owner: key.owner_id.to_buffer(),
                })
            }
        }
    }
    if skipped_established > 0 {
        tracing::warn!(
            count = skipped_established,
            "skipped established contacts of tombstoned identities during rehydration merge"
        );
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

        let state = load_prekeyed(&conn, &w).unwrap();
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

    /// `load_prekeyed` hard-errors when a persisted `identity_keys` entry's
    /// owner is absent from the load for a reason other than a tombstone
    /// (here: the identity is parented to a different wallet). Live public
    /// keys must never vanish silently — only a known-tombstoned owner's
    /// orphaned rows are safe to skip.
    #[test]
    fn load_prekeyed_hard_errors_on_orphaned_non_tombstoned_owner() {
        use platform_wallet::changeset::IdentityKeysChangeSet;

        let mut conn = migrated_conn();
        let a = [0xA1u8; 32];
        let b = [0xB2u8; 32];
        insert_wallet(&conn, &a);
        insert_wallet(&conn, &b);

        // Identity X exists but is parented to wallet B, so load_prekeyed(A)
        // neither loads it nor sees it tombstoned in A's scope — a genuine
        // orphan, not a logical delete.
        let x = Identifier::from([0x33u8; 32]);
        let mut ids_b = IdentityChangeSet::default();
        ids_b
            .identities
            .insert(x, entry([0x33; 32], Some(b), 100, Some(0)));
        apply_in_tx(&mut conn, &b, &ids_b);

        // A public key for X is filed under wallet A (owner absent from A).
        let mut keys = IdentityKeysChangeSet::default();
        keys.upserts.insert((x, 0), sample_key_entry(x, 0xC3));
        {
            let tx = conn.transaction().unwrap();
            crate::sqlite::schema::identity_keys::apply(&tx, &a, &keys).unwrap();
            tx.commit().unwrap();
        }

        let err =
            load_prekeyed(&conn, &a).expect_err("orphaned non-tombstoned owner must hard-error");
        assert!(
            matches!(err, WalletStorageError::OrphanedIdentityEntry { .. }),
            "expected OrphanedIdentityEntry, got {err:?}"
        );
    }

    /// `load_prekeyed` skips — never hard-errors on — an `identity_keys`
    /// entry whose owner is a known-tombstoned identity: those orphaned rows
    /// are the expected, self-explained fallout of a logical delete.
    #[test]
    fn load_prekeyed_skips_orphaned_keys_of_tombstoned_owner() {
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

        let state =
            load_prekeyed(&conn, &a).expect("tombstoned-owner orphan must be skipped, not fatal");
        assert!(
            state
                .wallet_identities
                .get(&a)
                .map(|m| m.is_empty())
                .unwrap_or(true),
            "tombstoned identity must not surface in the loaded state"
        );
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
