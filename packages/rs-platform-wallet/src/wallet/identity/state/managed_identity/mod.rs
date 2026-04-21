//! Managed identity that combines a Platform Identity with wallet-specific metadata
//!
//! This module provides the `ManagedIdentity` struct which wraps a Platform Identity
//! with additional metadata for wallet management.

mod contact_requests;
mod contacts;
mod identity_ops;
mod label;
mod sync;

// `block_time` + `key_storage` moved to `crate::wallet::identity::types`.
// Re-export so every `impl ManagedIdentity` block below keeps working
// unchanged and external users can still reach them through the old
// `state::managed_identity::*` path.
pub use crate::wallet::identity::types::block_time::{self, BlockTime};
pub use crate::wallet::identity::types::key_storage::{
    self, DpnsNameInfo, IdentityStatus, KeyStorage, PrivateKeyData, WatchedIdentity,
};

use crate::wallet::identity::{ContactRequest, DashPayProfile, EstablishedContact, PaymentEntry};
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// A managed identity that combines an Identity with wallet-specific metadata
#[derive(Debug, Clone)]
pub struct ManagedIdentity {
    /// The Platform identity
    pub identity: Identity,

    /// The BIP-9 HD identity index used during registration or discovery.
    ///
    /// This is the index in the derivation path `m/9'/coin'/5'/0'/key_type'/identity_index'/key_id'`.
    /// Recorded during identity registration or gap-limit discovery so that
    /// subsequent operations (signing, ECDH) can derive the correct keys.
    pub identity_index: u32,

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

    /// Private key storage mapping KeyID to (public key, private key data).
    pub key_storage: KeyStorage,

    /// Identity lifecycle status on Platform.
    pub status: IdentityStatus,

    /// DPNS usernames associated with this identity.
    pub dpns_names: Vec<DpnsNameInfo>,

    /// DPNS labels this identity is currently contending for —
    /// names that are still in the contested-voting period and
    /// haven't been resolved (won or locked) yet.
    ///
    /// Caches only the label list. Contest metadata (contenders,
    /// votes, end time) changes throughout the voting period, so
    /// caching it would go stale quickly — the UI queries it fresh
    /// via `Sdk::get_non_resolved_dpns_contests_for_identity` when
    /// it needs the details. Resolved contests (won or locked)
    /// should migrate off this list and onto `dpns_names` on
    /// success.
    pub contested_dpns_names: Vec<String>,

    /// Wallet identifier (`SHA256(root_pub_key || chain_code)`) of
    /// the wallet that owns this identity, if known. Set during
    /// gap-limit scan and identity recovery.
    pub wallet_id: Option<[u8; 32]>,

    /// DashPay profile (display name, bio, avatar, public message)
    /// published via the DashPay data contract. `None` until the
    /// profile has been fetched or set.
    pub dashpay_profile: Option<DashPayProfile>,

    /// DashPay payment history keyed by transaction id (hex string).
    /// Each entry records a single Dash payment to or from a contact
    /// identity, with direction, amount, memo, and status.
    pub dashpay_payments: BTreeMap<String, PaymentEntry>,
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
        let managed = ManagedIdentity::new(identity, 0);

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
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        managed.set_label("Test Identity".to_string(), &p);
        assert_eq!(managed.label, Some("Test Identity".to_string()));

        managed.clear_label(&p);
        assert_eq!(managed.label, None);
    }

    #[test]
    fn test_balance_block_time() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let block_time = BlockTime::new(100000, 900000, 1234567890);
        managed.update_balance_block_time(block_time, &p);

        assert_eq!(managed.last_updated_balance_block_time, Some(block_time));
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().height,
            100000
        );
        assert_eq!(
            managed.last_updated_balance_block_time.unwrap().core_height,
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
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let block_time = BlockTime::new(50000, 450000, 9876543210);
        managed.update_keys_sync_block_time(block_time, &p);

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
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        // Never updated - needs update
        assert_eq!(managed.needs_balance_update(1000, 100), true);

        // Just updated
        let block_time = BlockTime::new(100, 900, 1000);
        managed.update_balance_block_time(block_time, &p);
        assert_eq!(managed.needs_balance_update(1050, 100), false);

        // Old update - needs update
        assert_eq!(managed.needs_balance_update(1200, 100), true);
    }

    #[test]
    fn test_needs_keys_sync() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        // Never synced - needs sync
        assert_eq!(managed.needs_keys_sync(1000, 100), true);

        // Just synced
        let block_time = BlockTime::new(100, 900, 1000);
        managed.update_keys_sync_block_time(block_time, &p);
        assert_eq!(managed.needs_keys_sync(1050, 100), false);

        // Old sync - needs sync
        assert_eq!(managed.needs_keys_sync(1200, 100), true);
    }

    #[test]
    fn test_auto_establish_on_sent_request() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // First, add an incoming request from the contact
        let incoming_request = ContactRequest::new(
            contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_incoming_contact_request(incoming_request, &p);

        // Verify it's in incoming requests
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Now add a sent request to the same contact - should auto-establish
        let outgoing_request = ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_sent_contact_request(outgoing_request, &p);

        // Verify contact was established
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_auto_establish_on_incoming_request() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // First, add a sent request to the contact
        let outgoing_request = ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_sent_contact_request(outgoing_request, &p);

        // Verify it's in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Now add an incoming request from the same contact - should auto-establish
        let incoming_request = ContactRequest::new(
            contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_incoming_contact_request(incoming_request, &p);

        // Verify contact was established
        assert_eq!(managed.incoming_contact_requests.len(), 0);
        assert_eq!(managed.sent_contact_requests.len(), 0);
        assert_eq!(managed.established_contacts.len(), 1);
        assert!(managed.established_contacts.contains_key(&contact_id));
    }

    #[test]
    fn test_no_auto_establish_without_reciprocal() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity, 0);
        let p = noop_persister();

        let contact_id = Identifier::from([2u8; 32]);
        let our_id = Identifier::from([1u8; 32]);

        // Add a sent request without a reciprocal incoming request
        let outgoing_request = ContactRequest::new(
            our_id,
            contact_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567890,
        );
        managed.add_sent_contact_request(outgoing_request, &p);

        // Verify it stays in sent requests
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);

        // Add an incoming request from a different contact
        let other_contact_id = Identifier::from([3u8; 32]);
        let incoming_request = ContactRequest::new(
            other_contact_id,
            our_id,
            0,
            0,
            0,
            vec![0u8; 96],
            100000,
            1234567891,
        );
        managed.add_incoming_contact_request(incoming_request, &p);

        // Verify both requests stay separate
        assert_eq!(managed.sent_contact_requests.len(), 1);
        assert_eq!(managed.incoming_contact_requests.len(), 1);
        assert_eq!(managed.established_contacts.len(), 0);
    }
}
