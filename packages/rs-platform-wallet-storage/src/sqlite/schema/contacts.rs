//! `contacts_sent` / `contacts_recv` / `contacts_established` writers
//! and per-wallet reader.

use rusqlite::{params, Connection, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, ContactsStartState, ReceivedContactRequestKey,
    SentContactRequestKey,
};
use platform_wallet::wallet::identity::EstablishedContact;
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

/// Build a [`ContactsStartState`] for one wallet from the three
/// `contacts_*` tables. Rows that fail to decode are skipped — the
/// skip count is returned so callers can surface it via the `load()`
/// summary log.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<(ContactsStartState, usize), WalletStorageError> {
    let mut state = ContactsStartState::default();
    let mut skipped = 0usize;

    let mut sent_stmt = conn.prepare(
        "SELECT owner_id, recipient_id, entry_blob FROM contacts_sent WHERE wallet_id = ?1",
    )?;
    let mut rows = sent_stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let recipient: Vec<u8> = row.get(1)?;
        let payload: Vec<u8> = row.get(2)?;
        let key = match decode_pair_key(&owner, &recipient) {
            Ok((o, r)) => SentContactRequestKey {
                owner_id: o,
                recipient_id: r,
            },
            Err(e) => {
                warn_skip(wallet_id, "contacts_sent", &e);
                skipped += 1;
                continue;
            }
        };
        let entry: ContactRequestEntry = match blob::decode(&payload) {
            Ok(e) => e,
            Err(e) => {
                warn_skip(wallet_id, "contacts_sent", &e);
                skipped += 1;
                continue;
            }
        };
        state.sent_requests.insert(key, entry);
    }

    let mut recv_stmt = conn.prepare(
        "SELECT owner_id, sender_id, entry_blob FROM contacts_recv WHERE wallet_id = ?1",
    )?;
    let mut rows = recv_stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let sender: Vec<u8> = row.get(1)?;
        let payload: Vec<u8> = row.get(2)?;
        let key = match decode_pair_key(&owner, &sender) {
            Ok((o, s)) => ReceivedContactRequestKey {
                owner_id: o,
                sender_id: s,
            },
            Err(e) => {
                warn_skip(wallet_id, "contacts_recv", &e);
                skipped += 1;
                continue;
            }
        };
        let entry: ContactRequestEntry = match blob::decode(&payload) {
            Ok(e) => e,
            Err(e) => {
                warn_skip(wallet_id, "contacts_recv", &e);
                skipped += 1;
                continue;
            }
        };
        state.incoming_requests.insert(key, entry);
    }

    let mut est_stmt = conn.prepare(
        "SELECT owner_id, contact_id, entry_blob FROM contacts_established WHERE wallet_id = ?1",
    )?;
    let mut rows = est_stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let contact: Vec<u8> = row.get(1)?;
        let payload: Vec<u8> = row.get(2)?;
        let key = match decode_pair_key(&owner, &contact) {
            Ok((o, c)) => SentContactRequestKey {
                owner_id: o,
                recipient_id: c,
            },
            Err(e) => {
                warn_skip(wallet_id, "contacts_established", &e);
                skipped += 1;
                continue;
            }
        };
        let value: EstablishedContact = match blob::decode(&payload) {
            Ok(e) => e,
            Err(e) => {
                warn_skip(wallet_id, "contacts_established", &e);
                skipped += 1;
                continue;
            }
        };
        state.established.insert(key, value);
    }

    Ok((state, skipped))
}

fn decode_pair_key(a: &[u8], b: &[u8]) -> Result<(Identifier, Identifier), WalletStorageError> {
    let a32 = <[u8; 32]>::try_from(a)
        .map_err(|_| WalletStorageError::blob_decode("contacts.id column is not 32 bytes"))?;
    let b32 = <[u8; 32]>::try_from(b)
        .map_err(|_| WalletStorageError::blob_decode("contacts.id column is not 32 bytes"))?;
    Ok((Identifier::from(a32), Identifier::from(b32)))
}

fn warn_skip(wallet_id: &WalletId, table: &'static str, err: &WalletStorageError) {
    tracing::warn!(
        wallet_id = %hex::encode(wallet_id),
        table,
        error = %err,
        "skipping undecodable contacts row"
    );
}
