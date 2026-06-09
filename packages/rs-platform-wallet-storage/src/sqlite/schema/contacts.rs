//! Unified `contacts` table writer and per-wallet reader.
//!
//! One row per `(wallet_id, owner_id, contact_id)` relationship; the `state`
//! column records its lifecycle stage (`sent` / `received` / `established`). A
//! pending relationship is `sent` XOR `received` and carries only the matching
//! request blob; an `established` one carries both request blobs plus the four
//! metadata columns (`alias`, `note`, `is_hidden`, `accepted_accounts`).

use std::collections::BTreeMap;

use rusqlite::{params, Connection, Transaction};

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, ReceivedContactRequestKey, SentContactRequestKey,
};
use platform_wallet::wallet::identity::{ContactRequest, EstablishedContact};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: contact-request types + the accepted-account
// index reaching the contacts `_blob` columns (contact requests carry
// public keys/refs; accepted_accounts is a list of account indices).
impl_persistable_blob!(ContactRequest, Vec<u32>);

/// Source of truth for the `contacts.state` TEXT domain, one label per
/// contact-relationship lifecycle stage. `migrations/V001__initial.rs`
/// interpolates it into a `CHECK (state IN (...))`;
/// `contact_state_labels_match_enum` keeps it in sync with
/// [`contact_state_db_label`].
pub(crate) const CONTACT_STATE_LABELS: &[&str] = &["sent", "received", "established"];

/// Lifecycle stage of a `contacts` row.
///
/// Not an upstream type — it names which [`ContactChangeSet`] slot a
/// row originated from, so the writer can stamp the `state` column and
/// the reader can bucket rows back into the right changeset map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContactState {
    Sent,
    Received,
    Established,
}

/// Stable database label for a [`ContactState`].
fn contact_state_db_label(state: ContactState) -> &'static str {
    match state {
        ContactState::Sent => "sent",
        ContactState::Received => "received",
        ContactState::Established => "established",
    }
}

/// Parse a `contacts.state` label back into a [`ContactState`]. An
/// unknown label is a hard error — the migration's `CHECK` constraint
/// already rejects writes outside the domain, so reaching this arm
/// means on-disk corruption or a forward-incompatible row.
fn contact_state_from_label(label: &str) -> Result<ContactState, WalletStorageError> {
    match label {
        "sent" => Ok(ContactState::Sent),
        "received" => Ok(ContactState::Received),
        "established" => Ok(ContactState::Established),
        _ => Err(WalletStorageError::blob_decode(
            "contacts.state column holds an unknown lifecycle label",
        )),
    }
}

/// Storage-internal snapshot of one wallet's `contacts` rows: the
/// populated-only subset of
/// [`ContactChangeSet`](platform_wallet::changeset::ContactChangeSet)
/// (`removed_*` absent — deletes reach storage as `DELETE`s). `pub(crate)`;
/// the public feed is [`load_changeset`]. Promoted to `pub` under
/// `__test-helpers` for this crate's integration tests.
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

