//! Established contact between identities in DashPay
//!
//! This module provides the `EstablishedContact` struct representing a bidirectional
//! relationship (friendship) between two identities where both have sent contact requests.

use crate::ContactRequest;
use dpp::prelude::Identifier;

/// An established contact represents a bidirectional relationship between two identities
///
/// This is formed when both identities have sent contact requests to each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstablishedContact {
    /// The contact's identity unique identifier
    pub contact_identity_id: Identifier,

    /// The outgoing contact request (from us to them)
    pub outgoing_request: ContactRequest,

    /// The incoming contact request (from them to us)
    pub incoming_request: ContactRequest,

    /// Optional alias/nickname for this contact
    pub alias: Option<String>,

    /// Optional note about this contact
    pub note: Option<String>,

    /// Whether this contact is hidden from the contact list
    pub is_hidden: bool,

    /// List of accepted account references beyond the default
    pub accepted_accounts: Vec<u32>,
}

impl EstablishedContact {
    /// Create a new established contact from bidirectional contact requests
    pub fn new(
        contact_identity_id: Identifier,
        outgoing_request: ContactRequest,
        incoming_request: ContactRequest,
    ) -> Self {
        Self {
            contact_identity_id,
            outgoing_request,
            incoming_request,
            alias: None,
            note: None,
            is_hidden: false,
            accepted_accounts: Vec::new(),
        }
    }

    /// Set the alias for this contact
    pub fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }

    /// Clear the alias for this contact
    pub fn clear_alias(&mut self) {
        self.alias = None;
    }

    /// Set a note for this contact
    pub fn set_note(&mut self, note: String) {
        self.note = Some(note);
    }

    /// Clear the note for this contact
    pub fn clear_note(&mut self) {
        self.note = None;
    }

    /// Hide this contact from the contact list
    pub fn hide(&mut self) {
        self.is_hidden = true;
    }

    /// Unhide this contact
    pub fn unhide(&mut self) {
        self.is_hidden = false;
    }

    /// Add an accepted account reference
    pub fn add_accepted_account(&mut self, account_reference: u32) {
        if !self.accepted_accounts.contains(&account_reference) {
            self.accepted_accounts.push(account_reference);
        }
    }

    /// Remove an accepted account reference
    pub fn remove_accepted_account(&mut self, account_reference: u32) {
        self.accepted_accounts.retain(|&a| a != account_reference);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_outgoing_request() -> ContactRequest {
        ContactRequest::new(
            Identifier::from([1u8; 32]), // sender (us)
            Identifier::from([2u8; 32]), // recipient (them)
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        )
    }

    fn create_test_incoming_request() -> ContactRequest {
        ContactRequest::new(
            Identifier::from([2u8; 32]), // sender (them)
            Identifier::from([1u8; 32]), // recipient (us)
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        )
    }

    #[test]
    fn test_established_contact_creation() {
        let contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        assert_eq!(contact.contact_identity_id, Identifier::from([2u8; 32]));
        assert_eq!(contact.alias, None);
        assert_eq!(contact.note, None);
        assert_eq!(contact.is_hidden, false);
        assert_eq!(contact.accepted_accounts.len(), 0);
    }

    #[test]
    fn test_alias_management() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        contact.set_alias("Best Friend".to_string());
        assert_eq!(contact.alias, Some("Best Friend".to_string()));

        contact.clear_alias();
        assert_eq!(contact.alias, None);
    }

    #[test]
    fn test_note_management() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        contact.set_note("Met at conference".to_string());
        assert_eq!(contact.note, Some("Met at conference".to_string()));

        contact.clear_note();
        assert_eq!(contact.note, None);
    }

    #[test]
    fn test_hide_unhide() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        assert_eq!(contact.is_hidden, false);

        contact.hide();
        assert_eq!(contact.is_hidden, true);

        contact.unhide();
        assert_eq!(contact.is_hidden, false);
    }

    #[test]
    fn test_accepted_accounts() {
        let mut contact = EstablishedContact::new(
            Identifier::from([2u8; 32]),
            create_test_outgoing_request(),
            create_test_incoming_request(),
        );

        // Add accounts
        contact.add_accepted_account(1);
        contact.add_accepted_account(2);
        assert_eq!(contact.accepted_accounts.len(), 2);
        assert!(contact.accepted_accounts.contains(&1));
        assert!(contact.accepted_accounts.contains(&2));

        // Adding duplicate should not increase count
        contact.add_accepted_account(1);
        assert_eq!(contact.accepted_accounts.len(), 2);

        // Remove account
        contact.remove_accepted_account(1);
        assert_eq!(contact.accepted_accounts.len(), 1);
        assert!(!contact.accepted_accounts.contains(&1));
        assert!(contact.accepted_accounts.contains(&2));
    }
}
