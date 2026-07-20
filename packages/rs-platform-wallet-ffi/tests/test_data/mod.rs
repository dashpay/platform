//! Test data module for platform-wallet-ffi tests
//!
//! This module provides realistic fake data for testing contact requests,
//! identities, and other platform wallet operations.

#![allow(dead_code)]

use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use platform_wallet::{ContactRequest, EstablishedContact, ManagedIdentity};
use std::collections::BTreeMap;

/// Create a test identity with a given ID and balance
pub fn create_test_identity(id_bytes: [u8; 32], balance: u64) -> Identity {
    use dpp::identity::v0::IdentityV0;

    let id = Identifier::from(id_bytes);

    // Create some public keys for the identity
    let mut public_keys = BTreeMap::new();

    // Master key (key ID 0)
    public_keys.insert(
        0,
        IdentityPublicKey::V0(
            dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
                id: 0,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                read_only: false,
                data: dpp::platform_value::BinaryData::new(vec![2u8; 33]),
                disabled_at: None,
                contract_bounds: None,
            },
        ),
    );

    // High security key (key ID 1)
    public_keys.insert(
        1,
        IdentityPublicKey::V0(
            dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
                id: 1,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                read_only: false,
                data: dpp::platform_value::BinaryData::new(vec![3u8; 33]),
                disabled_at: None,
                contract_bounds: None,
            },
        ),
    );

    // Encryption key (key ID 2)
    public_keys.insert(
        2,
        IdentityPublicKey::V0(
            dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
                id: 2,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::ENCRYPTION,
                security_level: SecurityLevel::MEDIUM,
                read_only: false,
                data: dpp::platform_value::BinaryData::new(vec![4u8; 33]),
                disabled_at: None,
                contract_bounds: None,
            },
        ),
    );

    let identity_v0 = IdentityV0 {
        id,
        public_keys,
        balance,
        revision: 1,
    };

    Identity::V0(identity_v0)
}

/// Create a managed identity. The `label` argument is retained for
/// signature compatibility with the existing fixture call sites but is
/// dropped — `ManagedIdentity` no longer carries a label field. Tests
/// that need to assert on a user-facing label should look at
/// `PersistentIdentity.alias` (Swift side) instead.
pub fn create_managed_identity(id_bytes: [u8; 32], balance: u64, _label: &str) -> ManagedIdentity {
    let identity = create_test_identity(id_bytes, balance);
    ManagedIdentity::new(identity, 0)
}

/// Create a contact request from sender to recipient
pub fn create_contact_request(
    sender_id: Identifier,
    recipient_id: Identifier,
    sender_key_index: u32,
    recipient_key_index: u32,
    account_reference: u32,
    timestamp: u64,
) -> ContactRequest {
    // Create realistic encrypted public key (96 bytes)
    let mut encrypted_public_key = Vec::with_capacity(96);
    // Simulate encrypted data with some pattern
    for i in 0..96 {
        let val = sender_id.as_bytes()[i % 32].wrapping_add(recipient_id.as_bytes()[i % 32]);
        encrypted_public_key.push(val);
    }

    ContactRequest::new(
        sender_id,
        recipient_id,
        sender_key_index,
        recipient_key_index,
        account_reference,
        encrypted_public_key,
        100000 + (timestamp / 1000) as u32, // Core block height derived from timestamp
        timestamp,
    )
}

/// Create an established contact between two identities
pub fn create_established_contact(
    contact_id: Identifier,
    our_id: Identifier,
    timestamp_outgoing: u64,
    timestamp_incoming: u64,
) -> EstablishedContact {
    let outgoing_request = create_contact_request(
        our_id,
        contact_id,
        0, // sender_key_index
        1, // recipient_key_index
        0, // account_reference
        timestamp_outgoing,
    );

    let incoming_request = create_contact_request(
        contact_id,
        our_id,
        1, // sender_key_index
        0, // recipient_key_index
        0, // account_reference
        timestamp_incoming,
    );

    EstablishedContact::new(contact_id, outgoing_request, incoming_request)
}

/// Predefined test identities
pub mod identities {
    use super::*;

    /// Alice's identity (primary test identity)
    pub fn alice() -> ManagedIdentity {
        create_managed_identity([1u8; 32], 10_000_000, "Alice")
    }

    /// Bob's identity
    pub fn bob() -> ManagedIdentity {
        create_managed_identity([2u8; 32], 5_000_000, "Bob")
    }

