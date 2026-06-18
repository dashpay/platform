//! Contact request management for ManagedIdentity
//!
//! This module handles the bidirectional contact request flow:
//! - Sending contact requests (outgoing)
//! - Receiving contact requests (incoming)
//! - Automatically establishing contacts when both parties send requests

use super::ManagedIdentity;
use crate::changeset::{
    ContactChangeSet, ContactRequestEntry, ReceivedContactRequestKey, SentContactRequestKey,
};
use crate::wallet::identity::crypto::contact_info::ContactInfoPrivateData;
use crate::wallet::persister::WalletPersister;
use crate::{ContactRequest, EstablishedContact};
use dpp::prelude::Identifier;

impl ManagedIdentity {
    /// The masked `accountReference` of the most recent request WE sent
    /// to `recipient`, or `None` if we've never sent one.
    ///
    /// Load-bearing for rotation: the next request's rotation version
    /// is derived by un-masking this prior reference and bumping it. The
    /// prior request lives in `sent_contact_requests` while pending but is
    /// moved into `established_contacts[..].outgoing_request` once the
    /// contact establishes — and rotation (re-keying) happens precisely on
    /// an established relationship. Consulting only the pending map would
    /// return `None` for every established contact, resetting the version
    /// to 0 and reproducing the original reference, which the contract's
    /// `($ownerId, toUserId, accountReference)` unique index rejects. So
    /// this checks both maps.
    pub fn prior_sent_account_reference(&self, recipient: &Identifier) -> Option<u32> {
        self.sent_contact_requests
            .get(recipient)
            .map(|r| r.account_reference)
            .or_else(|| {
                self.established_contacts
                    .get(recipient)
                    .map(|c| c.outgoing_request.account_reference)
            })
    }

