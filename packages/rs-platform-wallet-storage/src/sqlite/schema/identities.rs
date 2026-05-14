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
        let mut stmt = tx.prepare_cached(
            "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
             VALUES (?1, ?2, ?3, ?4, 0) \
             ON CONFLICT(wallet_id, identity_id) DO UPDATE SET \
                wallet_index = excluded.wallet_index, \
                entry_blob = excluded.entry_blob, \
                tombstoned = 0",
        )?;
        for (id, entry) in &cs.identities {
            let payload = blob::encode(entry)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                entry.identity_index.map(i64::from),
                id.as_slice(),
                payload,
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        let mut stmt = tx.prepare_cached(
            "UPDATE identities SET tombstoned = 1 WHERE wallet_id = ?1 AND identity_id = ?2",
        )?;
        for id in &cs.removed {
            stmt.execute(params![wallet_id.as_slice(), id.as_slice()])?;
        }
    }
    Ok(())
}

/// Decode a single `identities` row back to its [`IdentityEntry`].
///
/// Returns `Ok(None)` if no row matches. Tombstoned rows decode to
/// `Some(entry)`; the caller inspects the dedicated `tombstoned`
/// column to discriminate when needed.
pub fn fetch(
    conn: &Connection,
    wallet_id: &WalletId,
    identity_id: &[u8; 32],
) -> Result<Option<IdentityEntry>, WalletStorageError> {
    use rusqlite::OptionalExtension;
    let row: Option<Vec<u8>> = conn
        .query_row(
            "SELECT entry_blob FROM identities WHERE wallet_id = ?1 AND identity_id = ?2",
            params![wallet_id.as_slice(), &identity_id[..]],
            |row| row.get(0),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some(payload) => Ok(Some(blob::decode(&payload)?)),
    }
}

/// Build an [`IdentityManagerStartState`] for one wallet from the
/// `identities` table. Tombstoned rows and rows that fail to decode
/// are skipped — the skip count is returned so callers can surface
/// it via the `load()` summary log.
///
/// The bucket selection mirrors `IdentityManager`'s layout:
/// rows with `IdentityEntry.identity_index = Some(_)` go into
/// `wallet_identities[wallet_id]`; rows with `None` go into
/// `out_of_wallet_identities`.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<(platform_wallet::changeset::IdentityManagerStartState, usize), WalletStorageError> {
    use platform_wallet::changeset::IdentityManagerStartState;

    let mut stmt = conn.prepare(
        "SELECT identity_id, entry_blob, tombstoned FROM identities WHERE wallet_id = ?1",
    )?;
    let mut state = IdentityManagerStartState::default();
    let mut skipped = 0usize;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let identity_id: Vec<u8> = row.get(0)?;
        let payload: Vec<u8> = row.get(1)?;
        let tombstoned: i64 = row.get(2)?;
        if tombstoned != 0 {
            continue;
        }
        let entry: IdentityEntry = match blob::decode(&payload) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    table = "identities",
                    identity_id = %hex::encode(&identity_id),
                    error = %e,
                    "skipping undecodable identity row"
                );
                skipped += 1;
                continue;
            }
        };
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
    Ok((state, skipped))
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
/// reference it via the FK trigger. Used by tests that exercise
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
    conn.execute(
        "INSERT OR IGNORE INTO identities \
            (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
         VALUES (?1, NULL, ?2, ?3, 0)",
        params![wallet_id.as_slice(), &identity_id[..], payload],
    )?;
    Ok(())
}
