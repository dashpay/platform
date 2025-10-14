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
        self.established_contacts.insert(*sender_id, contact.clone());

        Some(contact)
    }
}
