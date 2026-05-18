//! `contacts_sent` / `contacts_recv` / `contacts_established` writers
//! and per-wallet reader.

use std::collections::BTreeMap;

use rusqlite::{params, Connection, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, ReceivedContactRequestKey, SentContactRequestKey,
};
use platform_wallet::wallet::identity::EstablishedContact;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// Storage-internal snapshot of one wallet's `contacts_*` rows.
///
/// Mirrors the populated-only subset of
/// [`ContactChangeSet`](platform_wallet::changeset::ContactChangeSet);
/// `removed_*` are absent because deletes never reach storage as rows
/// (the writer applies them as `DELETE`s). Crate-internal on purpose —
/// rs-platform-wallet's `ClientStartState` does not carry a contacts
/// slot, so this type is never re-exported across the crate boundary.
/// Promoted to `pub` only under `__test-helpers` so this crate's own
/// integration tests can assert on the hardened reader directly.
#[derive(Debug, Default, PartialEq)]
#[cfg(not(feature = "__test-helpers"))]
pub(crate) struct ContactsRecords {
    pub sent_requests: BTreeMap<SentContactRequestKey, ContactRequestEntry>,
    pub incoming_requests: BTreeMap<ReceivedContactRequestKey, ContactRequestEntry>,
    pub established: BTreeMap<SentContactRequestKey, EstablishedContact>,
}

/// See the `not(__test-helpers)` definition for the canonical docs.
#[derive(Debug, Default, PartialEq)]
#[cfg(feature = "__test-helpers")]
pub struct ContactsRecords {
    pub sent_requests: BTreeMap<SentContactRequestKey, ContactRequestEntry>,
    pub incoming_requests: BTreeMap<ReceivedContactRequestKey, ContactRequestEntry>,
    pub established: BTreeMap<SentContactRequestKey, EstablishedContact>,
}

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

/// Build a [`ContactsRecords`] for one wallet from the three
/// `contacts_*` tables. Any row that fails to decode is a hard error —
/// corruption is never silently dropped.
pub(crate) fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<ContactsRecords, WalletStorageError> {
    let mut state = ContactsRecords::default();

    let mut sent_stmt = conn.prepare(
        "SELECT owner_id, recipient_id, entry_blob FROM contacts_sent WHERE wallet_id = ?1",
    )?;
    let mut rows = sent_stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let recipient: Vec<u8> = row.get(1)?;
        let payload: Vec<u8> = row.get(2)?;
        let (owner_id, recipient_id) = decode_pair_key(&owner, &recipient)?;
        let entry: ContactRequestEntry = blob::decode(&payload)?;
        state.sent_requests.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id,
            },
            entry,
        );
    }

    let mut recv_stmt = conn.prepare(
        "SELECT owner_id, sender_id, entry_blob FROM contacts_recv WHERE wallet_id = ?1",
    )?;
    let mut rows = recv_stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let sender: Vec<u8> = row.get(1)?;
        let payload: Vec<u8> = row.get(2)?;
        let (owner_id, sender_id) = decode_pair_key(&owner, &sender)?;
        let entry: ContactRequestEntry = blob::decode(&payload)?;
        state.incoming_requests.insert(
            ReceivedContactRequestKey {
                owner_id,
                sender_id,
            },
            entry,
        );
    }

    let mut est_stmt = conn.prepare(
        "SELECT owner_id, contact_id, entry_blob FROM contacts_established WHERE wallet_id = ?1",
    )?;
    let mut rows = est_stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let contact: Vec<u8> = row.get(1)?;
        let payload: Vec<u8> = row.get(2)?;
        let (owner_id, recipient_id) = decode_pair_key(&owner, &contact)?;
        let value: EstablishedContact = blob::decode(&payload)?;
        state.established.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id,
            },
            value,
        );
    }

    Ok(state)
}

/// Bulk reader: one [`load_state`] call per wallet id listed in
/// `wallet_metadata`. Constant-query w.r.t. the number of wallets
/// touched per call site (FR-P4-6).
///
/// Driven by [`wallet_meta::list_ids`](crate::sqlite::schema::wallet_meta::list_ids):
/// orphaned `contacts_*` rows whose `wallet_id` is absent from
/// `wallet_metadata` are intentionally NOT surfaced. FK triggers
/// prevent such orphans; a future re-wire that needs them must restore
/// the id-union over the area tables.
// Dormant sibling of `load_state` — kept for API symmetry with the
// other area readers; `load()` no longer fans out to it.
#[allow(dead_code)]
pub(crate) fn load_all(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, ContactsRecords>, WalletStorageError> {
    let mut out = BTreeMap::new();
    for wallet_id in crate::sqlite::schema::wallet_meta::list_ids(conn)? {
        out.insert(wallet_id, load_state(conn, &wallet_id)?);
    }
    Ok(out)
}

fn decode_pair_key(a: &[u8], b: &[u8]) -> Result<(Identifier, Identifier), WalletStorageError> {
    let a32 = <[u8; 32]>::try_from(a)
        .map_err(|_| WalletStorageError::blob_decode("contacts.id column is not 32 bytes"))?;
    let b32 = <[u8; 32]>::try_from(b)
        .map_err(|_| WalletStorageError::blob_decode("contacts.id column is not 32 bytes"))?;
    Ok((Identifier::from(a32), Identifier::from(b32)))
}

/// Test-helper wrapper over [`load_state`] so this crate's integration
/// tests can assert on the hardened (fail-hard) contacts reader without
/// promoting the production surface beyond `pub(crate)`.
#[cfg(feature = "__test-helpers")]
pub fn load_state_for_test(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<ContactsRecords, WalletStorageError> {
    load_state(conn, wallet_id)
}
