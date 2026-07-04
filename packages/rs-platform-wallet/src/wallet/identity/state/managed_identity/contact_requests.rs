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
    /// the recipient is already tracked as established with the SAME
    /// outgoing `accountReference` or already in the sent map (symmetric
    /// to the received-side dedup in `sync_contact_requests`). When it
    /// must (re-)establish against a pre-existing incoming request, it
    /// MERGES into any existing `EstablishedContact` to preserve metadata.
    ///
    /// **Sent-side rotation supersede.** When we re-send to an
    /// already-established contact with a *different* outgoing
    /// `accountReference` (a re-key: we rotated our own receiving xpub and
    /// broadcast a superseding request), the established contact's
    /// `outgoing_request` is advanced in place — the mirror of
    /// [`Self::apply_rotated_incoming_request`] on the receive side. Without
    /// this the tracked outgoing reference stays frozen at the first send,
    /// so the next rotation re-derives (un-mask → bump) from the stale
    /// version and collides with the already-broadcast reference on the
    /// contract's `($ownerId, toUserId, accountReference)` unique index,
    /// permanently breaking rotation after one use. User metadata is
    /// preserved (only `outgoing_request` moves).
    pub fn add_sent_contact_request(
        &mut self,
        request: ContactRequest,
        persister: &WalletPersister,
    ) -> Result<(), crate::changeset::PersistenceError> {
        let owner_id = self.id();
        let recipient_id = request.recipient_id;

        // Sent-side guard / rotation supersede: already established.
        if let Some(existing) = self.established_contacts.get(&recipient_id) {
            // Same outgoing reference → a re-ingest of a doc we already
            // track carries no new information; re-establishing would wipe
            // user metadata, so it's a no-op.
            if existing.outgoing_request.account_reference == request.account_reference {
                return Ok(());
            }
            // Different outgoing reference → a rotation re-send. Advance
            // `outgoing_request` so the next rotation reads the fresh
            // version, preserving all user metadata. Persist BEFORE
            // committing to memory (same order as `set_contact_metadata`):
            // if the store fails, memory must stay on the old reference so
            // the retry sweep doesn't hit the same-reference no-op guard
            // above and silently lose the rotation from disk for the
            // process lifetime.
            let mut updated = existing.clone();
            updated.outgoing_request = request;
            let mut cs = ContactChangeSet::default();
            cs.established.insert(
                SentContactRequestKey {
                    owner_id,
                    recipient_id,
                },
                updated.clone(),
            );
            persister.store(cs.into())?;
            self.established_contacts.insert(recipient_id, updated);
            return Ok(());
        }
        // Already tracked as a pending sent request → no-op (no phantom
        // row, no redundant changeset write).
        if self.sent_contact_requests.contains_key(&recipient_id) {
            return Ok(());
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
        persister.store(cs.into())?;
        Ok(())
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

    /// Marker key for the auto-accept verify-failed set:
    /// `SHA256(sender_id ‖ proof_bytes)`. Keying on the proof bytes (not the
    /// sender alone) means a DIFFERENT proof from the same sender is not
    /// suppressed by a prior bad one.
    pub fn auto_accept_verify_failed_key(sender_id: &Identifier, proof: &[u8]) -> [u8; 32] {
        use dashcore::hashes::{sha256, Hash, HashEngine};
        let mut engine = sha256::Hash::engine();
        engine.input(&sender_id.to_buffer());
        engine.input(proof);
        sha256::Hash::from_engine(engine).to_byte_array()
    }

    /// Hard cap on [`Self::auto_accept_verify_failed`] (32 KiB of keys at
    /// most). Every entry is attacker-funded (one distinct malformed
    /// `contactRequest` document per key, each costing platform credits to
    /// publish), so the set stays tiny in any legitimate wallet; the cap only
    /// bounds memory against a griefer willing to keep paying. Evicting an
    /// arbitrary entry over cap is safe — an evicted proof merely re-verifies
    /// (and re-fails) on the next sweep, which the per-launch-retry design
    /// already treats as harmless.
    pub const AUTO_ACCEPT_VERIFY_FAILED_CAP: usize = 1024;

    /// Record that `proof` (from `sender_id`) failed cryptographic verification
    /// permanently, so the sync sweep's enqueue gate does not re-queue it. Only
    /// PERMANENT failures should be marked; transient ones must stay retryable.
    /// In-memory only — cleared on relaunch (retry once per launch). Bounded by
    /// [`Self::AUTO_ACCEPT_VERIFY_FAILED_CAP`]: over cap, an arbitrary existing
    /// entry is evicted to make room (see the cap docs for why that is safe).
    pub fn mark_auto_accept_verify_failed(&mut self, sender_id: &Identifier, proof: &[u8]) {
        while self.auto_accept_verify_failed.len() >= Self::AUTO_ACCEPT_VERIFY_FAILED_CAP {
            self.auto_accept_verify_failed.pop_first();
        }
        self.auto_accept_verify_failed
            .insert(Self::auto_accept_verify_failed_key(sender_id, proof));
    }

    /// Whether `proof` (from `sender_id`) has already failed verification this
    /// launch — the enqueue gate consults this before re-queuing an
    /// `AutoAccept` op.
    pub fn is_auto_accept_verify_failed(&self, sender_id: &Identifier, proof: &[u8]) -> bool {
        self.auto_accept_verify_failed
            .contains(&Self::auto_accept_verify_failed_key(sender_id, proof))
    }

    /// Whether an inbound `request` from `sender_id` should be queued for the
    /// next signer-present auto-accept drain. The signerless sweep can only do
    /// cheap local checks here — the cryptographic verify runs in the drain:
    ///
    /// 1. Not already an established contact.
    /// 2. Carries a structurally-valid `autoAcceptProof` (DIP-15 size band +
    ///    the ECDSA key-type lead byte). The lower bound is the exact ECDSA
    ///    proof length — `key_type(1) + timestamp(4) + sig_size(1) +
    ///    signature(64) = 70` — so a shorter `0x00`-led byte run can't burn a
    ///    drain round-trip before being discarded as malformed.
    /// 3. Not a proof a prior drain already rejected cryptographically this
    ///    launch (see [`Self::is_auto_accept_verify_failed`]). Keyed by
    ///    `(sender, proof)`, so a DIFFERENT proof from the same sender still
    ///    enqueues — only the exact bad blob is suppressed. Without this an
    ///    attacker-published garbage proof would re-enqueue every sweep,
    ///    keeping the "waiting to finish setup" banner permanently tripped.
    pub fn should_enqueue_auto_accept(
        &self,
        sender_id: &Identifier,
        request: &ContactRequest,
    ) -> bool {
        if self.established_contacts.contains_key(sender_id) {
            return false;
        }
        let Some(proof) = request.auto_accept_proof.as_deref() else {
            return false;
        };
        if !((70..=102).contains(&proof.len()) && proof[0] == 0x00) {
            return false;
        }
        !self.is_auto_accept_verify_failed(sender_id, proof)
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
    ) -> Result<(), crate::changeset::PersistenceError> {
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
        persister.store(cs.into())?;
        Ok(())
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
    /// `Ok(false)` = no-op (contact isn't established); `Ok(true)` = applied
    /// (or unchanged, skipping the persist). `Err` = the persist failed; the
    /// in-memory contact is left UNCHANGED (we persist before committing to
    /// memory), so a retry is not defeated by the unchanged-equality
    /// short-circuit above. The caller MUST surface it so the drain leaves the
    /// `ContactInfoDecrypt` queue entry in place for that retry.
    pub fn set_contact_metadata(
        &mut self,
        contact_id: &Identifier,
        metadata: ContactInfoPrivateData,
        persister: &WalletPersister,
    ) -> Result<bool, crate::changeset::PersistenceError> {
        let owner_id = self.id();
        let Some(contact) = self.established_contacts.get_mut(contact_id) else {
            return Ok(false);
        };
        if contact.alias == metadata.alias_name
            && contact.note == metadata.note
            && contact.is_hidden == metadata.display_hidden
        {
            // Unchanged — skip the persister round (the recurring sync
            // calls this for every decrypted doc on every pass).
            return Ok(true);
        }
        // Persist BEFORE committing to memory: build the changeset from a copy
        // carrying the new metadata, store it, and only apply to the live
        // contact once the store succeeds. On a store failure memory stays
        // equal to the persisted state, so the unchanged-equality short-circuit
        // above does NOT swallow the next retry — the drain's ContactInfoDecrypt
        // entry re-runs and re-persists cleanly.
        let mut updated = contact.clone();
        updated.alias = metadata.alias_name;
        updated.note = metadata.note;
        updated.is_hidden = metadata.display_hidden;

        let mut cs = ContactChangeSet::default();
        cs.established.insert(
            SentContactRequestKey {
                owner_id,
                recipient_id: *contact_id,
            },
            updated.clone(),
        );
        persister.store(cs.into())?;
        *contact = updated;
        Ok(true)
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
    ) -> Result<bool, crate::changeset::PersistenceError> {
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
            return Ok(false);
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
                // The label belongs to the incoming request being replaced —
                // drop it so the rebuilt external account re-derives it from
                // the new request rather than showing the old label against
                // fresh key material.
                contact.contact_account_label = None;
                // The stale external account (built from the old reference)
                // is torn down by the caller — reset the marker so the build
                // sweep re-registers from the new xpub and re-stamps it.
                contact.external_account_reference = None;
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
                return Ok(false);
            };

        persister.store(cs.into())?;
        Ok(rekeyed_established)
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

    /// A persister whose every `store` fails — pins that a mutator surfaces the
    /// failure instead of swallowing it.
    fn failing_persister() -> WalletPersister {
        use crate::changeset::{
            ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
        };
        use crate::wallet::platform_wallet::WalletId;
        struct Failing;
        impl PlatformWalletPersistence for Failing {
            fn store(
                &self,
                _wallet_id: WalletId,
                _changeset: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                Err(PersistenceError::backend("store armed to fail"))
            }
            fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }
        WalletPersister::new([0u8; 32], Arc::new(Failing))
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

        managed
            .add_sent_contact_request(request.clone(), &p)
            .expect("setup persists");

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

        managed
            .add_incoming_contact_request(create_contact_request(sender_id, owner_id, 1234), &p)
            .expect("setup persists");
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

        managed
            .add_incoming_contact_request(request.clone(), &p)
            .expect("setup persists");

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
        managed
            .add_sent_contact_request(outgoing, &p)
            .expect("setup persists");

        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add incoming request - should auto-establish
        let incoming = create_contact_request(contact_id, our_id, 1234567891);
        managed
            .add_incoming_contact_request(incoming, &p)
            .expect("setup persists");

        // Requests should be moved to established contacts
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    /// DIP-15 §8.5 receive-side label: a contact's account label is derived
    /// from their incoming request, so a rotation (new incoming request →
    /// new key material, stale external account torn down + rebuilt) MUST
    /// drop the old label — otherwise the rebuilt channel would show the old
    /// contact's label. Pins that the in-place rotation path clears
    /// `contact_account_label` where it clears `payment_channel_broken` (the
    /// constructor-based reset is unreachable for an already-established
    /// contact, so this is the load-bearing reset).
    #[test]
    fn rotation_resets_contact_account_label() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // An established contact carrying a stale label and a broken channel.
        let outgoing = create_contact_request(our_id, contact_id, 1000);
        let incoming = create_contact_request(contact_id, our_id, 1001);
        let mut established = EstablishedContact::new(contact_id, outgoing, incoming);
        established.contact_account_label = Some("Old label".to_string());
        established.payment_channel_broken = true;
        managed.established_contacts.insert(contact_id, established);

        // A superseding incoming request rotates the relationship.
        let rotated = create_contact_request(contact_id, our_id, 2000);
        let rekeyed = managed
            .apply_rotated_incoming_request(rotated, &p)
            .expect("rotation persists in test");

        assert!(
            rekeyed,
            "a superseding incoming request must re-key the established contact"
        );
        let contact = managed
            .established_contacts
            .get(&contact_id)
            .expect("still established after rotation");
        assert_eq!(
            contact.contact_account_label, None,
            "rotation must drop the stale label so it re-derives from the new request"
        );
        assert!(
            !contact.payment_channel_broken,
            "rotation clears the broken flag (existing behavior, regression guard)"
        );
    }

    /// A failed metadata persist must SURFACE as `Err`, not return `Ok(true)` —
    /// else the alias/note is lost on restart AND the drain clears the only
    /// retry hook. The pre-fix `set_contact_metadata` logged + returned `true`.
    #[test]
    fn set_contact_metadata_surfaces_persist_failure() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let outgoing = create_contact_request(our_id, contact_id, 1000);
        let incoming = create_contact_request(contact_id, our_id, 1001);
        managed.established_contacts.insert(
            contact_id,
            EstablishedContact::new(contact_id, outgoing, incoming),
        );

        let result = managed.set_contact_metadata(
            &contact_id,
            ContactInfoPrivateData {
                alias_name: Some("New alias".to_string()),
                note: None,
                display_hidden: false,
                accepted_accounts: Vec::new(),
            },
            &failing_persister(),
        );
        assert!(
            result.is_err(),
            "a failed metadata persist must surface, not report success"
        );
    }

    /// A rotation supersede whose persist fails must leave the in-memory
    /// `outgoing_request` on the OLD reference — else the retry hits the
    /// same-reference no-op guard and the rotation is silently lost from disk
    /// for the process lifetime (only a restart re-fetching from platform
    /// would heal it). Mirror of the `set_contact_metadata` persist-order
    /// tests below.
    #[test]
    fn rotation_supersede_failed_persist_leaves_memory_on_old_reference() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let outgoing = create_contact_request(our_id, contact_id, 1000);
        let incoming = create_contact_request(contact_id, our_id, 1001);
        managed.established_contacts.insert(
            contact_id,
            EstablishedContact::new(contact_id, outgoing.clone(), incoming),
        );

        // A superseding re-send with a bumped account_reference.
        let mut rotated = create_contact_request(our_id, contact_id, 1002);
        rotated.account_reference = outgoing.account_reference + 1;

        let result = managed.add_sent_contact_request(rotated.clone(), &failing_persister());
        assert!(result.is_err(), "a failed supersede persist must surface");
        assert_eq!(
            managed.established_contacts[&contact_id]
                .outgoing_request
                .account_reference,
            outgoing.account_reference,
            "memory must stay on the old reference so the retry re-persists"
        );

        // The retry against a working persister must take the supersede path
        // (not the same-reference no-op) and commit the rotation.
        managed
            .add_sent_contact_request(rotated.clone(), &noop_persister())
            .expect("retry persists");
        assert_eq!(
            managed.established_contacts[&contact_id]
                .outgoing_request
                .account_reference,
            rotated.account_reference,
            "the retried rotation must land in memory"
        );
    }

    /// A retry after a failed metadata persist must actually re-store — it must
    /// NOT be swallowed by the unchanged-equality short-circuit. Pre-fix,
    /// `set_contact_metadata` mutated memory BEFORE persisting, so a failed
    /// store left memory == metadata; the next retry hit the equality
    /// short-circuit and returned `Ok(true)` WITHOUT persisting, permanently
    /// losing the alias/note.
    #[test]
    fn set_contact_metadata_retry_after_failed_persist_actually_persists() {
        use crate::changeset::{
            ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
        };
        use crate::wallet::platform_wallet::WalletId;
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct Counting(Arc<AtomicUsize>);
        impl PlatformWalletPersistence for Counting {
            fn store(
                &self,
                _w: WalletId,
                _cs: PlatformWalletChangeSet,
            ) -> Result<(), PersistenceError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
            fn flush(&self, _w: WalletId) -> Result<(), PersistenceError> {
                Ok(())
            }
            fn load(&self) -> Result<ClientStartState, PersistenceError> {
                Ok(ClientStartState::default())
            }
        }

        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let outgoing = create_contact_request(our_id, contact_id, 1000);
        let incoming = create_contact_request(contact_id, our_id, 1001);
        managed.established_contacts.insert(
            contact_id,
            EstablishedContact::new(contact_id, outgoing, incoming),
        );
        let meta = ContactInfoPrivateData {
            alias_name: Some("New alias".to_string()),
            note: None,
            display_hidden: false,
            accepted_accounts: Vec::new(),
        };

        // First attempt fails to persist → surfaces as Err.
        let r1 = managed.set_contact_metadata(&contact_id, meta.clone(), &failing_persister());
        assert!(
            r1.is_err(),
            "first attempt must surface the persist failure"
        );

        // Retry with the SAME metadata against a working persister. The fix
        // (persist-before-commit) leaves memory unchanged on the failed
        // attempt, so this retry must NOT short-circuit — it must re-store.
        let count = Arc::new(AtomicUsize::new(0));
        let working = WalletPersister::new([0u8; 32], Arc::new(Counting(count.clone())));
        let r2 = managed.set_contact_metadata(&contact_id, meta, &working);
        assert!(r2.is_ok(), "retry must succeed");
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "retry after a failed persist must actually re-store, not be swallowed \
             by the unchanged-equality short-circuit"
        );
        assert_eq!(
            managed
                .established_contacts
                .get(&contact_id)
                .unwrap()
                .alias
                .as_deref(),
            Some("New alias"),
            "memory must reflect the persisted alias after a successful retry"
        );
    }

    #[test]
    fn test_add_incoming_then_sent_auto_establishes() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // Add incoming request first
        let incoming = create_contact_request(contact_id, our_id, 1234567890);
        managed
            .add_incoming_contact_request(incoming, &p)
            .expect("setup persists");

        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add sent request - should auto-establish
        let outgoing = create_contact_request(our_id, contact_id, 1234567891);
        managed
            .add_sent_contact_request(outgoing, &p)
            .expect("setup persists");

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
        managed
            .add_sent_contact_request(request.clone(), &p)
            .expect("setup persists");

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
        managed
            .add_incoming_contact_request(request.clone(), &p)
            .expect("setup persists");

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
        managed
            .add_sent_contact_request(request.clone(), &p)
            .expect("setup persists");
        assert_eq!(managed.sent_contact_requests.len(), 1);

        // Re-ingest the SAME sent request (recurring sweep). It must not
        // create a duplicate / phantom row.
        managed
            .add_sent_contact_request(request, &p)
            .expect("setup persists");
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
        managed
            .add_incoming_contact_request(create_contact_request(contact_id, our_id, 1), &p)
            .expect("setup persists");
        managed
            .add_sent_contact_request(create_contact_request(our_id, contact_id, 2), &p)
            .expect("setup persists");
        assert_eq!(managed.established_contacts.len(), 1);
        let established = managed.established_contacts.get_mut(&contact_id).unwrap();
        established.set_alias("Alice".to_string());
        established.set_note("from work".to_string());
        established.hide();

        // Recurring sweep re-ingests our own sent request for an already
        // established contact — must not reset metadata.
        managed
            .add_sent_contact_request(create_contact_request(our_id, contact_id, 3), &p)
            .expect("setup persists");

        let established = managed.established_contacts.get(&contact_id).unwrap();
        assert_eq!(established.alias, Some("Alice".to_string()));
        assert_eq!(established.note, Some("from work".to_string()));
        assert_eq!(established.is_hidden, true);
    }

    /// Sent-side rotation supersede: re-sending to an already-established
    /// contact with a DIFFERENT outgoing `accountReference` must advance
    /// the established contact's `outgoing_request` in place (mirroring the
    /// receive-side `apply_rotated_incoming_request`) while preserving user
    /// metadata. Without this the tracked outgoing reference freezes at the
    /// first send and the next rotation re-derives the same reference,
    /// colliding on the contract's unique index. Two consecutive rotation
    /// re-sends must therefore be tracked with DISTINCT references, and the
    /// tracked reference must match the newest re-send.
    #[test]
    fn add_sent_contact_request_rotation_supersedes_outgoing_reference() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact_id = Identifier::from([2u8; 32]);
        let p = noop_persister();

        // Establish, then attach user metadata.
        managed
            .add_incoming_contact_request(create_contact_request(contact_id, our_id, 1), &p)
            .expect("setup persists");
        let mut first_send = create_contact_request(our_id, contact_id, 2);
        first_send.account_reference = 100; // R0
        managed
            .add_sent_contact_request(first_send, &p)
            .expect("setup persists");
        assert_eq!(managed.established_contacts.len(), 1);
        let est = managed.established_contacts.get_mut(&contact_id).unwrap();
        est.set_alias("Carol".to_string());
        assert_eq!(est.outgoing_request.account_reference, 100);

        // Rotation #1: re-send with a bumped reference R1.
        let mut rotation1 = create_contact_request(our_id, contact_id, 3);
        rotation1.account_reference = 101; // R1
        managed
            .add_sent_contact_request(rotation1, &p)
            .expect("rotation #1 persists");
        assert_eq!(
            managed
                .established_contacts
                .get(&contact_id)
                .unwrap()
                .outgoing_request
                .account_reference,
            101,
            "rotation #1 must advance the tracked outgoing reference (not freeze at R0)"
        );

        // Rotation #2: re-send with another bumped reference R2.
        let mut rotation2 = create_contact_request(our_id, contact_id, 4);
        rotation2.account_reference = 102; // R2
        managed
            .add_sent_contact_request(rotation2, &p)
            .expect("rotation #2 persists");
        let est = managed.established_contacts.get(&contact_id).unwrap();
        assert_eq!(
            est.outgoing_request.account_reference, 102,
            "rotation #2 must advance to the newest reference — distinct across sends"
        );
        // User metadata survives the rotations.
        assert_eq!(est.alias, Some("Carol".to_string()));
        // Re-ingesting the SAME (newest) reference is a metadata-preserving
        // no-op (the same-reference guard).
        let mut resend_same = create_contact_request(our_id, contact_id, 5);
        resend_same.account_reference = 102;
        managed
            .add_sent_contact_request(resend_same, &p)
            .expect("same-reference re-ingest is a no-op");
        let est = managed.established_contacts.get(&contact_id).unwrap();
        assert_eq!(est.outgoing_request.account_reference, 102);
        assert_eq!(est.alias, Some("Carol".to_string()));
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
        managed
            .add_incoming_contact_request(create_contact_request(contact_id, our_id, 1), &p)
            .expect("setup persists");
        managed
            .add_sent_contact_request(create_contact_request(our_id, contact_id, 2), &p)
            .expect("setup persists");
        let est = managed.established_contacts.get_mut(&contact_id).unwrap();
        est.set_alias("Bob".to_string());

        // Simulate a re-ingested incoming reciprocal landing while a sent
        // request also exists in the map (forced state).
        managed
            .sent_contact_requests
            .insert(contact_id, create_contact_request(our_id, contact_id, 4));
        managed
            .add_incoming_contact_request(create_contact_request(contact_id, our_id, 5), &p)
            .expect("setup persists");

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
        managed
            .add_incoming_contact_request(request, &p)
            .expect("setup persists");
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
            .add_sent_contact_request(create_contact_request(our_id, contact1_id, 1234567890), &p)
            .expect("setup persists");
        managed
            .add_sent_contact_request(create_contact_request(our_id, contact2_id, 1234567891), &p)
            .expect("setup persists");

        // Add incoming request that doesn't match sent
        managed
            .add_incoming_contact_request(
                create_contact_request(contact3_id, our_id, 1234567892),
                &p,
            )
            .expect("setup persists");

        assert_eq!(managed.sent_contact_requests.len(), 2);
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add incoming from contact1 - should establish
        managed
            .add_incoming_contact_request(
                create_contact_request(contact1_id, our_id, 1234567893),
                &p,
            )
            .expect("setup persists");

        assert_eq!(managed.sent_contact_requests.len(), 1); // Only contact2 left
        assert_eq!(managed.incoming_contact_requests.len(), 1); // Only contact3 left
        assert_eq!(managed.established_contacts.len(), 1); // contact1 established
        assert!(managed.established_contacts.contains_key(&contact1_id));
    }

    /// A structurally-valid but cryptographically-garbage `autoAcceptProof`
    /// on an attacker-published contact request must be enqueued at most once:
    /// after the drain marks it verify-failed, the next sweep's enqueue gate
    /// must NOT re-pick it — otherwise the "waiting to finish setup" banner is
    /// permanently re-tripped. A DIFFERENT proof from the same sender is not
    /// suppressed by the prior bad one.
    #[test]
    fn verify_failed_auto_accept_proof_is_not_re_enqueued() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);

        // Minimal structurally-valid proof: 70 bytes, ECDSA lead byte 0x00.
        let mk_request = |proof: Vec<u8>| {
            let mut r = create_contact_request(sender_id, our_id, 1234567890);
            r.auto_accept_proof = Some(proof);
            r
        };
        let garbage = mk_request({
            let mut p = vec![0x11u8; 70];
            p[0] = 0x00;
            p
        });

        // First sweep: structurally valid + unmarked → enqueue.
        assert!(
            managed.should_enqueue_auto_accept(&sender_id, &garbage),
            "a structurally-valid, unmarked proof must enqueue"
        );

        // The drain fails cryptographic verification and marks it.
        let proof_bytes = garbage.auto_accept_proof.clone().unwrap();
        managed.mark_auto_accept_verify_failed(&sender_id, &proof_bytes);

        // Next sweep: the same bad proof must NOT be re-enqueued.
        assert!(
            !managed.should_enqueue_auto_accept(&sender_id, &garbage),
            "a proof a prior drain rejected must not be re-enqueued"
        );

        // A DIFFERENT proof from the SAME sender still enqueues — the marker
        // is keyed by (sender, proof), not the sender alone.
        let different = mk_request({
            let mut p = vec![0x22u8; 70];
            p[0] = 0x00;
            p
        });
        assert!(
            managed.should_enqueue_auto_accept(&sender_id, &different),
            "a different proof from the same sender must still enqueue"
        );
    }
}
