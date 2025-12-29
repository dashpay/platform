//! Contact request between identities in DashPay
//!
//! This module provides the `ContactRequest` struct representing a one-way relationship
//! between a sender identity and a recipient identity.

use dpp::identity::TimestampMillis;
use dpp::prelude::{CoreBlockHeight, Identifier};

/// A contact request represents a one-way relationship between two identities
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactRequest {
    /// The unique id of the sender (owner of the contact request)
    pub sender_id: Identifier,

    /// The unique id of the recipient
    pub recipient_id: Identifier,

    /// The index of the sender's identity public key used for ECDH
    pub sender_key_index: u32,

    /// The index of the recipient's identity public key used for ECDH
    pub recipient_key_index: u32,

    /// Account reference (encrypted for the sender)
    pub account_reference: u32,

    /// Encrypted account label (optional)
    pub encrypted_account_label: Option<Vec<u8>>,

    /// Encrypted extended public key for receiving payments
    pub encrypted_public_key: Vec<u8>,

    /// Auto accept proof (optional)
    pub auto_accept_proof: Option<Vec<u8>>,

    /// Core height when the contact request was created
    pub core_height_created_at: CoreBlockHeight,

    /// Timestamp when the contact request was created (milliseconds)
    pub created_at: TimestampMillis,
}

impl ContactRequest {
    /// Create a new contact request
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sender_id: Identifier,
        recipient_id: Identifier,
        sender_key_index: u32,
        recipient_key_index: u32,
        account_reference: u32,
        encrypted_public_key: Vec<u8>,
        core_height_created_at: CoreBlockHeight,
        created_at: TimestampMillis,
    ) -> Self {
        Self {
            sender_id,
            recipient_id,
            sender_key_index,
            recipient_key_index,
            account_reference,
            encrypted_account_label: None,
            encrypted_public_key,
            auto_accept_proof: None,
            core_height_created_at,
            created_at,
        }
    }

    /// Check if this is an outgoing request for the given identity
    pub fn is_outgoing(&self, identity_id: &Identifier) -> bool {
        &self.sender_id == identity_id
    }

    /// Check if this is an incoming request for the given identity
    pub fn is_incoming(&self, identity_id: &Identifier) -> bool {
        &self.recipient_id == identity_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_contact_request() -> ContactRequest {
        ContactRequest::new(
            Identifier::from([1u8; 32]),
            Identifier::from([2u8; 32]),
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        )
    }

    #[test]
    fn test_contact_request_creation() {
        let request = create_test_contact_request();

        assert_eq!(request.sender_id, Identifier::from([1u8; 32]));
        assert_eq!(request.recipient_id, Identifier::from([2u8; 32]));
        assert_eq!(request.sender_key_index, 0);
        assert_eq!(request.recipient_key_index, 0);
        assert_eq!(request.account_reference, 0);
        assert_eq!(request.encrypted_public_key.len(), 96);
        assert_eq!(request.core_height_created_at, 100000);
        assert_eq!(request.created_at, 1234567890);
    }

    #[test]
    fn test_is_outgoing() {
        let request = create_test_contact_request();
        let sender_id = Identifier::from([1u8; 32]);
        let other_id = Identifier::from([3u8; 32]);

        assert!(request.is_outgoing(&sender_id));
        assert!(!request.is_outgoing(&other_id));
    }

    #[test]
    fn test_is_incoming() {
        let request = create_test_contact_request();
        let recipient_id = Identifier::from([2u8; 32]);
        let other_id = Identifier::from([3u8; 32]);

        assert!(request.is_incoming(&recipient_id));
        assert!(!request.is_incoming(&other_id));
    }
}