    /// Add a sent contact request.
    ///
    /// If there's already an incoming request from the recipient, the
    /// contact is auto-established. Persists the resulting
    /// [`ContactChangeSet`] via `persister` and returns `()`.
    ///
    /// **Sent-side ingest guard.** A recurring sweep re-ingests the
    /// identity's own sent requests on every pass; without a guard that
    /// would create a phantom pending-sent row + a changeset write per
    /// contact per sweep, and an `EstablishedContact::new` for an
    /// already-established pair would wipe the user's alias / note /
    /// hide-flag / accepted-accounts. So this method is a **no-op** when
    /// the recipient is already tracked as established or already in the
    /// sent map (symmetric to the received-side dedup in
    /// `sync_contact_requests`). When it must (re-)establish against a
    /// pre-existing incoming request, it MERGES into any existing
    /// `EstablishedContact` to preserve metadata.
    pub fn add_sent_contact_request(
        &mut self,
        request: ContactRequest,
        persister: &WalletPersister,
    ) {
        let owner_id = self.id();
        let recipient_id = request.recipient_id;

        // Sent-side guard: already established → nothing to do. The
        // on-platform request is immutable, so a re-ingest carries no new
        // information; re-establishing would wipe user metadata.
        if self.established_contacts.contains_key(&recipient_id) {
            return;
        }
        // Already tracked as a pending sent request → no-op (no phantom
        // row, no redundant changeset write).
        if self.sent_contact_requests.contains_key(&recipient_id) {
            return;
        }

        let mut cs = ContactChangeSet::default();

        // Check if there's already an incoming request from this recipient
        if let Some(incoming_request) = self.incoming_contact_requests.remove(&recipient_id) {
            // Automatically establish the contact — per the ContactChangeSet
            // auto-establishment contract, `established` implies the matching
            // pending entries are dropped, so we don't also emit a
            // `removed_incoming` tombstone here. Preserve metadata if a
            // prior `EstablishedContact` exists for this pair.
            let contact = match self.established_contacts.get(&recipient_id) {
                Some(existing) => EstablishedContact::reestablish_preserving_metadata(
                    existing,
                    request,
                    incoming_request,
                ),
                None => EstablishedContact::new(recipient_id, request, incoming_request),
            };
            cs.established.insert(
                SentContactRequestKey {
                    owner_id,
                    recipient_id,
                },
                contact.clone(),
            );
            self.established_contacts.insert(recipient_id, contact);
        } else {
            // No matching incoming request, just add as sent
            cs.sent_requests.insert(
                SentContactRequestKey {
                    owner_id,
                    recipient_id,
                },
                ContactRequestEntry {
                    request: request.clone(),
                },
            );
            self.sent_contact_requests.insert(recipient_id, request);
        }
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Ignore `sender_id` (per-sender mute, = block, reversible).
    ///
    /// Drops the sender's pending incoming entry (if present) and records
    /// the sender in `ignored_senders` so the recurring sync ingest path
    /// won't resurrect *any* of that sender's still-on-platform immutable
    /// `contactRequest` documents — including rotated ones with a bumped
    /// `accountReference`. Suppression is per-sender by design (unlike the
    /// old per-`(sender, accountReference)` reject). Returns the
    /// [`ContactChangeSet`] carrying the ignore (the caller is responsible
    /// for persisting it through the same write guard it holds).
    pub fn ignore_sender(&mut self, sender_id: &Identifier) -> ContactChangeSet {
        let owner_id = self.id();
        self.incoming_contact_requests.remove(sender_id);
        self.ignored_senders.insert(*sender_id);

        let mut cs = ContactChangeSet::default();
        // Emit `removed_incoming` too — NOT just the ignore entry. The
        // Rust SQLite contacts writer DELETEs the persisted
        // `state='received'` row only on a `removed_incoming` entry; its
        // `ignored` branch upserts solely the ignored-senders table.
        // Without this the ignored sender's row survives in SQLite and
        // rehydrates as a live incoming entry on the next load — the
        // user's ignore is silently undone on that backend. (The SwiftData
        // persister already deletes the row via its `ignored` handler, so
        // this makes the two backends consistent.)
        cs.removed_incoming.insert(ReceivedContactRequestKey {
            owner_id,
            sender_id: *sender_id,
        });
        cs.ignored.insert((owner_id, *sender_id));
        cs
    }

    /// Whether `sender_id` is ignored (per-sender). When `true`, ALL of
    /// the sender's incoming requests are suppressed from the main pending
    /// list — including rotated (bumped-`accountReference`) ones.
    pub fn is_sender_ignored(&self, sender_id: &Identifier) -> bool {
        self.ignored_senders.contains(sender_id)
    }

    /// Un-ignore `sender_id` (reverse [`Self::ignore_sender`]).
    ///
    /// Removes the sender from `ignored_senders` AND rewinds the received
    /// high-water cursor to `None`. The rewind is load-bearing: while the
    /// sender was ignored, the recurring sweep kept advancing the cursor
    /// past their on-chain requests, so without resetting it the next
    /// sweep's incremental `$createdAt >` query would never re-fetch them
    /// and the un-ignored sender's request would never reappear. `None`
    /// forces one full re-fetch (safe — ingest is a fixpoint).
    ///
    /// Returns a [`ContactChangeSet`] carrying the ignore tombstone removal
    /// (the caller persists it through its write guard). The cursor reset
    /// is in-memory only (the high-water mark is not itself persisted; it
    /// resets to `None` on cold restart anyway), so no changeset field is
    /// needed for it. A no-op (empty changeset) when the sender wasn't
    /// ignored.
    pub fn unignore_sender(&mut self, sender_id: &Identifier) -> ContactChangeSet {
        let owner_id = self.id();
        let was_ignored = self.ignored_senders.remove(sender_id);
        if !was_ignored {
            return ContactChangeSet::default();
        }
        // Rewind the receive cursor so the next sweep re-fetches the
        // now-un-ignored sender's on-chain requests.
        self.high_water_received_ms = None;

        let mut cs = ContactChangeSet::default();
        cs.unignored.insert((owner_id, *sender_id));
        cs
    }

    /// Remove a sent contact request.
    ///
    /// Returns the removed request (if any) and a tombstone changeset.
    pub fn remove_sent_contact_request(
        &mut self,
        recipient_id: &Identifier,
    ) -> (Option<ContactRequest>, ContactChangeSet) {
        let removed = self.sent_contact_requests.remove(recipient_id);
        let mut cs = ContactChangeSet::default();
        if removed.is_some() {
            cs.removed_sent.insert(SentContactRequestKey {
                owner_id: self.id(),
                recipient_id: *recipient_id,
            });
        }
        (removed, cs)
    }

    /// Add an incoming contact request.
    ///
    /// If there's already a sent request to the sender, the contact is
    /// auto-established. Persists the resulting [`ContactChangeSet`] via
    /// `persister` and returns `()`.
    pub fn add_incoming_contact_request(
        &mut self,
        request: ContactRequest,
        persister: &WalletPersister,
    ) {
        let owner_id = self.id();
        let sender_id = request.sender_id;
        let mut cs = ContactChangeSet::default();

        // Check if there's already a sent request to this sender
        if let Some(outgoing_request) = self.sent_contact_requests.remove(&sender_id) {
            // Automatically establish the contact — per the ContactChangeSet
            // auto-establishment contract, `established` implies the matching
            // pending entries are dropped, so we don't also emit a
            // `removed_sent` tombstone here. Preserve metadata if a prior
            // `EstablishedContact` exists for this pair (a recurring sweep
            // can re-ingest a reciprocal while the relationship already
            // exists — naive re-establish would wipe the user's metadata).
            let contact = match self.established_contacts.get(&sender_id) {
                Some(existing) => EstablishedContact::reestablish_preserving_metadata(
                    existing,
                    outgoing_request,
                    request,
                ),
                None => EstablishedContact::new(sender_id, outgoing_request, request),
            };
            cs.established.insert(
                SentContactRequestKey {
                    owner_id,
                    recipient_id: sender_id,
                },
                contact.clone(),
            );
            self.established_contacts.insert(sender_id, contact);
        } else {
            // No matching sent request, just add as incoming
            cs.incoming_requests.insert(
                ReceivedContactRequestKey {
                    owner_id,
                    sender_id,
                },
                ContactRequestEntry {
                    request: request.clone(),
                },
            );
            self.incoming_contact_requests.insert(sender_id, request);
        }
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Set the owner-private metadata (alias / note / hidden) on an
    /// established contact and persist the changeset.
    ///
    /// Takes the decrypted [`ContactInfoPrivateData`] payload directly —
    /// the same struct the `contactInfo` codec produces — so callers don't
    /// explode it into positional args. This is the local half of
    /// `contactInfo`: callers route user edits AND decrypted
    /// on-platform `contactInfo` payloads through here so SwiftData mirrors
    /// either source. The wire field names (`alias_name` / `display_hidden`)
    /// map onto the domain names (`alias` / `is_hidden`) on the contact.
    /// Returns `false` (no-op) when the contact isn't established.
    pub fn set_contact_metadata(
        &mut self,
        contact_id: &Identifier,
        metadata: ContactInfoPrivateData,
        persister: &WalletPersister,
    ) -> bool {
        let owner_id = self.id();
        let Some(contact) = self.established_contacts.get_mut(contact_id) else {
            return false;
        };
        if contact.alias == metadata.alias_name
            && contact.note == metadata.note
            && contact.is_hidden == metadata.display_hidden
        {
            // Unchanged — skip the persister round (the recurring sync
            // calls this for every decrypted doc on every pass).
            return true;
        }
        contact.alias = metadata.alias_name;
        contact.note = metadata.note;
        contact.is_hidden = metadata.display_hidden;

        let mut cs = ContactChangeSet::default();
        cs.established.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id: *contact_id,
            },
            contact.clone(),
        );
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
        true
    }

