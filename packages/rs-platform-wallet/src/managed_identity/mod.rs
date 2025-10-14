//! Managed identity that combines a Platform Identity with wallet-specific metadata
//!
//! This module provides the `ManagedIdentity` struct which wraps a Platform Identity
//! with additional metadata for wallet management.

use crate::{BlockTime, ContactRequest, EstablishedContact};
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

// Import implementation modules
mod identity_ops;
mod label;
mod sync;
mod contacts;
mod contact_requests;

/// A managed identity that combines an Identity with wallet-specific metadata
#[derive(Debug, Clone)]
pub struct ManagedIdentity {
    /// The Platform identity
    pub identity: Identity,

    /// Last block time when balance was updated for this identity
    pub last_updated_balance_block_time: Option<BlockTime>,

    /// Last block time when keys were synced for this identity
    pub last_synced_keys_block_time: Option<BlockTime>,

    /// User-defined label for this identity
    pub label: Option<String>,

    /// Map of established contacts (bidirectional relationships) keyed by contact identity ID
    pub established_contacts: BTreeMap<Identifier, EstablishedContact>,

    /// Map of sent contact requests (outgoing, not yet reciprocated) keyed by recipient ID
    pub sent_contact_requests: BTreeMap<Identifier, ContactRequest>,

    /// Map of incoming contact requests (not yet accepted) keyed by sender ID
    pub incoming_contact_requests: BTreeMap<Identifier, ContactRequest>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    fn create_test_identity() -> Identity {
        let identity_v0 = IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 1000,
            revision: 1,
        };
        Identity::V0(identity_v0)
    }

    #[test]
    fn test_managed_identity_creation() {
        let identity = create_test_identity();
        let managed = ManagedIdentity::new(identity);

        assert_eq!(managed.id(), Identifier::from([1u8; 32]));
        assert_eq!(managed.balance(), 1000);
        assert_eq!(managed.revision(), 1);
        assert_eq!(managed.label, None);
        assert_eq!(managed.last_updated_balance_block_time, None);
        assert_eq!(managed.last_synced_keys_block_time, None);
    }

    #[test]
    fn test_label_management() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        managed.set_label("Test Identity".to_string());
        assert_eq!(managed.label, Some("Test Identity".to_string()));

        managed.clear_label();
        assert_eq!(managed.label, None);
    }

    #[test]
    fn test_balance_block_time() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        let block_time = super::super::BlockTime::new(100000, 900000, 1234567890);
        managed.update_balance_block_time(block_time);

        assert_eq!(managed.last_updated_balance_block_time, Some(block_time));
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().height,
            100000
        );
        assert_eq!(
            managed
                .last_updated_balance_block_time
                .unwrap()
                .core_height,
            900000
        );
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().timestamp,
            1234567890
        );
    }

    #[test]
    fn test_keys_sync_block_time() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        let block_time = super::super::BlockTime::new(50000, 450000, 9876543210);
        managed.update_keys_sync_block_time(block_time);

        assert_eq!(managed.last_synced_keys_block_time, Some(block_time));
        assert_eq!(managed.last_synced_keys_block_time.unwrap().height, 50000);
        assert_eq!(
            managed.last_synced_keys_block_time.unwrap().core_height,
            450000
        );
        assert_eq!(
            managed.last_synced_keys_block_time.unwrap().timestamp,
            9876543210
        );
    }

    #[test]
    fn test_needs_balance_update() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        // Never updated - needs update
        assert_eq!(managed.needs_balance_update(1000, 100), true);

        // Just updated
        let block_time = super::super::BlockTime::new(100, 900, 1000);
        managed.update_balance_block_time(block_time);
        assert_eq!(managed.needs_balance_update(1050, 100), false);

        // Old update - needs update
        assert_eq!(managed.needs_balance_update(1200, 100), true);
    }

    #[test]
    fn test_needs_keys_sync() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        // Never synced - needs sync
        assert_eq!(managed.needs_keys_sync(1000, 100), true);

        // Just synced
        let block_time = super::super::BlockTime::new(100, 900, 1000);
        managed.update_keys_sync_block_time(block_time);
        assert_eq!(managed.needs_keys_sync(1050, 100), false);

        // Old sync - needs sync
        assert_eq!(managed.needs_keys_sync(1200, 100), true);
    }

    #[test]
    fn test_auto_establish_on_sent_request() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // First, add an incoming request from the contact
        let incoming_request = super::super::ContactRequest::new(
            contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_incoming_contact_request(incoming_request);

        // Verify it's in incoming requests
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Now add a sent request to the same contact - should auto-establish
        let outgoing_request = super::super::ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_sent_contact_request(outgoing_request);

        // Verify contact was established
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_auto_establish_on_incoming_request() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // First, add a sent request to the contact
        let outgoing_request = super::super::ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_sent_contact_request(outgoing_request);

        // Verify it's in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Now add an incoming request from the same contact - should auto-establish
        let incoming_request = super::super::ContactRequest::new(
            contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_incoming_contact_request(incoming_request);

        // Verify contact was established
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_no_auto_establish_without_reciprocal() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // Add a sent request without a reciprocal incoming request
        let outgoing_request = super::super::ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_sent_contact_request(outgoing_request);

        // Verify it stays in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add an incoming request from a different contact
        let other_contact_id = Identifier::from([3u8; 32]);
        let incoming_request = super::super::ContactRequest::new(
            other_contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_incoming_contact_request(incoming_request);

        // Verify both requests stay separate
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);
    }
}
