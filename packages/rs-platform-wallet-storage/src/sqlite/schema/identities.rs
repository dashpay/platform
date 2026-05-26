//! `identities` table writer.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::{IdentityChangeSet, IdentityEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &IdentityChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.identities.is_empty() {
        // V002: PK is `identity_id` alone; `wallet_id` is nullable
        // and links the identity to its parent wallet for cascade.
        // The sentinel-zero wallet id (`[0u8; 32]`) is the legacy
        // placeholder for "no parent wallet known" — stored as NULL
        // so the FK to `wallet_metadata` doesn't activate.
        // INTENTIONAL(SEC-001): NULL wallet_id allowed per CODE-002 design;
        // COALESCE upsert is the intended merge semantic for orphan-identity-to-wallet promotion.
        // Existing wallet_id is preserved on re-upsert; new wallet_id fills NULL.
        let mut stmt = tx.prepare_cached(
            "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
             VALUES (?1, ?2, ?3, ?4, 0) \
             ON CONFLICT(identity_id) DO UPDATE SET \
                wallet_id = COALESCE(excluded.wallet_id, identities.wallet_id), \
                wallet_index = excluded.wallet_index, \
                entry_blob = excluded.entry_blob, \
                tombstoned = 0",
        )?;
        let wallet_id_param = wallet_id_to_param(wallet_id);
        for (id, entry) in &cs.identities {
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

/// V002: callers still receive a `WalletId` (32 bytes) from the
/// caller boundary. Treat the all-zero sentinel as "no parent wallet"
/// (NULL) so the nullable `identities.wallet_id` FK matches reality.
fn wallet_id_to_param(wallet_id: &WalletId) -> Option<&[u8]> {
    if wallet_id.iter().all(|b| *b == 0) {
        None
    } else {
        Some(wallet_id.as_slice())
    }
}

/// Decode a single `identities` row back to its [`IdentityEntry`].
///
/// Returns `Ok(None)` if no row matches. This reads only `entry_blob`
/// and does NOT expose the `tombstoned` column — a tombstoned row still
/// decodes to `Some(entry)` here. Callers that must skip logically
/// deleted identities should use [`load_state`], which filters
/// tombstoned rows.
pub fn fetch(
    conn: &Connection,
    _wallet_id: &WalletId,
    identity_id: &[u8; 32],
) -> Result<Option<IdentityEntry>, WalletStorageError> {
    use rusqlite::OptionalExtension;
    // V002: `identity_id` is the PK; the caller-supplied `wallet_id`
    // is preserved on the signature for source-compatibility but is
    // no longer part of the lookup key.
    let row: Option<Vec<u8>> = conn
        .query_row(
            "SELECT entry_blob FROM identities WHERE identity_id = ?1",
            params![&identity_id[..]],
            |row| row.get(0),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some(payload) => Ok(Some(blob::decode(&payload)?)),
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
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<platform_wallet::changeset::IdentityManagerStartState, WalletStorageError> {
    use platform_wallet::changeset::IdentityManagerStartState;

    // V002: wallet_id is nullable on identities; this load path still
    // wants only the rows belonging to the wallet the caller asked
    // for, so the WHERE clause matches by wallet_id (orphan identities
    // — wallet_id NULL — are out of scope for this per-wallet loader).
    let mut stmt = conn.prepare(
        "SELECT identity_id, entry_blob, tombstoned FROM identities WHERE wallet_id = ?1",
    )?;
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
        let managed = managed_identity_from_entry(&entry, wallet_id);
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
fn managed_identity_from_entry(
    entry: &IdentityEntry,
    wallet_id: &WalletId,
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
    ManagedIdentity {
        identity,
        identity_index: entry.identity_index,
        last_updated_balance_block_time: entry.last_updated_balance_block_time,
        last_synced_keys_block_time: entry.last_synced_keys_block_time,
        established_contacts: Default::default(),
        sent_contact_requests: Default::default(),
        incoming_contact_requests: Default::default(),
        status: entry.status,
        dpns_names: entry.dpns_names.clone(),
        contested_dpns_names: entry.contested_dpns_names.clone(),
        wallet_id: entry.wallet_id.or(Some(*wallet_id)),
        dashpay_profile: entry.dashpay_profile.clone(),
        dashpay_payments: entry.dashpay_payments.clone(),
    }
}

/// Insert a stub identity row so identity_keys / dashpay_profiles can
/// reference it via their native composite FK. Used by tests that exercise
/// identity_keys persistence without going through the full identity
/// flow. The stub row carries a `null`-encoded `IdentityEntry` so the
/// `entry_blob` column always decodes — callers wanting real data
/// overwrite via [`apply`].
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
