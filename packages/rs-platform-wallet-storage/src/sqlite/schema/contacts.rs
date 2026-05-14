//! `contacts_sent` / `contacts_recv` / `contacts_established` writers.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::ContactChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &ContactChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.sent_requests.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO contacts_sent (wallet_id, owner_id, recipient_id, entry_blob) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, owner_id, recipient_id) DO UPDATE SET entry_blob = excluded.entry_blob",
        )?;
        for (key, entry) in &cs.sent_requests {
            let payload = blob::encode(entry)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
                payload,
            ])?;
        }
    }
    if !cs.removed_sent.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM contacts_sent WHERE wallet_id = ?1 AND owner_id = ?2 AND recipient_id = ?3",
        )?;
        for key in &cs.removed_sent {
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
            ])?;
        }
    }
    if !cs.incoming_requests.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO contacts_recv (wallet_id, owner_id, sender_id, entry_blob) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, owner_id, sender_id) DO UPDATE SET entry_blob = excluded.entry_blob",
        )?;
        for (key, entry) in &cs.incoming_requests {
            let payload = blob::encode(entry)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.sender_id.as_slice(),
                payload,
            ])?;
        }
    }
    if !cs.removed_incoming.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM contacts_recv WHERE wallet_id = ?1 AND owner_id = ?2 AND sender_id = ?3",
        )?;
        for key in &cs.removed_incoming {
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.sender_id.as_slice(),
            ])?;
        }
    }
    if !cs.established.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO contacts_established (wallet_id, owner_id, contact_id, entry_blob) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, owner_id, contact_id) DO UPDATE SET entry_blob = excluded.entry_blob",
        )?;
        for (key, established) in &cs.established {
            let payload = blob::encode(established)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
                payload,
            ])?;
        }
    }
    Ok(())
}
