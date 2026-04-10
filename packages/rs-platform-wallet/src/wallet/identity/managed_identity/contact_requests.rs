//! Contact request management for ManagedIdentity
//!
//! This module handles the bidirectional contact request flow:
//! - Sending contact requests (outgoing)
//! - Receiving contact requests (incoming)
//! - Automatically establishing contacts when both parties send requests

use super::ManagedIdentity;
use crate::changeset::{ContactChangeSet, ContactRequestEntry};
use crate::{ContactRequest, EstablishedContact};
use dpp::prelude::Identifier;

impl ManagedIdentity {
    /// Add a sent contact request.
    ///
    /// If there's already an incoming request from the recipient, the
    /// contact is auto-established and both requests are tombstoned in
    /// the returned [`ContactChangeSet`].
    pub fn add_sent_contact_request(&mut self, request: ContactRequest) -> ContactChangeSet {
        let owner_id = self.id();
        let recipient_id = request.recipient_id;
        let mut cs = ContactChangeSet::default();

        // Check if there's already an incoming request from this recipient
        if let Some(incoming_request) = self.incoming_contact_requests.remove(&recipient_id) {
            // Automatically establish the contact — per the ContactChangeSet
            // auto-establishment contract, `established` implies the matching
            // pending entries are dropped, so we don't also emit a
            // `removed_incoming` tombstone here.
            let contact = EstablishedContact::new(recipient_id, request, incoming_request);
            self.established_contacts.insert(recipient_id, contact);
            cs.established.insert((owner_id, recipient_id));
        } else {
            // No matching incoming request, just add as sent
            cs.sent_requests.insert(
                (owner_id, recipient_id),
                ContactRequestEntry {
                    request: request.clone(),
                },
            );
            self.sent_contact_requests.insert(recipient_id, request);
        }
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
            cs.removed_sent.insert((self.id(), *recipient_id));
        }
        (removed, cs)
    }

    /// Add an incoming contact request.
    ///
    /// If there's already a sent request to the sender, the contact is
    /// auto-established and both requests are tombstoned in the returned
    /// [`ContactChangeSet`].
    pub fn add_incoming_contact_request(&mut self, request: ContactRequest) -> ContactChangeSet {
        let owner_id = self.id();
        let sender_id = request.sender_id;
        let mut cs = ContactChangeSet::default();

        // Check if there's already a sent request to this sender
        if let Some(outgoing_request) = self.sent_contact_requests.remove(&sender_id) {
            // Automatically establish the contact — per the ContactChangeSet
            // auto-establishment contract, `established` implies the matching
            // pending entries are dropped, so we don't also emit a
            // `removed_sent` tombstone here.
            let contact = EstablishedContact::new(sender_id, outgoing_request, request);
            self.established_contacts.insert(sender_id, contact);
            cs.established.insert((owner_id, sender_id));
        } else {
            // No matching sent request, just add as incoming
            cs.incoming_requests.insert(
                (owner_id, sender_id),
                ContactRequestEntry {
                    request: request.clone(),
                },
            );
            self.incoming_contact_requests.insert(sender_id, request);
        }
        cs
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
            cs.removed_incoming.insert((self.id(), *sender_id));
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
        cs.established.insert((owner_id, *sender_id));

        (Some(contact), cs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

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

        let request = create_contact_request(sender_id, recipient_id, 1234567890);

        managed.add_sent_contact_request(request.clone());

        // Should be in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert!(managed.sent_contact_requests.contains_key(&recipient_id));
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 0);
    }

    #[test]
    fn test_add_incoming_contact_request_without_reciprocal() {
        let mut managed = create_test_identity([1u8; 32]);
        let sender_id = Identifier::from([2u8; 32]);
        let recipient_id = Identifier::from([1u8; 32]);

        let request = create_contact_request(sender_id, recipient_id, 1234567890);

        managed.add_incoming_contact_request(request.clone());

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

        // Add sent request first
        let outgoing = create_contact_request(our_id, contact_id, 1234567890);
        managed.add_sent_contact_request(outgoing);

        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add incoming request - should auto-establish
        let incoming = create_contact_request(contact_id, our_id, 1234567891);
        managed.add_incoming_contact_request(incoming);

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

        // Add incoming request first
        let incoming = create_contact_request(contact_id, our_id, 1234567890);
        managed.add_incoming_contact_request(incoming);

        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add sent request - should auto-establish
        let outgoing = create_contact_request(our_id, contact_id, 1234567891);
        managed.add_sent_contact_request(outgoing);

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

        let request = create_contact_request(sender_id, recipient_id, 1234567890);
        managed.add_sent_contact_request(request.clone());

        assert_eq!(managed.sent_contact_requests.len(), 1);

        // Remove the request
        let (removed, cs) = managed.remove_sent_contact_request(&recipient_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().recipient_id, recipient_id);
        assert!(cs
            .removed_sent
            .contains(&(managed.id(), recipient_id)));
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

        let request = create_contact_request(sender_id, recipient_id, 1234567890);
        managed.add_incoming_contact_request(request.clone());

        assert_eq!(managed.incoming_contact_requests.len(), 1);

        // Remove the request
        let (removed, cs) = managed.remove_incoming_contact_request(&sender_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().sender_id, sender_id);
        assert!(cs.removed_incoming.contains(&(managed.id(), sender_id)));
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
        assert!(cs.established.contains(&(our_id, contact_id)));
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

    #[test]
    fn test_multiple_contact_requests() {
        let mut managed = create_test_identity([1u8; 32]);
        let our_id = Identifier::from([1u8; 32]);
        let contact1_id = Identifier::from([2u8; 32]);
        let contact2_id = Identifier::from([3u8; 32]);
        let contact3_id = Identifier::from([4u8; 32]);

        // Add multiple sent requests
        managed.add_sent_contact_request(create_contact_request(our_id, contact1_id, 1234567890));
        managed.add_sent_contact_request(create_contact_request(our_id, contact2_id, 1234567891));

        // Add incoming request that doesn't match sent
        managed.add_incoming_contact_request(create_contact_request(
            contact3_id,
            our_id,
            1234567892,
        ));

        assert_eq!(managed.sent_contact_requests.len(), 2);
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add incoming from contact1 - should establish
        managed.add_incoming_contact_request(create_contact_request(
            contact1_id,
            our_id,
            1234567893,
        ));

        assert_eq!(managed.sent_contact_requests.len(), 1); // Only contact2 left
        assert_eq!(managed.incoming_contact_requests.len(), 1); // Only contact3 left
        assert_eq!(managed.established_contacts.len(), 1); // contact1 established
        assert!(managed.established_contacts.contains_key(&contact1_id));
    }
}
