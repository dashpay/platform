//! Contact request management for ManagedIdentity
//!
//! This module handles the bidirectional contact request flow:
//! - Sending contact requests (outgoing)
//! - Receiving contact requests (incoming)
//! - Automatically establishing contacts when both parties send requests

use super::ManagedIdentity;
use crate::{ContactRequest, EstablishedContact};
use dpp::prelude::Identifier;

impl ManagedIdentity {
    /// Add a sent contact request
    /// If there's already an incoming request from the recipient, automatically establish the contact
    pub fn add_sent_contact_request(&mut self, request: ContactRequest) {
        let recipient_id = request.recipient_id;

        // Check if there's already an incoming request from this recipient
        if let Some(incoming_request) = self.incoming_contact_requests.remove(&recipient_id) {
            // Automatically establish the contact
            let contact = EstablishedContact::new(recipient_id, request, incoming_request);
            self.established_contacts.insert(recipient_id, contact);
        } else {
            // No matching incoming request, just add as sent
            self.sent_contact_requests.insert(recipient_id, request);
        }
    }

    /// Remove a sent contact request
    pub fn remove_sent_contact_request(
        &mut self,
        recipient_id: &Identifier,
    ) -> Option<ContactRequest> {
        self.sent_contact_requests.remove(recipient_id)
    }

    /// Add an incoming contact request
    /// If there's already a sent request to the sender, automatically establish the contact
    pub fn add_incoming_contact_request(&mut self, request: ContactRequest) {
        let sender_id = request.sender_id;

        // Check if there's already a sent request to this sender
        if let Some(outgoing_request) = self.sent_contact_requests.remove(&sender_id) {
            // Automatically establish the contact
            let contact = EstablishedContact::new(sender_id, outgoing_request, request);
            self.established_contacts.insert(sender_id, contact);
        } else {
            // No matching sent request, just add as incoming
            self.incoming_contact_requests.insert(sender_id, request);
        }
    }

    /// Remove an incoming contact request
    pub fn remove_incoming_contact_request(
        &mut self,
        sender_id: &Identifier,
    ) -> Option<ContactRequest> {
        self.incoming_contact_requests.remove(sender_id)
    }

    /// Accept an incoming contact request and establish the contact
    /// Returns the established contact if successful
    pub fn accept_incoming_request(
        &mut self,
        sender_id: &Identifier,
    ) -> Option<EstablishedContact> {
        // Remove both requests
        let incoming_request = self.incoming_contact_requests.remove(sender_id)?;
        let outgoing_request = self.sent_contact_requests.remove(sender_id)?;

        // Create the established contact
        let contact = EstablishedContact::new(*sender_id, outgoing_request, incoming_request);

        // Add to established contacts
        self.established_contacts
            .insert(*sender_id, contact.clone());

        Some(contact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    fn create_test_identity(id_bytes: [u8; 32]) -> super::super::ManagedIdentity {
        let identity_v0 = IdentityV0 {
            id: Identifier::from(id_bytes),
            public_keys: BTreeMap::new(),
            balance: 1000,
            revision: 1,
        };
        super::super::ManagedIdentity::new(dpp::identity::Identity::V0(identity_v0))
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
        let removed = managed.remove_sent_contact_request(&recipient_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().recipient_id, recipient_id);
        assert_eq!(managed.sent_contact_requests.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_sent_request() {
        let mut managed = create_test_identity([1u8; 32]);
        let nonexistent_id = Identifier::from([99u8; 32]);

        let removed = managed.remove_sent_contact_request(&nonexistent_id);
        assert!(removed.is_none());
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
        let removed = managed.remove_incoming_contact_request(&sender_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().sender_id, sender_id);
        assert_eq!(managed.incoming_contact_requests.len(), 0);
    }

    #[test]
    fn test_remove_nonexistent_incoming_request() {
        let mut managed = create_test_identity([1u8; 32]);
        let nonexistent_id = Identifier::from([99u8; 32]);

        let removed = managed.remove_incoming_contact_request(&nonexistent_id);
        assert!(removed.is_none());
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
        let result = managed.accept_incoming_request(&contact_id);
        assert!(result.is_some());

        let contact = result.unwrap();
        assert_eq!(contact.contact_identity_id, contact_id);

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
        let result = managed.accept_incoming_request(&contact_id);
        assert!(result.is_none());
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
        let result = managed.accept_incoming_request(&contact_id);
        assert!(result.is_none());
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