    /// Apply a **rotation** contact request (receive side, DIP-15
    /// §"sender rotated their addresses"): a request from a sender we
    /// already track, carrying a *different* `accountReference` than
    /// the tracked one. The new request supersedes the old —
    /// last-write-wins per pair; simultaneous multi-account
    /// relationships ride `accepted_accounts` later.
    ///
    /// - **Established contact**: replace `incoming_request` (the new
    ///   encrypted xpub + key indices) and clear
    ///   `payment_channel_broken` — a superseding request is exactly
    ///   the "wait for a new request" recovery the broken flag's
    ///   docs promise. The caller is responsible for tearing down the
    ///   stale external account so the build sweep re-registers it
    ///   from the new xpub.
    /// - **Pending incoming** (not yet accepted): replace the entry —
    ///   accepting later uses the freshest key material.
    ///
    /// No-op if the sender isn't tracked at all (callers route fresh
    /// requests through [`Self::add_incoming_contact_request`]).
    /// Persists the resulting changeset. Returns `true` when an
    /// established contact was re-keyed (the caller's signal to tear
    /// down the stale external account).
    pub fn apply_rotated_incoming_request(
        &mut self,
        request: ContactRequest,
        persister: &WalletPersister,
    ) -> bool {
        let owner_id = self.id();
        let sender_id = request.sender_id;

        // Idempotency guard: if the incoming request already stored for
        // this sender is byte-identical, this is a re-ingest of a doc we
        // already applied — do NOT persist a changeset or report a re-key.
        // The sync sweep collapses to the newest doc per sender, so this
        // shouldn't normally fire, but the state method must be safe to
        // call repeatedly with the same request without thrashing the
        // persister or re-tearing-down the external account.
        let already_applied = self
            .established_contacts
            .get(&sender_id)
            .map(|c| c.incoming_request == request)
            .or_else(|| {
                self.incoming_contact_requests
                    .get(&sender_id)
                    .map(|r| *r == request)
            })
            .unwrap_or(false);
        if already_applied {
            return false;
        }

        let mut cs = ContactChangeSet::default();

        let rekeyed_established =
            if let Some(contact) = self.established_contacts.get_mut(&sender_id) {
                tracing::info!(
                    owner = %owner_id,
                    sender = %sender_id,
                    old_reference = contact.incoming_request.account_reference,
                    new_reference = request.account_reference,
                    "Contact rotated their addresses — re-keying the established contact"
                );
                contact.incoming_request = request;
                contact.payment_channel_broken = false;
                cs.established.insert(
                    SentContactRequestKey {
                        owner_id,
                        recipient_id: sender_id,
                    },
                    contact.clone(),
                );
                true
            } else if let Some(slot) = self.incoming_contact_requests.get_mut(&sender_id) {
                // Pending (not-yet-accepted) incoming request — replace it
                // in place so a later Accept uses the freshest key material.
                *slot = request.clone();
                cs.incoming_requests.insert(
                    ReceivedContactRequestKey {
                        owner_id,
                        sender_id,
                    },
                    ContactRequestEntry { request },
                );
                false
            } else {
                return false;
            };

        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
        rekeyed_established
    }

