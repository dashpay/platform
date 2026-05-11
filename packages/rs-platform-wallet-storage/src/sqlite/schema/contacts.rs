//! `contacts_sent` / `contacts_recv` / `contacts_established` writers.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::ContactChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;
use crate::sqlite::schema::blob::BlobWriter;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &ContactChangeSet,
) -> Result<(), SqlitePersisterError> {
    for key in cs.sent_requests.keys() {
        // `ContactRequestEntry` carries an opaque `ContactRequest`
        // upstream type with no serde — store the key columns and an
        // empty marker blob; the contact-request payload itself is
        // recomputable from network sources.
        tx.execute(
            "INSERT INTO contacts_sent (wallet_id, owner_id, recipient_id, entry_blob) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, owner_id, recipient_id) DO UPDATE SET entry_blob = excluded.entry_blob",
            params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
                BlobWriter::new().finish(),
            ],
        )?;
    }
    for key in &cs.removed_sent {
        tx.execute(
            "DELETE FROM contacts_sent WHERE wallet_id = ?1 AND owner_id = ?2 AND recipient_id = ?3",
            params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
            ],
        )?;
    }
    for key in cs.incoming_requests.keys() {
        tx.execute(
            "INSERT INTO contacts_recv (wallet_id, owner_id, sender_id, entry_blob) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, owner_id, sender_id) DO UPDATE SET entry_blob = excluded.entry_blob",
            params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.sender_id.as_slice(),
                BlobWriter::new().finish(),
            ],
        )?;
    }
    for key in &cs.removed_incoming {
        tx.execute(
            "DELETE FROM contacts_recv WHERE wallet_id = ?1 AND owner_id = ?2 AND sender_id = ?3",
            params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.sender_id.as_slice(),
            ],
        )?;
    }
    for key in cs.established.keys() {
        tx.execute(
            "INSERT INTO contacts_established (wallet_id, owner_id, contact_id, entry_blob) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, owner_id, contact_id) DO UPDATE SET entry_blob = excluded.entry_blob",
            params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
                BlobWriter::new().finish(),
            ],
        )?;
    }
    Ok(())
}
