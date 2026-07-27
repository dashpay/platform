//! Integration tests for contact request workflows
//!
//! These tests cover the complete workflow from sending contact requests to establishing contacts,
//! similar to the DashSync E2E tests but at a unit/integration level.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::identity_public_key::{IdentityPublicKey, Purpose};
use dpp::identity::v0::IdentityV0;
use dpp::identity::{Identity, KeyType, SecurityLevel};
use dpp::prelude::Identifier;
use platform_wallet::wallet::persister::{NoPlatformPersistence, WalletPersister};
use platform_wallet::{ContactRequest, ManagedIdentity};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Build a no-op `WalletPersister` for tests — mutation methods on
/// `ManagedIdentity` require one but none of these tests exercise the
/// persistence side, so a backend that drops every changeset on the
/// floor is sufficient.
fn noop_persister() -> WalletPersister {
    WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
}

/// Helper function to create a test identity with encryption key
fn create_test_identity(id_bytes: [u8; 32]) -> Identity {
    let mut public_keys = BTreeMap::new();

    // Add encryption key at index 0
    let encryption_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: 0,
        purpose: Purpose::ENCRYPTION,
        security_level: SecurityLevel::MEDIUM,
        contract_bounds: None,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: dpp::platform_value::BinaryData::new(vec![1u8; 33]),
        disabled_at: None,
    });

    public_keys.insert(0, encryption_key);

    let identity_v0 = IdentityV0 {
        id: Identifier::from(id_bytes),
        public_keys,
        balance: 1_000_000, // 0.01 Dash in duffs
        revision: 1,
    };
    Identity::V0(identity_v0)
}

/// Helper function to create a contact request
fn create_contact_request(
    sender_id: Identifier,
    recipient_id: Identifier,
    account_reference: u32,
    timestamp: u64,
) -> ContactRequest {
    ContactRequest::new(
        sender_id,
        recipient_id,
        0, // sender_key_index
        0, // recipient_key_index
        account_reference,
        vec![0u8; 96], // encrypted_public_key
        100000,        // core_height_created_at
        timestamp,
    )
}

#[test]
fn test_send_and_accept_contact_request_same_wallet() {
    // Simulate testGSendAndAcceptContactRequestSameWallet from DashSync
    // This tests sending friend requests between two identities within the same wallet

    // Create two identities (like identityA and identityB in DashSync)
    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);

    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);
    let mut managed_b = ManagedIdentity::new(identity_b, 1);

    // Identity A sends friend request to Identity B
    let request_a_to_b = create_contact_request(id_a, id_b, 0, 1234567890);
    managed_a
        .add_sent_contact_request(request_a_to_b.clone(), &noop_persister())
        .expect("setup persists");

    // Verify request is pending
    assert_eq!(managed_a.dashpay().sent_contact_requests().len(), 1);
    assert_eq!(managed_a.dashpay().established_contacts().len(), 0);

    // Identity B receives the request
    managed_b
        .add_incoming_contact_request(request_a_to_b, &noop_persister())
        .expect("setup persists");

    // Verify B has incoming request
    assert_eq!(managed_b.dashpay().incoming_contact_requests().len(), 1);
    assert_eq!(managed_b.dashpay().established_contacts().len(), 0);

    // Identity B sends friend request back to Identity A
    let request_b_to_a = create_contact_request(id_b, id_a, 0, 1234567891);
    managed_b
        .add_sent_contact_request(request_b_to_a.clone(), &noop_persister())
        .expect("setup persists");

    // This should auto-establish on B's side
    assert_eq!(managed_b.dashpay().sent_contact_requests().len(), 0);
    assert_eq!(managed_b.dashpay().incoming_contact_requests().len(), 0);
    assert_eq!(managed_b.dashpay().established_contacts().len(), 1);
    assert!(managed_b
        .dashpay()
        .established_contacts()
        .contains_key(&id_a));

    // Identity A receives B's request
    managed_a
        .add_incoming_contact_request(request_b_to_a, &noop_persister())
        .expect("setup persists");

    // This should auto-establish on A's side
    assert_eq!(managed_a.dashpay().sent_contact_requests().len(), 0);
    assert_eq!(managed_a.dashpay().incoming_contact_requests().len(), 0);
    assert_eq!(managed_a.dashpay().established_contacts().len(), 1);
    assert!(managed_a
        .dashpay()
        .established_contacts()
        .contains_key(&id_b));

    // Both should have established contacts now
    let contact_a = managed_a
        .dashpay()
        .established_contacts()
        .get(&id_b)
        .unwrap();
    let contact_b = managed_b
        .dashpay()
        .established_contacts()
        .get(&id_a)
        .unwrap();

    assert_eq!(contact_a.contact_identity_id, id_b);
    assert_eq!(contact_b.contact_identity_id, id_a);
}