    /// Remove an incoming contact request.
    ///
    /// Returns the removed request (if any) and a tombstone changeset.
    pub fn remove_incoming_contact_request(
        &mut self,
        sender_id: &Identifier,
    ) -> (Option<ContactRequest>, ContactChangeSet) {
        let removed = self.incoming_contact_requests.remove(sender_id);
        let mut cs = ContactChangeSet::default();
        if removed.is_some() {
            cs.removed_incoming.insert(ReceivedContactRequestKey {
                owner_id: self.id(),
                sender_id: *sender_id,
            });
        }
        (removed, cs)
    }

    /// Accept an incoming contact request and establish the contact.
    ///
    /// Returns the established contact (if both incoming and outgoing
    /// requests exist) and a changeset describing the transition. Returns
    /// `(None, empty)` without modifying state if either request is
    /// missing.
    pub fn accept_incoming_request(
        &mut self,
        sender_id: &Identifier,
    ) -> (Option<EstablishedContact>, ContactChangeSet) {
        // Check both exist before removing either (prevents data loss).
        if !self.incoming_contact_requests.contains_key(sender_id)
            || !self.sent_contact_requests.contains_key(sender_id)
        {
            return (None, ContactChangeSet::default());
        }
        // Both `remove` calls are guaranteed `Some` by the pre-check above.
        let incoming_request = self
            .incoming_contact_requests
            .remove(sender_id)
            .expect("incoming request presence checked above");
        let outgoing_request = self
            .sent_contact_requests
            .remove(sender_id)
            .expect("sent request presence checked above");

        // Create the established contact
        let contact = EstablishedContact::new(*sender_id, outgoing_request, incoming_request);

        // Add to established contacts
        self.established_contacts
            .insert(*sender_id, contact.clone());

        // Per the ContactChangeSet auto-establishment contract, `established`
        // implies the matching pending requests are dropped — no separate
        // `removed_sent` / `removed_incoming` emission needed here.
        let owner_id = self.id();
        let mut cs = ContactChangeSet::default();
        cs.established.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id: *sender_id,
            },
            contact.clone(),
        );

        (Some(contact), cs)
    }
}