    /// Carol's identity
    pub fn carol() -> ManagedIdentity {
        create_managed_identity([3u8; 32], 8_000_000, "Carol")
    }

    /// Dave's identity
    pub fn dave() -> ManagedIdentity {
        create_managed_identity([4u8; 32], 3_000_000, "Dave")
    }

    /// Eve's identity (potential adversary in tests)
    pub fn eve() -> ManagedIdentity {
        create_managed_identity([5u8; 32], 1_000_000, "Eve")
    }
}

/// Predefined contact request scenarios
pub mod scenarios {
    use super::*;
    use dpp::identity::accessors::IdentityGettersV0;
    use platform_wallet::wallet::persister::NoPlatformPersistence;
    use platform_wallet::WalletPersister;
    use std::sync::Arc;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

    /// Alice sends contact request to Bob
    pub fn alice_to_bob_contact_request() -> ContactRequest {
        let alice_id = identities::alice().identity.id();
        let bob_id = identities::bob().identity.id();
        create_contact_request(alice_id, bob_id, 0, 1, 0, 1_700_000_000)
    }

    /// Bob sends contact request to Alice
    pub fn bob_to_alice_contact_request() -> ContactRequest {
        let alice_id = identities::alice().identity.id();
        let bob_id = identities::bob().identity.id();
        create_contact_request(bob_id, alice_id, 1, 0, 0, 1_700_000_100)
    }

    /// Carol sends contact request to Alice
    pub fn carol_to_alice_contact_request() -> ContactRequest {
        let alice_id = identities::alice().identity.id();
        let carol_id = identities::carol().identity.id();
        create_contact_request(carol_id, alice_id, 0, 0, 0, 1_700_000_200)
    }

    /// Alice and Bob have established contact
    pub fn alice_bob_established_contact() -> EstablishedContact {
        let alice_id = identities::alice().identity.id();
        let bob_id = identities::bob().identity.id();
        create_established_contact(bob_id, alice_id, 1_700_000_000, 1_700_000_100)
    }

    /// Alice has multiple pending sent requests
    pub fn alice_with_pending_sent_requests() -> (ManagedIdentity, Vec<ContactRequest>) {
        let mut alice = identities::alice();
        let bob_id = identities::bob().identity.id();
        let carol_id = identities::carol().identity.id();
        let dave_id = identities::dave().identity.id();

        let alice_id = alice.identity.id();

        let req1 = create_contact_request(alice_id, bob_id, 0, 1, 0, 1_700_000_000);
        let req2 = create_contact_request(alice_id, carol_id, 0, 1, 1, 1_700_000_050);
        let req3 = create_contact_request(alice_id, dave_id, 0, 1, 2, 1_700_000_100);

        alice
            .add_sent_contact_request(req1.clone(), &noop_persister())
            .expect("test setup persists");
        alice
            .add_sent_contact_request(req2.clone(), &noop_persister())
            .expect("test setup persists");
        alice
            .add_sent_contact_request(req3.clone(), &noop_persister())
            .expect("test setup persists");

        (alice, vec![req1, req2, req3])
    }

    /// Alice has multiple pending incoming requests
    pub fn alice_with_pending_incoming_requests() -> (ManagedIdentity, Vec<ContactRequest>) {
        let mut alice = identities::alice();
        let bob_id = identities::bob().identity.id();
        let carol_id = identities::carol().identity.id();
        let dave_id = identities::dave().identity.id();

        let alice_id = alice.identity.id();

        let req1 = create_contact_request(bob_id, alice_id, 1, 0, 0, 1_700_000_000);
        let req2 = create_contact_request(carol_id, alice_id, 1, 0, 0, 1_700_000_050);
        let req3 = create_contact_request(dave_id, alice_id, 1, 0, 0, 1_700_000_100);

        alice
            .add_incoming_contact_request(req1.clone(), &noop_persister())
            .expect("test setup persists");
        alice
            .add_incoming_contact_request(req2.clone(), &noop_persister())
            .expect("test setup persists");
        alice
            .add_incoming_contact_request(req3.clone(), &noop_persister())
            .expect("test setup persists");

        (alice, vec![req1, req2, req3])
    }