#[test]
fn test_send_and_accept_contact_request_different_wallets() {
    // Simulate testHSendAndAcceptContactRequestDifferentWallet from DashSync
    // This tests sending friend requests between identities in different wallets

    let identity_1 = create_test_identity([10u8; 32]);
    let identity_2 = create_test_identity([20u8; 32]);

    let id_1 = identity_1.id();
    let id_2 = identity_2.id();

    let mut managed_1 = ManagedIdentity::new(identity_1, 0);
    let mut managed_2 = ManagedIdentity::new(identity_2, 1);

    // Identity 1 sends friend request to Identity 2
    let request_1_to_2 = create_contact_request(id_1, id_2, 0, 1234567900);
    managed_1
        .add_sent_contact_request(request_1_to_2.clone(), &noop_persister())
        .expect("setup persists");

    // Identity 2 receives the request
    managed_2
        .add_incoming_contact_request(request_1_to_2, &noop_persister())
        .expect("setup persists");

    // Verify states before reciprocation
    assert_eq!(managed_1.dashpay().sent_contact_requests().len(), 1);
    assert_eq!(managed_2.dashpay().incoming_contact_requests().len(), 1);

    // Identity 2 sends friend request back
    let request_2_to_1 = create_contact_request(id_2, id_1, 0, 1234567901);
    managed_2
        .add_sent_contact_request(request_2_to_1.clone(), &noop_persister())
        .expect("setup persists");

    // Should auto-establish on identity 2's side
    assert_eq!(managed_2.dashpay().established_contacts().len(), 1);

    // Identity 1 receives the reciprocal request
    managed_1
        .add_incoming_contact_request(request_2_to_1, &noop_persister())
        .expect("setup persists");

    // Should auto-establish on identity 1's side
    assert_eq!(managed_1.dashpay().established_contacts().len(), 1);

    // Verify both have the friendship established
    assert!(managed_1
        .dashpay()
        .established_contacts()
        .contains_key(&id_2));
    assert!(managed_2
        .dashpay()
        .established_contacts()
        .contains_key(&id_1));
}

#[test]
fn test_multiple_contact_requests_workflow() {
    // Test managing multiple concurrent contact requests
    // Similar to having multiple identities sending requests

    let identity_main = create_test_identity([1u8; 32]);
    let identity_friend1 = create_test_identity([2u8; 32]);
    let identity_friend2 = create_test_identity([3u8; 32]);
    let identity_friend3 = create_test_identity([4u8; 32]);

    let id_main = identity_main.id();
    let id_friend1 = identity_friend1.id();
    let id_friend2 = identity_friend2.id();
    let id_friend3 = identity_friend3.id();

    let mut managed_main = ManagedIdentity::new(identity_main, 0);

    // Send requests to three different identities
    managed_main
        .add_sent_contact_request(
            create_contact_request(id_main, id_friend1, 0, 1000),
            &noop_persister(),
        )
        .expect("setup persists");
    managed_main
        .add_sent_contact_request(
            create_contact_request(id_main, id_friend2, 0, 2000),
            &noop_persister(),
        )
        .expect("setup persists");
    managed_main
        .add_sent_contact_request(
            create_contact_request(id_main, id_friend3, 0, 3000),
            &noop_persister(),
        )
        .expect("setup persists");

    assert_eq!(managed_main.dashpay().sent_contact_requests().len(), 3);

    // Receive incoming request from friend1 (should auto-establish)
    managed_main
        .add_incoming_contact_request(
            create_contact_request(id_friend1, id_main, 0, 1001),
            &noop_persister(),
        )
        .expect("setup persists");

    assert_eq!(managed_main.dashpay().sent_contact_requests().len(), 2); // friend2 and friend3 left
    assert_eq!(managed_main.dashpay().established_contacts().len(), 1); // friend1 established

    // Receive incoming request from friend2 (should auto-establish)
    managed_main
        .add_incoming_contact_request(
            create_contact_request(id_friend2, id_main, 0, 2001),
            &noop_persister(),
        )
        .expect("setup persists");

    assert_eq!(managed_main.dashpay().sent_contact_requests().len(), 1); // only friend3 left
    assert_eq!(managed_main.dashpay().established_contacts().len(), 2); // friend1 and friend2 established

    // Receive incoming from unknown identity (should stay in incoming)
    let id_stranger = Identifier::from([99u8; 32]);
    managed_main
        .add_incoming_contact_request(
            create_contact_request(id_stranger, id_main, 0, 9000),
            &noop_persister(),
        )
        .expect("setup persists");

    assert_eq!(managed_main.dashpay().incoming_contact_requests().len(), 1);
    assert_eq!(managed_main.dashpay().sent_contact_requests().len(), 1);
    assert_eq!(managed_main.dashpay().established_contacts().len(), 2);
}