// --- Apply (restore from changeset) ---

impl ManagedIdentity {
    /// Promote a contact to established during apply, also dropping any
    /// matching pending requests per the [`ContactChangeSet`](crate::changeset::ContactChangeSet)
    /// auto-establishment contract: when `established` is populated for
    /// `(owner, contact)`, the apply path MUST drop the matching
    /// entries from both `sent_contact_requests` and
    /// `incoming_contact_requests`.
    ///
    /// This is the only contact-side apply helper that earns its name —
    /// the trivial sent / incoming insert and remove paths are inlined
    /// at the call site in `wallet/apply.rs` (single map operation, no
    /// invariant to protect).
    pub(crate) fn apply_established_contact(&mut self, contact: EstablishedContact) {
        let contact_id = contact.contact_identity_id;
        self.sent_contact_requests.remove(&contact_id);
        self.incoming_contact_requests.remove(&contact_id);
        self.established_contacts.insert(contact_id, contact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

    fn create_test_identity(id_bytes: [u8; 32]) -> ManagedIdentity {
        let identity_v0 = IdentityV0 {
            id: Identifier::from(id_bytes),
            public_keys: BTreeMap::new(),
            balance: 1000,
            revision: 1,
        };
        ManagedIdentity::new(dpp::identity::Identity::V0(identity_v0), 0)
    }

    fn create_contact_request(
        sender_id: Identifier,
        recipient_id: Identifier,
        timestamp: u64,
    ) -> ContactRequest {
        ContactRequest::new(
            sender_id,
            recipient_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            timestamp,
        )
    }

    #[test]
    fn test_add_sent_contact_request_without_reciprocal() {
        let mut managed = create_test_identity([1u8; 32]);
        let recipient_id = Identifier::from([2u8; 32]);
        let sender_id = Identifier::from([1u8; 32]);
        let p = noop_persister();

        let request = create_contact_request(sender_id, recipient_id, 1234567890);

        managed.add_sent_contact_request(request.clone(), &p);

        // Should be in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert!(managed.sent_contact_requests.contains_key(&recipient_id));
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 0);
    }

    /// **Blocking — ignore must DELETE the persisted incoming row, not only
    /// record the suppression.** The Rust SQLite contacts writer issues
    /// `DELETE FROM contacts` only on a `removed_incoming` changeset entry;
    /// its `ignored` branch upserts solely the ignored-senders table. So if
    /// `ignore_sender` emits only `ignored`, the `state='received'` row
    /// survives in SQLite and the ignored request rehydrates as live on the
    /// next load — the user's ignore silently undone on that backend. Pin
    /// that BOTH are emitted.
    #[test]
    fn ignore_sender_emits_removed_incoming_so_sqlite_deletes_the_row() {
        let mut managed = create_test_identity([1u8; 32]);
        let owner_id = managed.id();
        let sender_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        managed.add_incoming_contact_request(create_contact_request(sender_id, owner_id, 1234), &p);
        assert_eq!(managed.incoming_contact_requests.len(), 1);

        let cs = managed.ignore_sender(&sender_id);

        // The per-sender ignore is recorded...
        assert!(
            cs.ignored.contains(&(owner_id, sender_id)),
            "ignore must record the per-sender suppression"
        );
        // ...AND the incoming-row deletion is emitted, so the SQLite writer
        // actually removes the persisted `state='received'` row.
        assert!(
            cs.removed_incoming.contains(&ReceivedContactRequestKey {
                owner_id,
                sender_id,
            }),
            "ignore must emit removed_incoming so the persisted contacts row is DELETEd"
        );
    }

    #[test]
    fn test_add_incoming_contact_request_without_reciprocal() {
        let mut managed = create_test_identity([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);
        let recipient_id = Identifier::from([1u8; 32]);
        let p = noop_persister();

        let request = create_contact_request(sender_id, recipient_id, 1234567890);

        managed.add_incoming_contact_request(request.clone(), &p);

        // Should be in incoming requests
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert!(managed.incoming_contact_requests.contains_key(&sender_id));
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 0);
    }

    #[test]
    fn test_add_sent_then_incoming_auto_establishes() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // Add sent request first
        let outgoing = create_contact_request(our_id, contact_id, 1234567890);
        managed.add_sent_contact_request(outgoing, &p);

        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add incoming request - should auto-establish
        let incoming = create_contact_request(contact_id, our_id, 1234567891);
        managed.add_incoming_contact_request(incoming, &p);

        // Requests should be moved to established contacts
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_add_incoming_then_sent_auto_establishes() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // Add incoming request first
        let incoming = create_contact_request(contact_id, our_id, 1234567890);
        managed.add_incoming_contact_request(incoming, &p);

        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add sent request - should auto-establish
        let outgoing = create_contact_request(our_id, contact_id, 1234567891);
        managed.add_sent_contact_request(outgoing, &p);

        // Requests should be moved to established contacts
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_remove_sent_contact_request() {
        let mut managed = create_test_identity([1u8; 32]);
        let recipient_id = Identifier::from([2u8; 32]);
        let sender_id = Identifier::from([1u8; 32]);
        let p = noop_persister();

        let request = create_contact_request(sender_id, recipient_id, 1234567890);
        managed.add_sent_contact_request(request.clone(), &p);

        assert_eq!(managed.sent_contact_requests.len(), 1);

        // Remove the request
        let (removed, cs) = managed.remove_sent_contact_request(&recipient_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().recipient_id, recipient_id);
        assert!(cs.removed_sent.contains(&SentContactRequestKey {
            owner_id: managed.id(),
            recipient_id
        }));
        assert_eq!(managed.sent_contact_requests.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_sent_request() {
        let mut managed = create_test_identity([1u8; 32]);
        let nonexistent_id = Identifier::from([99u8; 32]);

        let (removed, cs) = managed.remove_sent_contact_request(&nonexistent_id);
        assert!(removed.is_none());
        assert!(cs.removed_sent.is_empty());
    }

    #[test]
    fn test_remove_incoming_contact_request() {
        let mut managed = create_test_identity([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);
        let recipient_id = Identifier::from([1u8; 32]);
        let p = noop_persister();

        let request = create_contact_request(sender_id, recipient_id, 1234567890);
        managed.add_incoming_contact_request(request.clone(), &p);

        assert_eq!(managed.incoming_contact_requests.len(), 1);

        // Remove the request
        let (removed, cs) = managed.remove_incoming_contact_request(&sender_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().sender_id, sender_id);
        assert!(cs.removed_incoming.contains(&ReceivedContactRequestKey {
            owner_id: managed.id(),
            sender_id
        }));
        assert_eq!(managed.incoming_contact_requests.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_incoming_request() {
        let mut managed = create_test_identity([1u8; 32]);
        let nonexistent_id = Identifier::from([99u8; 32]);

        let (removed, cs) = managed.remove_incoming_contact_request(&nonexistent_id);
        assert!(removed.is_none());
        assert!(cs.removed_incoming.is_empty());
    }

    #[test]
    fn test_accept_incoming_request_success() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);

        // Add both requests without auto-establishment
        let outgoing = create_contact_request(our_id, contact_id, 1234567890);
        let incoming = create_contact_request(contact_id, our_id, 1234567891);

        managed.sent_contact_requests.insert(contact_id, outgoing);
        managed
            .incoming_contact_requests
            .insert(contact_id, incoming);

        // Accept the incoming request
        let (result, cs) = managed.accept_incoming_request(&contact_id);
        assert!(result.is_some());

        let contact = result.unwrap();
        assert_eq!(contact.contact_identity_id, contact_id);
        assert!(cs.established.contains_key(&SentContactRequestKey {
            owner_id: our_id,
            recipient_id: contact_id
        }));
        // Per the auto-establishment contract, `established` implies the
        // matching pending requests are dropped — no separate tombstones.
        assert!(cs.removed_sent.is_empty());
        assert!(cs.removed_incoming.is_empty());

        // Verify requests were removed and contact established
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_accept_incoming_request_missing_incoming() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);

        // Only add outgoing request
        let outgoing = create_contact_request(our_id, contact_id, 1234567890);
        managed.sent_contact_requests.insert(contact_id, outgoing);

        // Accept should fail - no incoming request
        let (result, cs) = managed.accept_incoming_request(&contact_id);
        assert!(result.is_none());
        assert!(<ContactChangeSet as crate::changeset::Merge>::is_empty(&cs));
    }

    #[test]
    fn test_accept_incoming_request_missing_outgoing() {
        let mut managed = create_test_identity([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // Only add incoming request
        let incoming = create_contact_request(contact_id, our_id, 1234567891);
        managed
            .incoming_contact_requests
            .insert(contact_id, incoming);

        // Accept should fail - no outgoing request
        let (result, cs) = managed.accept_incoming_request(&contact_id);
        assert!(result.is_none());
        assert!(<ContactChangeSet as crate::changeset::Merge>::is_empty(&cs));
    }

    /// Re-ingesting one's own already-tracked sent request must be a
    /// no-op — no phantom pending-sent row, no second changeset write. The
    /// sent-side guard mirrors the received-side dedup.
    #[test]
    fn test_add_sent_contact_request_is_idempotent_when_already_tracked() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let recipient_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        let request = create_contact_request(our_id, recipient_id, 1234567890);
        managed.add_sent_contact_request(request.clone(), &p);
        assert_eq!(managed.sent_contact_requests.len(), 1);

        // Re-ingest the SAME sent request (recurring sweep). It must not
        // create a duplicate / phantom row.
        managed.add_sent_contact_request(request, &p);
        assert_eq!(
            managed.sent_contact_requests.len(),
            1,
            "re-ingesting an already-tracked sent request must not duplicate it"
        );
        assert_eq!(managed.established_contacts.len(), 0);
    }

    /// Re-ingesting a sent request to an ALREADY-established contact
    /// must be a no-op — it must NOT wipe the existing contact's user
    /// metadata (alias/note/is_hidden/accepted_accounts).
    #[test]
    fn test_add_sent_contact_request_preserves_established_metadata() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // Establish a contact and attach user metadata.
        managed.add_incoming_contact_request(create_contact_request(contact_id, our_id, 1), &p);
        managed.add_sent_contact_request(create_contact_request(our_id, contact_id, 2), &p);
        assert_eq!(managed.established_contacts.len(), 1);
        let established = managed.established_contacts.get_mut(&contact_id).unwrap();
        established.set_alias("Alice".to_string());
        established.set_note("from work".to_string());
        established.hide();

        // Recurring sweep re-ingests our own sent request for an already
        // established contact — must not reset metadata.
        managed.add_sent_contact_request(create_contact_request(our_id, contact_id, 3), &p);

        let established = managed.established_contacts.get(&contact_id).unwrap();
        assert_eq!(established.alias, Some("Alice".to_string()));
        assert_eq!(established.note, Some("from work".to_string()));
        assert_eq!(established.is_hidden, true);
    }

    /// When a sent request auto-establishes against a pre-existing
    /// incoming, but the pair was previously established and we carry
    /// forward metadata — the re-establish must preserve it. (Covers the
    /// case where the incoming map still holds a request because a sweep
    /// re-ingested it.)
    #[test]
    fn test_reestablish_via_incoming_preserves_metadata() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // Establish, then attach metadata.
        managed.add_incoming_contact_request(create_contact_request(contact_id, our_id, 1), &p);
        managed.add_sent_contact_request(create_contact_request(our_id, contact_id, 2), &p);
        let est = managed.established_contacts.get_mut(&contact_id).unwrap();
        est.set_alias("Bob".to_string());

        // Simulate a re-ingested incoming reciprocal landing while a sent
        // request also exists in the map (forced state).
        managed
            .sent_contact_requests
            .insert(contact_id, create_contact_request(our_id, contact_id, 4));
        managed.add_incoming_contact_request(create_contact_request(contact_id, our_id, 5), &p);

        let est = managed.established_contacts.get(&contact_id).unwrap();
        assert_eq!(
            est.alias,
            Some("Bob".to_string()),
            "re-establish must preserve the alias"
        );
    }

    /// Ignoring a sender drops the pending incoming entry and records the
    /// sender in `ignored_senders` — per-sender, NOT per-accountReference.
    /// A rotated request (bumped `accountReference`) from the same sender
    /// is ALSO suppressed (the per-sender semantics).
    #[test]
    fn test_ignore_sender_suppresses_sender_per_sender() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        let mut request = create_contact_request(sender_id, our_id, 1);
        request.account_reference = 0;
        managed.add_incoming_contact_request(request, &p);
        assert_eq!(managed.incoming_contact_requests.len(), 1);

        let cs = managed.ignore_sender(&sender_id);

        // Incoming dropped, sender recorded as ignored.
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert!(managed.ignored_senders.contains(&sender_id));
        assert!(cs.ignored.contains(&(our_id, sender_id)));
        // The sender is ignored regardless of accountReference — both the
        // original (0) and a rotated (1) request are suppressed.
        assert!(managed.is_sender_ignored(&sender_id));
    }

    /// Un-ignoring a sender removes them from `ignored_senders`, rewinds
    /// the received high-water cursor to `None` (so the next sweep
    /// re-fetches their requests), and emits the un-ignore changeset.
    /// Un-ignoring a sender who wasn't ignored is a no-op (empty
    /// changeset, cursor untouched).
    #[test]
    fn test_unignore_sender_clears_cursor_and_removes() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);