/// Apply a [`ContactChangeSet`] onto the `contacts` table (inserts before
/// removes). Auto-establishment is honoured by `state` precedence: a pending
/// upsert never downgrades an already-`established` row, and an `established`
/// upsert collapses any prior pending row for the pair (both request blobs +
/// the four metadata columns, `state = 'established'`).
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &ContactChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.sent_requests.is_empty() {
        // Pending-sent upsert: `state` becomes 'sent' unless the row is
        // already established or an incoming blob is present, in which case
        // both sides exist and it promotes to 'established' in one statement.
        let sent = contact_state_db_label(ContactState::Sent);
        let mut stmt = tx.prepare_cached(
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state, outgoing_request) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, owner_id, contact_id) DO UPDATE SET \
                outgoing_request = excluded.outgoing_request, \
                state = CASE \
                    WHEN contacts.state = 'established' THEN 'established' \
                    WHEN contacts.incoming_request IS NOT NULL THEN 'established' \
                    ELSE excluded.state END",
        )?;
        for (key, entry) in &cs.sent_requests {
            let payload = blob::encode(&entry.request)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
                sent,
                payload,
            ])?;
        }
    }
    if !cs.removed_sent.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM contacts WHERE wallet_id = ?1 AND owner_id = ?2 AND contact_id = ?3",
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
        // Pending-received upsert, symmetric to the sent path.
        let received = contact_state_db_label(ContactState::Received);
        let mut stmt = tx.prepare_cached(
            "INSERT INTO contacts (wallet_id, owner_id, contact_id, state, incoming_request) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, owner_id, contact_id) DO UPDATE SET \
                incoming_request = excluded.incoming_request, \
                state = CASE \
                    WHEN contacts.state = 'established' THEN 'established' \
                    WHEN contacts.outgoing_request IS NOT NULL THEN 'established' \
                    ELSE excluded.state END",
        )?;
        for (key, entry) in &cs.incoming_requests {
            let payload = blob::encode(&entry.request)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.sender_id.as_slice(),
                received,
                payload,
            ])?;
        }
    }
    if !cs.removed_incoming.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM contacts WHERE wallet_id = ?1 AND owner_id = ?2 AND contact_id = ?3",
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
        // Collapse any prior pending row: both request blobs + the four
        // metadata columns, forcing the `established` state.
        let established_label = contact_state_db_label(ContactState::Established);
        let mut stmt = tx.prepare_cached(
            "INSERT INTO contacts \
                (wallet_id, owner_id, contact_id, state, outgoing_request, incoming_request, \
                 alias, note, is_hidden, accepted_accounts) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT(wallet_id, owner_id, contact_id) DO UPDATE SET \
                state = excluded.state, \
                outgoing_request = excluded.outgoing_request, \
                incoming_request = excluded.incoming_request, \
                alias = excluded.alias, \
                note = excluded.note, \
                is_hidden = excluded.is_hidden, \
                accepted_accounts = excluded.accepted_accounts",
        )?;
        for (key, established) in &cs.established {
            let outgoing = blob::encode(&established.outgoing_request)?;
            let incoming = blob::encode(&established.incoming_request)?;
            let accepted = blob::encode(&established.accepted_accounts)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                key.owner_id.as_slice(),
                key.recipient_id.as_slice(),
                established_label,
                outgoing,
                incoming,
                established.alias,
                established.note,
                established.is_hidden as i64,
                accepted,
            ])?;
        }
    }
    Ok(())
}

/// Build a [`ContactsRecords`] for one wallet from the unified
/// `contacts` table, bucketing rows by `state`. Any row that fails to
/// decode (bad blob, non-32-byte id, unknown state, or a pending row
/// missing its request blob) is a hard error — corruption is never
/// silently dropped.
pub(crate) fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<ContactsRecords, WalletStorageError> {
    let mut state = ContactsRecords::default();

    let mut stmt = conn.prepare(
        "SELECT owner_id, contact_id, state, outgoing_request, incoming_request, \
                alias, note, is_hidden, accepted_accounts \
         FROM contacts WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        let owner: Vec<u8> = row.get(0)?;
        let contact: Vec<u8> = row.get(1)?;
        let label: String = row.get(2)?;
        let outgoing: Option<Vec<u8>> = row.get(3)?;
        let incoming: Option<Vec<u8>> = row.get(4)?;
        let (owner_id, contact_id) = decode_pair_key(&owner, &contact)?;

        match contact_state_from_label(&label)? {
            ContactState::Sent => {
                let request = decode_request("outgoing_request", outgoing.as_deref())?;
                state.sent_requests.insert(
                    SentContactRequestKey {
                        owner_id,
                        recipient_id: contact_id,
                    },
                    ContactRequestEntry { request },
                );
            }
            ContactState::Received => {
                let request = decode_request("incoming_request", incoming.as_deref())?;
                state.incoming_requests.insert(
                    ReceivedContactRequestKey {
                        owner_id,
                        sender_id: contact_id,
                    },
                    ContactRequestEntry { request },
                );
            }
            ContactState::Established => {
                let outgoing_request = decode_request("outgoing_request", outgoing.as_deref())?;
                let incoming_request = decode_request("incoming_request", incoming.as_deref())?;
                let alias: Option<String> = row.get(5)?;
                let note: Option<String> = row.get(6)?;
                let is_hidden: bool = row.get::<_, Option<i64>>(7)?.unwrap_or(0) != 0;
                let accepted_blob: Option<Vec<u8>> = row.get(8)?;
                let accepted_accounts: Vec<u32> = match accepted_blob {
                    Some(bytes) => blob::decode(&bytes)?,
                    None => Vec::new(),
                };
                state.established.insert(
                    SentContactRequestKey {
                        owner_id,
                        recipient_id: contact_id,
                    },
                    EstablishedContact {
                        contact_identity_id: contact_id,
                        outgoing_request,
                        incoming_request,
                        alias,
                        note,
                        is_hidden,
                        accepted_accounts,
                    },
                );
            }
        }
    }

    Ok(state)
}