    /// Alice has established contacts with multiple people
    pub fn alice_with_established_contacts() -> (ManagedIdentity, Vec<EstablishedContact>) {
        let mut alice = identities::alice();
        let bob_id = identities::bob().identity.id();
        let carol_id = identities::carol().identity.id();

        let alice_id = alice.identity.id();

        let contact1 = create_established_contact(bob_id, alice_id, 1_700_000_000, 1_700_000_100);
        let contact2 = create_established_contact(carol_id, alice_id, 1_700_000_200, 1_700_000_300);

        alice.apply_established_contact(contact1.clone());
        alice.apply_established_contact(contact2.clone());

        (alice, vec![contact1, contact2])
    }

    /// Complex scenario: Alice has all types of contacts
    pub fn alice_with_mixed_contacts() -> ManagedIdentity {
        let mut alice = identities::alice();
        let bob_id = identities::bob().identity.id();
        let carol_id = identities::carol().identity.id();
        let dave_id = identities::dave().identity.id();
        let eve_id = identities::eve().identity.id();

        let alice_id = alice.identity.id();

        // Established contact with Bob
        let bob_contact =
            create_established_contact(bob_id, alice_id, 1_700_000_000, 1_700_000_100);
        alice.apply_established_contact(bob_contact);

        // Pending sent request to Carol (not reciprocated yet)
        let carol_request = create_contact_request(alice_id, carol_id, 0, 1, 0, 1_700_000_200);
        alice
            .add_sent_contact_request(carol_request, &noop_persister())
            .expect("test setup persists");

        // Pending incoming request from Dave (we haven't sent back yet)
        let dave_request = create_contact_request(dave_id, alice_id, 1, 0, 0, 1_700_000_300);
        alice
            .add_incoming_contact_request(dave_request, &noop_persister())
            .expect("test setup persists");

        // Pending incoming request from Eve
        let eve_request = create_contact_request(eve_id, alice_id, 1, 0, 0, 1_700_000_400);
        alice
            .add_incoming_contact_request(eve_request, &noop_persister())
            .expect("test setup persists");

        alice
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::accessors::IdentityGettersV0;

    #[test]
    fn test_create_test_identity() {
        let identity = create_test_identity([1u8; 32], 1_000_000);
        assert_eq!(identity.id(), Identifier::from([1u8; 32]));
        assert_eq!(identity.balance(), 1_000_000);
        assert_eq!(identity.revision(), 1);
    }

    #[test]
    fn test_create_managed_identity() {
        let managed = create_managed_identity([2u8; 32], 500_000, "Test User");
        assert_eq!(managed.identity.id(), Identifier::from([2u8; 32]));
        assert_eq!(managed.identity.balance(), 500_000);
    }

    #[test]
    fn test_create_contact_request() {
        let sender_id = Identifier::from([1u8; 32]);
        let recipient_id = Identifier::from([2u8; 32]);

        let request = create_contact_request(sender_id, recipient_id, 0, 1, 0, 1_700_000_000);

        assert_eq!(request.sender_id, sender_id);
        assert_eq!(request.recipient_id, recipient_id);
        assert_eq!(request.sender_key_index, 0);
        assert_eq!(request.recipient_key_index, 1);
        assert_eq!(request.account_reference, 0);
        assert_eq!(request.created_at, 1_700_000_000);
        assert_eq!(request.encrypted_public_key.len(), 96);
    }

    #[test]
    fn test_identities() {
        let alice = identities::alice();
        let bob = identities::bob();
        let carol = identities::carol();

        // `ManagedIdentity.label` is gone — labels are a UI concern
        // (Swift `PersistentIdentity.alias`). Just exercise the
        // non-label fixture invariants.
        assert_eq!(alice.identity.balance(), 10_000_000);
        assert_eq!(bob.identity.balance(), 5_000_000);
        assert_eq!(carol.identity.balance(), 8_000_000);
    }

    #[test]
    fn test_alice_with_pending_sent_requests() {
        let (alice, requests) = scenarios::alice_with_pending_sent_requests();

        assert_eq!(alice.dashpay().sent_contact_requests().len(), 3);
        assert_eq!(requests.len(), 3);

        // Verify requests are in the managed identity
        for request in &requests {
            assert!(alice
                .dashpay()
                .sent_contact_requests()
                .contains_key(&request.recipient_id));
        }
    }

    #[test]
    fn test_alice_with_mixed_contacts() {
        let alice = scenarios::alice_with_mixed_contacts();

        assert_eq!(alice.dashpay().established_contacts().len(), 1); // Bob
        assert_eq!(alice.dashpay().sent_contact_requests().len(), 1); // Carol
        assert_eq!(alice.dashpay().incoming_contact_requests().len(), 2); // Dave, Eve
    }
}
