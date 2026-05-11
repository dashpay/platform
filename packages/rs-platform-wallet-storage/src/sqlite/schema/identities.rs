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
    for (id, entry) in &cs.identities {
        let payload = blob::encode(entry)?;
        tx.execute(
            "INSERT INTO identities (wallet_id, wallet_index, identity_id, entry_blob, tombstoned) \
             VALUES (?1, ?2, ?3, ?4, 0) \
             ON CONFLICT(wallet_id, identity_id) DO UPDATE SET \
                wallet_index = excluded.wallet_index, \
                entry_blob = excluded.entry_blob, \
                tombstoned = 0",
            params![
                wallet_id.as_slice(),
                entry.identity_index.map(i64::from),
                id.as_slice(),
                payload,
            ],
        )?;
    }
    for id in &cs.removed {
        tx.execute(
            "UPDATE identities SET tombstoned = 1 WHERE wallet_id = ?1 AND identity_id = ?2",
            params![wallet_id.as_slice(), id.as_slice()],
        )?;
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