#[test]
fn test_contact_alias_and_metadata() {
    // Test setting alias, notes, and other metadata on established contacts

    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);

    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);

    // Establish contact
    let request_a_to_b = create_contact_request(id_a, id_b, 0, 1000);
    let request_b_to_a = create_contact_request(id_b, id_a, 0, 1001);

    managed_a
        .add_sent_contact_request(request_a_to_b, &noop_persister())
        .expect("setup persists");
    managed_a
        .add_incoming_contact_request(request_b_to_a, &noop_persister())
        .expect("setup persists");

    // Contact should be established
    assert_eq!(managed_a.dashpay().established_contacts().len(), 1);

    // Get mutable reference to contact and modify metadata
    let contact = managed_a.established_contact_mut(&id_b).unwrap();

    // Set alias
    contact.set_alias("Best Friend".to_string());
    assert_eq!(contact.alias, Some("Best Friend".to_string()));

    // Set note
    contact.set_note("Met at DevCon 2024".to_string());
    assert_eq!(contact.note, Some("Met at DevCon 2024".to_string()));

    // Test hiding/unhiding
    assert!(!contact.is_hidden);
    contact.hide();
    assert!(contact.is_hidden);
    contact.unhide();
    assert!(!contact.is_hidden);

    // Test account management
    contact.add_accepted_account(1);
    contact.add_accepted_account(2);
    assert_eq!(contact.accepted_accounts.len(), 2);

    contact.remove_accepted_account(1);
    assert_eq!(contact.accepted_accounts.len(), 1);
    assert!(contact.accepted_accounts.contains(&2));
}

#[test]
fn test_reject_contact_request() {
    // Test rejecting/removing contact requests

    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);

    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);

    // Receive incoming request
    managed_a
        .add_incoming_contact_request(
            create_contact_request(id_b, id_a, 0, 1000),
            &noop_persister(),
        )
        .expect("setup persists");

    assert_eq!(managed_a.dashpay().incoming_contact_requests().len(), 1);

    // Reject by removing the request
    let (removed, _cs) = managed_a.remove_incoming_contact_request(&id_b);
    assert!(removed.is_some());
    assert_eq!(managed_a.dashpay().incoming_contact_requests().len(), 0);
}

#[test]
fn test_cancel_sent_contact_request() {
    // Test canceling a sent contact request

    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);

    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);

    // Send request
    managed_a
        .add_sent_contact_request(
            create_contact_request(id_a, id_b, 0, 1000),
            &noop_persister(),
        )
        .expect("setup persists");

    assert_eq!(managed_a.dashpay().sent_contact_requests().len(), 1);

    // Cancel by removing the request
    let (removed, _cs) = managed_a.remove_sent_contact_request(&id_b);
    assert!(removed.is_some());
    assert_eq!(managed_a.dashpay().sent_contact_requests().len(), 0);
}

#[test]
fn test_contact_request_with_different_account_references() {
    // Test contact requests with different account references
    // This represents different DashPay receiving accounts

    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);

    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);

    // Send request with account reference 0
    let mut request_a_to_b = create_contact_request(id_a, id_b, 0, 1000);
    request_a_to_b.account_reference = 0;
    managed_a
        .add_sent_contact_request(request_a_to_b.clone(), &noop_persister())
        .expect("setup persists");

    // Receive reciprocal request with account reference 1
    let mut request_b_to_a = create_contact_request(id_b, id_a, 1, 1001);
    request_b_to_a.account_reference = 1;
    managed_a
        .add_incoming_contact_request(request_b_to_a, &noop_persister())
        .expect("setup persists");

    // Should establish contact
    assert_eq!(managed_a.dashpay().established_contacts().len(), 1);

    let contact = managed_a
        .dashpay()
        .established_contacts()
        .get(&id_b)
        .unwrap();
    assert_eq!(contact.outgoing_request.account_reference, 0);
    assert_eq!(contact.incoming_request.account_reference, 1);
}

// `test_identity_label_management` was deleted alongside the
// `ManagedIdentity.label` field. Per-identity user-facing labels live
// on the Swift `PersistentIdentity.alias` column now, not on the
// in-memory wallet object.