        managed.ignore_sender(&sender_id);
        assert!(managed.is_sender_ignored(&sender_id));
        // Simulate the sweep having advanced the cursor past the sender's
        // requests while they were ignored.
        managed.high_water_received_ms = Some(123_456);

        let cs = managed.unignore_sender(&sender_id);

        assert!(!managed.is_sender_ignored(&sender_id));
        assert!(cs.unignored.contains(&(our_id, sender_id)));
        assert_eq!(
            managed.high_water_received_ms, None,
            "un-ignore must rewind the receive cursor so the sender's requests re-fetch"
        );

        // Un-ignoring again (no longer ignored) is a no-op and does NOT
        // touch the cursor a second time.
        managed.high_water_received_ms = Some(999);
        let cs2 = managed.unignore_sender(&sender_id);
        assert!(
            <ContactChangeSet as crate::changeset::Merge>::is_empty(&cs2),
            "un-ignoring a non-ignored sender must be a no-op"
        );
        assert_eq!(managed.high_water_received_ms, Some(999));
    }

    #[test]
    fn test_multiple_contact_requests() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact1_id = Identifier::from([2u8; 32]);
        let contact2_id = Identifier::from([3u8; 32]);
        let contact3_id = Identifier::from([4u8; 32]);
        let p = noop_persister();

        // Add multiple sent requests
        managed
            .add_sent_contact_request(create_contact_request(our_id, contact1_id, 1234567890), &p);
        managed
            .add_sent_contact_request(create_contact_request(our_id, contact2_id, 1234567891), &p);

        // Add incoming request that doesn't match sent
        managed.add_incoming_contact_request(
            create_contact_request(contact3_id, our_id, 1234567892),
            &p,
        );

        assert_eq!(managed.sent_contact_requests.len(), 2);
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add incoming from contact1 - should establish
        managed.add_incoming_contact_request(
            create_contact_request(contact1_id, our_id, 1234567893),
            &p,
        );

        assert_eq!(managed.sent_contact_requests.len(), 1); // Only contact2 left
        assert_eq!(managed.incoming_contact_requests.len(), 1); // Only contact3 left
        assert_eq!(managed.established_contacts.len(), 1); // contact1 established
        assert!(managed.established_contacts.contains_key(&contact1_id));
    }
}