/// Build a keyless [`ContactChangeSet`] for one wallet — the
/// rehydration feed the manager layers onto the restored managed
/// identities. PUBLIC material only; `removed_*` are always empty
/// (deletes never reach storage as rows). Fail-hard on a corrupt row,
/// inherited from [`load_state`].
pub fn load_changeset(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<ContactChangeSet, WalletStorageError> {
    let records = load_state(conn, wallet_id)?;
    Ok(ContactChangeSet {
        sent_requests: records.sent_requests,
        incoming_requests: records.incoming_requests,
        established: records.established,
        removed_sent: Default::default(),
        removed_incoming: Default::default(),
    })
}

/// Decode a `ContactRequest` from a nullable request column. A NULL
/// column on a state that requires it (a pending row missing its blob,
/// or an established row missing either side) is a hard error — the
/// shape invariant is part of the on-disk contract.
fn decode_request(
    column: &'static str,
    bytes: Option<&[u8]>,
) -> Result<ContactRequest, WalletStorageError> {
    match bytes {
        Some(b) => blob::decode(b),
        None => Err(WalletStorageError::blob_decode(match column {
            "outgoing_request" => "contacts row is missing its outgoing_request blob",
            _ => "contacts row is missing its incoming_request blob",
        })),
    }
}

fn decode_pair_key(a: &[u8], b: &[u8]) -> Result<(Identifier, Identifier), WalletStorageError> {
    let a32 = <[u8; 32]>::try_from(a)
        .map_err(|_| WalletStorageError::blob_decode("contacts.id column is not 32 bytes"))?;
    let b32 = <[u8; 32]>::try_from(b)
        .map_err(|_| WalletStorageError::blob_decode("contacts.id column is not 32 bytes"))?;
    Ok((Identifier::from(a32), Identifier::from(b32)))
}

/// Test-helper wrapper over [`load_state`] without promoting the production
/// surface beyond `pub(crate)`.
#[cfg(feature = "__test-helpers")]
pub fn load_state_for_test(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<ContactsRecords, WalletStorageError> {
    load_state(conn, wallet_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every [`ContactState`] variant; the wildcard-free match below fails to
    /// compile if one is added.
    fn all_contact_state_variants() -> Vec<ContactState> {
        let variants = vec![
            ContactState::Sent,
            ContactState::Received,
            ContactState::Established,
        ];
        for v in &variants {
            match v {
                ContactState::Sent | ContactState::Received | ContactState::Established => {}
            }
        }
        variants
    }

    #[test]
    fn contact_state_labels_match_enum() {
        let from_writer: HashSet<&'static str> = all_contact_state_variants()
            .into_iter()
            .map(contact_state_db_label)
            .collect();
        let from_const: HashSet<&'static str> = CONTACT_STATE_LABELS.iter().copied().collect();
        assert_eq!(
            from_writer, from_const,
            "CONTACT_STATE_LABELS ({from_const:?}) drifted from contact_state_db_label codomain ({from_writer:?})"
        );
    }
}