#[test]
fn test_concurrent_bidirectional_requests() {
    // Test when both parties send requests at nearly the same time
    // This can happen in real-world scenarios

    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);

    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);
    let mut managed_b = ManagedIdentity::new(identity_b, 1);

    // Both send requests "simultaneously"
    let request_a_to_b = create_contact_request(id_a, id_b, 0, 1000);
    let request_b_to_a = create_contact_request(id_b, id_a, 0, 1001);

    managed_a
        .add_sent_contact_request(request_a_to_b.clone(), &noop_persister())
        .expect("setup persists");
    managed_b
        .add_sent_contact_request(request_b_to_a.clone(), &noop_persister())
        .expect("setup persists");

    // Both have sent requests pending
    assert_eq!(managed_a.dashpay().sent_contact_requests().len(), 1);
    assert_eq!(managed_b.dashpay().sent_contact_requests().len(), 1);

    // Now they receive each other's requests
    managed_a
        .add_incoming_contact_request(request_b_to_a, &noop_persister())
        .expect("setup persists");
    managed_b
        .add_incoming_contact_request(request_a_to_b, &noop_persister())
        .expect("setup persists");

    // Both should have auto-established
    assert_eq!(managed_a.dashpay().established_contacts().len(), 1);
    assert_eq!(managed_b.dashpay().established_contacts().len(), 1);
    assert_eq!(managed_a.dashpay().sent_contact_requests().len(), 0);
    assert_eq!(managed_b.dashpay().sent_contact_requests().len(), 0);
}

/// G3 receive side: a rotation request (same sender, bumped
/// accountReference) must supersede the tracked state instead of being
/// dropped as a duplicate.
///
/// - Established contact: the incoming request is replaced with the
///   rotated one and `payment_channel_broken` is cleared — a
///   superseding request is exactly the recovery the broken flag's
///   contract promises ("wait for the contact to send a new request").
/// - Pending incoming (not yet accepted): the entry is replaced so a
///   later Accept uses the freshest key material.
#[test]
fn test_rotation_request_rekeys_established_contact_and_clears_broken_flag() {
    let identity_a = create_test_identity([1u8; 32]);
    let identity_b = create_test_identity([2u8; 32]);
    let id_a = identity_a.id();
    let id_b = identity_b.id();

    let mut managed_a = ManagedIdentity::new(identity_a, 0);

    // Establish A <-> B (A sent, then B's reciprocal arrives).
    let request_a_to_b = create_contact_request(id_a, id_b, 0, 1000);
    let request_b_to_a = create_contact_request(id_b, id_a, 0, 1001);
    managed_a
        .add_sent_contact_request(request_a_to_b, &noop_persister())
        .expect("setup persists");
    managed_a
        .add_incoming_contact_request(request_b_to_a, &noop_persister())
        .expect("setup persists");
    assert_eq!(managed_a.dashpay().established_contacts().len(), 1);

    // Simulate a broken payment channel — e.g. the old request's
    // xpub stopped decrypting after B rotated keys.
    managed_a
        .established_contact_mut(&id_b)
        .expect("established contact")
        .payment_channel_broken = true;

    // B rotates: a new request with a different accountReference.
    let rotated = create_contact_request(id_b, id_a, 7, 2000);
    let rekeyed = managed_a
        .apply_rotated_incoming_request(rotated.clone(), &noop_persister())
        .expect("rotation persists in test");

    assert!(rekeyed, "an established contact must report re-keying");
    let contact = managed_a
        .dashpay()
        .established_contacts()
        .get(&id_b)
        .expect("contact still established");
    assert_eq!(
        contact.incoming_request.account_reference, 7,
        "the rotated request must supersede the tracked incoming request"
    );
    assert_eq!(
        contact.incoming_request.created_at, 2000,
        "the rotated request's payload must be the new one"
    );
    assert!(
        !contact.payment_channel_broken,
        "a superseding request must clear the broken-channel flag"
    );

    // Pending-incoming variant: a not-yet-accepted request is replaced.
    let identity_c = create_test_identity([3u8; 32]);
    let id_c = identity_c.id();
    let pending = create_contact_request(id_c, id_a, 0, 3000);
    managed_a
        .add_incoming_contact_request(pending, &noop_persister())
        .expect("setup persists");
    let rotated_pending = create_contact_request(id_c, id_a, 4, 3001);
    let rekeyed = managed_a
        .apply_rotated_incoming_request(rotated_pending, &noop_persister())
        .expect("rotation persists in test");
    assert!(
        !rekeyed,
        "pending (non-established) rotation is not a re-key"
    );
    assert_eq!(
        managed_a
            .dashpay()
            .incoming_contact_requests()
            .get(&id_c)
            .expect("pending request still tracked")
            .account_reference,
        4,
        "the pending entry must be replaced by the rotated request"
    );

    // Unknown sender: a no-op (fresh requests go through
    // add_incoming_contact_request).
    let identity_d = create_test_identity([4u8; 32]);
    let stranger = create_contact_request(identity_d.id(), id_a, 0, 4000);
    assert!(!managed_a
        .apply_rotated_incoming_request(stranger, &noop_persister())
        .expect("rotation persists in test"));
    assert!(!managed_a
        .dashpay()
        .incoming_contact_requests()
        .contains_key(&identity_d.id()));
}
