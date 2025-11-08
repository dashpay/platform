//! Comprehensive unit tests for platform-wallet-ffi
//!
//! These tests cover all functionality with realistic fake data

mod test_data;

use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet_ffi::*;
use std::ffi::CString;
use test_data::identities;
use test_data::scenarios;

#[test]
fn test_contact_request_field_access() {
    // Create Alice with outgoing requests
    let (alice, requests) = scenarios::alice_with_pending_sent_requests();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let bob = identities::bob();
    let bob_id_bytes: IdentifierBytes = bob.identity.id().into();

    let mut error = PlatformWalletFFIError::success();

    // Get the contact request for Bob
    let mut request_handle: Handle = NULL_HANDLE;
    let result = managed_identity_get_sent_contact_request(
        alice_handle,
        bob_id_bytes,
        &mut request_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_ne!(request_handle, NULL_HANDLE);

    // Verify sender ID
    let mut sender_id = IdentifierBytes { bytes: [0u8; 32] };
    let result = contact_request_get_sender_id(request_handle, &mut sender_id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(sender_id.bytes, [1u8; 32]); // Alice's ID

    // Verify recipient ID
    let mut recipient_id = IdentifierBytes { bytes: [0u8; 32] };
    let result = contact_request_get_recipient_id(request_handle, &mut recipient_id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(recipient_id.bytes, [2u8; 32]); // Bob's ID

    // Verify sender key index
    let mut sender_key_idx = 999u32;
    let result = contact_request_get_sender_key_index(request_handle, &mut sender_key_idx, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(sender_key_idx, 0);

    // Verify recipient key index
    let mut recipient_key_idx = 999u32;
    let result = contact_request_get_recipient_key_index(request_handle, &mut recipient_key_idx, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(recipient_key_idx, 1);

    // Verify account reference
    let mut account_ref = 999u32;
    let result = contact_request_get_account_reference(request_handle, &mut account_ref, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(account_ref, 0);

    // Verify timestamp
    let mut created_at = 0u64;
    let result = contact_request_get_created_at(request_handle, &mut created_at, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(created_at, 1_700_000_000);

    // Verify encrypted public key
    let mut bytes_ptr: *mut u8 = std::ptr::null_mut();
    let mut len: usize = 0;
    let result = contact_request_get_encrypted_public_key(
        request_handle,
        &mut bytes_ptr,
        &mut len,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert!(!bytes_ptr.is_null());
    assert_eq!(len, 96);

    // Cleanup
    platform_wallet_bytes_free(bytes_ptr, len);
    contact_request_destroy(request_handle);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_incoming_contact_request_retrieval() {
    // Create Alice with incoming requests
    let (alice, requests) = scenarios::alice_with_pending_incoming_requests();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let bob = identities::bob();
    let bob_id_bytes: IdentifierBytes = bob.identity.id().into();

    let mut error = PlatformWalletFFIError::success();

    // Get the incoming contact request from Bob
    let mut request_handle: Handle = NULL_HANDLE;
    let result = managed_identity_get_incoming_contact_request(
        alice_handle,
        bob_id_bytes,
        &mut request_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_ne!(request_handle, NULL_HANDLE);

    // Verify it's from Bob to Alice
    let mut sender_id = IdentifierBytes { bytes: [0u8; 32] };
    let result = contact_request_get_sender_id(request_handle, &mut sender_id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(sender_id.bytes, [2u8; 32]); // Bob's ID

    let mut recipient_id = IdentifierBytes { bytes: [0u8; 32] };
    let result = contact_request_get_recipient_id(request_handle, &mut recipient_id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(recipient_id.bytes, [1u8; 32]); // Alice's ID

    // Cleanup
    contact_request_destroy(request_handle);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_multiple_contact_requests() {
    // Create Alice with 3 outgoing requests
    let (alice, requests) = scenarios::alice_with_pending_sent_requests();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let mut error = PlatformWalletFFIError::success();

    // Get all sent contact request IDs
    let mut array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    let result = managed_identity_get_sent_contact_request_ids(alice_handle, &mut array, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(array.count, 3);

    // Verify we can retrieve each request
    for i in 0..array.count {
        let id_bytes = unsafe { *array.items.add(i) };
        let mut request_handle: Handle = NULL_HANDLE;
        let result = managed_identity_get_sent_contact_request(
            alice_handle,
            id_bytes,
            &mut request_handle,
            &mut error,
        );
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(request_handle, NULL_HANDLE);

        contact_request_destroy(request_handle);
    }

    // Cleanup
    platform_wallet_identifier_array_free(array);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_established_contacts() {
    // Create Alice with established contacts
    let (alice, contacts) = scenarios::alice_with_established_contacts();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let mut error = PlatformWalletFFIError::success();

    // Get all established contact IDs
    let mut array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    let result = managed_identity_get_established_contact_ids(alice_handle, &mut array, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(array.count, 2); // Bob and Carol

    // Check if Bob is established
    let bob_id_bytes: IdentifierBytes = identities::bob().identity.id().into();
    let mut is_established = false;
    let result = managed_identity_is_contact_established(
        alice_handle,
        bob_id_bytes,
        &mut is_established,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert!(is_established);

    // Check if Dave is NOT established
    let dave_id_bytes: IdentifierBytes = identities::dave().identity.id().into();
    let mut is_established = true;
    let result = managed_identity_is_contact_established(
        alice_handle,
        dave_id_bytes,
        &mut is_established,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert!(!is_established);

    // Cleanup
    platform_wallet_identifier_array_free(array);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_mixed_contact_scenario() {
    // Create Alice with all types of contacts
    let alice = scenarios::alice_with_mixed_contacts();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let mut error = PlatformWalletFFIError::success();

    // Verify established contacts count
    let mut established_array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_established_contact_ids(alice_handle, &mut established_array, &mut error);
    assert_eq!(established_array.count, 1); // Only Bob

    // Verify sent requests count
    let mut sent_array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_sent_contact_request_ids(alice_handle, &mut sent_array, &mut error);
    assert_eq!(sent_array.count, 1); // Only Carol

    // Verify incoming requests count
    let mut incoming_array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_incoming_contact_request_ids(alice_handle, &mut incoming_array, &mut error);
    assert_eq!(incoming_array.count, 2); // Dave and Eve

    // Cleanup
    platform_wallet_identifier_array_free(established_array);
    platform_wallet_identifier_array_free(sent_array);
    platform_wallet_identifier_array_free(incoming_array);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_identity_manager_with_multiple_identities() {
    use dpp::identity::accessors::IdentityGettersV0;

    let mut error = PlatformWalletFFIError::success();

    // Create identity manager
    let mut manager_handle: Handle = NULL_HANDLE;
    let result = identity_manager_create(&mut manager_handle, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Add Alice, Bob, and Carol
    let alice = identities::alice();
    let bob = identities::bob();
    let carol = identities::carol();

    let alice_id = alice.identity.id();
    let bob_id = bob.identity.id();
    let carol_id = carol.identity.id();

    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);
    let bob_handle = MANAGED_IDENTITY_STORAGE.insert(bob);
    let carol_handle = MANAGED_IDENTITY_STORAGE.insert(carol);

    identity_manager_add_identity(manager_handle, alice_handle, &mut error);
    identity_manager_add_identity(manager_handle, bob_handle, &mut error);
    identity_manager_add_identity(manager_handle, carol_handle, &mut error);

    // Verify count
    let mut count: usize = 0;
    let result = identity_manager_get_identity_count(manager_handle, &mut count, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(count, 3);

    // Get all identity IDs
    let mut array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    let result = identity_manager_get_all_identity_ids(manager_handle, &mut array, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(array.count, 3);

    // Set Alice as primary
    let alice_id_bytes: IdentifierBytes = alice_id.into();
    let result =
        identity_manager_set_primary_identity(manager_handle, alice_id_bytes, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Get primary
    let mut primary_id = IdentifierBytes { bytes: [0u8; 32] };
    let result =
        identity_manager_get_primary_identity_id(manager_handle, &mut primary_id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(primary_id.bytes, alice_id_bytes.bytes);

    // Cleanup
    platform_wallet_identifier_array_free(array);
    identity_manager_destroy(manager_handle);
}

#[test]
fn test_managed_identity_label_operations() {
    let alice = identities::alice();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let mut error = PlatformWalletFFIError::success();

    // Get current label
    let mut label_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
    let result = managed_identity_get_label(alice_handle, &mut label_ptr, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert!(!label_ptr.is_null());

    let label = unsafe { std::ffi::CStr::from_ptr(label_ptr).to_str().unwrap() };
    assert_eq!(label, "Alice");
    platform_wallet_string_free(label_ptr);

    // Set new label
    let new_label = CString::new("Alice the Great").unwrap();
    let result = managed_identity_set_label(alice_handle, new_label.as_ptr(), &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Verify new label
    let mut label_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
    let result = managed_identity_get_label(alice_handle, &mut label_ptr, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);

    let label = unsafe { std::ffi::CStr::from_ptr(label_ptr).to_str().unwrap() };
    assert_eq!(label, "Alice the Great");
    platform_wallet_string_free(label_ptr);

    // Cleanup
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_managed_identity_balance_and_block_time() {
    use dpp::identity::accessors::IdentityGettersV0;

    let alice = identities::alice();
    let expected_balance = alice.identity.balance();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let mut error = PlatformWalletFFIError::success();

    // Get balance
    let mut balance: u64 = 0;
    let result = managed_identity_get_balance(alice_handle, &mut balance, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(balance, expected_balance);

    // Set balance block time
    let block_time = BlockTime {
        height: 123_456,
        core_height: 987_654,
        timestamp: 1_700_000_000,
    };
    let result =
        managed_identity_set_last_updated_balance_block_time(alice_handle, block_time, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Get balance block time
    let mut retrieved_bt = BlockTime {
        height: 0,
        core_height: 0,
        timestamp: 0,
    };
    let result = managed_identity_get_last_updated_balance_block_time(
        alice_handle,
        &mut retrieved_bt,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(retrieved_bt.height, 123_456);
    assert_eq!(retrieved_bt.core_height, 987_654);
    assert_eq!(retrieved_bt.timestamp, 1_700_000_000);

    // Cleanup
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_error_handling_invalid_handles() {
    let mut error = PlatformWalletFFIError::success();
    let invalid_handle = 99999;

    // Try to get identity with invalid handle
    let mut id_bytes = IdentifierBytes { bytes: [0u8; 32] };
    let result = managed_identity_get_id(invalid_handle, &mut id_bytes, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
    assert!(!error.message.is_null());
    platform_wallet_ffi_error_free(error);

    // Try to get contact request with invalid handle
    error = PlatformWalletFFIError::success();
    let result = contact_request_get_sender_id(invalid_handle, &mut id_bytes, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
    platform_wallet_ffi_error_free(error);

    // Try to destroy invalid handle
    let result = managed_identity_destroy(invalid_handle);
    assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
}

#[test]
fn test_error_handling_null_pointers() {
    let alice = identities::alice();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    // Try to get ID with null output pointer
    let result = managed_identity_get_id(alice_handle, std::ptr::null_mut(), std::ptr::null_mut());
    assert_eq!(result, PlatformWalletFFIResult::ErrorNullPointer);

    // Try to get balance with null output pointer
    let result =
        managed_identity_get_balance(alice_handle, std::ptr::null_mut(), std::ptr::null_mut());
    assert_eq!(result, PlatformWalletFFIResult::ErrorNullPointer);

    // Try to get sent requests with null output pointer
    let result = managed_identity_get_sent_contact_request_ids(
        alice_handle,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    assert_eq!(result, PlatformWalletFFIResult::ErrorNullPointer);

    // Cleanup
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_contact_request_not_found() {
    let alice = identities::alice(); // Has no contacts by default
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let mut error = PlatformWalletFFIError::success();
    let eve_id_bytes: IdentifierBytes = identities::eve().identity.id().into();

    // Try to get non-existent sent request
    let mut request_handle: Handle = NULL_HANDLE;
    let result = managed_identity_get_sent_contact_request(
        alice_handle,
        eve_id_bytes,
        &mut request_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::ErrorContactNotFound);
    assert!(!error.message.is_null());

    // Cleanup
    platform_wallet_ffi_error_free(error);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_identifier_operations() {
    let mut error = PlatformWalletFFIError::success();

    // Generate random identifier
    let mut id = IdentifierBytes { bytes: [0u8; 32] };
    let result = platform_wallet_generate_random_identifier(&mut id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    // Should not be all zeros
    assert_ne!(id.bytes, [0u8; 32]);

    // Convert to string (actually Base58, despite function name)
    let mut id_string: *mut std::os::raw::c_char = std::ptr::null_mut();
    let result = platform_wallet_identifier_to_hex(id, &mut id_string, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert!(!id_string.is_null());

    let id_str = unsafe { std::ffi::CStr::from_ptr(id_string).to_str().unwrap() };
    assert_eq!(id_str.len(), 44); // Base58-encoded identifier is 44 chars

    // Convert back from string
    let mut id2 = IdentifierBytes { bytes: [0u8; 32] };
    let result = platform_wallet_identifier_from_hex(id_string, &mut id2, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Should match original
    assert_eq!(id.bytes, id2.bytes);

    // Cleanup
    platform_wallet_string_free(id_string);
}

#[test]
fn test_memory_lifecycle() {
    // Test proper creation and destruction of multiple objects

    // Create multiple managed identities
    let alice = identities::alice();
    let bob = identities::bob();
    let carol = identities::carol();

    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);
    let bob_handle = MANAGED_IDENTITY_STORAGE.insert(bob);
    let carol_handle = MANAGED_IDENTITY_STORAGE.insert(carol);

    // Verify they exist
    let mut error = PlatformWalletFFIError::success();
    let mut id = IdentifierBytes { bytes: [0u8; 32] };

    assert_eq!(
        managed_identity_get_id(alice_handle, &mut id, &mut error),
        PlatformWalletFFIResult::Success
    );
    assert_eq!(
        managed_identity_get_id(bob_handle, &mut id, &mut error),
        PlatformWalletFFIResult::Success
    );
    assert_eq!(
        managed_identity_get_id(carol_handle, &mut id, &mut error),
        PlatformWalletFFIResult::Success
    );

    // Destroy Alice
    assert_eq!(
        managed_identity_destroy(alice_handle),
        PlatformWalletFFIResult::Success
    );

    // Alice should be gone, but Bob and Carol should still exist
    assert_eq!(
        managed_identity_get_id(alice_handle, &mut id, &mut error),
        PlatformWalletFFIResult::ErrorInvalidHandle
    );
    platform_wallet_ffi_error_free(error);
    error = PlatformWalletFFIError::success();

    assert_eq!(
        managed_identity_get_id(bob_handle, &mut id, &mut error),
        PlatformWalletFFIResult::Success
    );
    assert_eq!(
        managed_identity_get_id(carol_handle, &mut id, &mut error),
        PlatformWalletFFIResult::Success
    );

    // Cleanup remaining
    managed_identity_destroy(bob_handle);
    managed_identity_destroy(carol_handle);

    // Double destroy should fail
    assert_eq!(
        managed_identity_destroy(bob_handle),
        PlatformWalletFFIResult::ErrorInvalidHandle
    );
}

#[test]
fn test_concurrent_identity_operations() {
    // Test that operations on different identities don't interfere

    let (alice, alice_requests) = scenarios::alice_with_pending_sent_requests();
    let (bob, bob_requests) = scenarios::alice_with_pending_incoming_requests(); // Reuse for Bob
    let carol = scenarios::alice_with_mixed_contacts(); // Reuse for Carol

    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);
    let bob_handle = MANAGED_IDENTITY_STORAGE.insert(bob);
    let carol_handle = MANAGED_IDENTITY_STORAGE.insert(carol);

    let mut error = PlatformWalletFFIError::success();

    // Verify Alice has 3 sent requests
    let mut alice_array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_sent_contact_request_ids(alice_handle, &mut alice_array, &mut error);
    assert_eq!(alice_array.count, 3);

    // Verify Bob has 3 incoming requests (we reused the scenario)
    let mut bob_array = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_incoming_contact_request_ids(bob_handle, &mut bob_array, &mut error);
    assert_eq!(bob_array.count, 3);

    // Verify Carol has mixed contacts
    let mut carol_sent = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_sent_contact_request_ids(carol_handle, &mut carol_sent, &mut error);
    assert_eq!(carol_sent.count, 1);

    // Operations on Alice shouldn't affect Bob or Carol
    let new_label = CString::new("Alice Updated").unwrap();
    managed_identity_set_label(alice_handle, new_label.as_ptr(), &mut error);

    // Bob's incoming requests should still be 3
    let mut bob_array2 = IdentifierArray {
        items: std::ptr::null_mut(),
        count: 0,
    };
    managed_identity_get_incoming_contact_request_ids(bob_handle, &mut bob_array2, &mut error);
    assert_eq!(bob_array2.count, 3);

    // Cleanup
    platform_wallet_identifier_array_free(alice_array);
    platform_wallet_identifier_array_free(bob_array);
    platform_wallet_identifier_array_free(bob_array2);
    platform_wallet_identifier_array_free(carol_sent);
    managed_identity_destroy(alice_handle);
    managed_identity_destroy(bob_handle);
    managed_identity_destroy(carol_handle);
}

// ============================================================================
// EstablishedContact FFI Tests
// ============================================================================

#[test]
fn test_get_established_contact_and_fields() {
    use dpp::identity::accessors::IdentityGettersV0;
    
    // Create Alice with established contacts
    let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

    let bob_id = test_data::identities::bob().identity.id();
    let bob_id_bytes: IdentifierBytes = bob_id.into();

    let mut contact_handle: Handle = NULL_HANDLE;
    let mut error = PlatformWalletFFIError::success();

    // Get established contact
    let result = managed_identity_get_established_contact(
        alice_handle,
        bob_id_bytes,
        &mut contact_handle,
        &mut error,
    );

    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_ne!(contact_handle, NULL_HANDLE);

    // Get contact ID
    let mut retrieved_id = IdentifierBytes { bytes: [0u8; 32] };
    let result = established_contact_get_contact_id(contact_handle, &mut retrieved_id, &mut error);
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(retrieved_id.bytes, bob_id_bytes.bytes);

    // Cleanup
    established_contact_destroy(contact_handle);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_established_contact_outgoing_and_incoming_requests() {
    use dpp::identity::accessors::IdentityGettersV0;
    
    let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

    let bob_id = test_data::identities::bob().identity.id();
    let bob_id_bytes: IdentifierBytes = bob_id.into();

    let mut contact_handle: Handle = NULL_HANDLE;
    let mut error = PlatformWalletFFIError::success();

    managed_identity_get_established_contact(
        alice_handle,
        bob_id_bytes,
        &mut contact_handle,
        &mut error,
    );

    // Get outgoing request
    let mut outgoing_handle: Handle = NULL_HANDLE;
    let result = established_contact_get_outgoing_request(
        contact_handle,
        &mut outgoing_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_ne!(outgoing_handle, NULL_HANDLE);

    // Get incoming request
    let mut incoming_handle: Handle = NULL_HANDLE;
    let result = established_contact_get_incoming_request(
        contact_handle,
        &mut incoming_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_ne!(incoming_handle, NULL_HANDLE);

    // Verify the requests have correct sender/recipient
    let alice_id = alice.identity.id();
    let mut sender_id = IdentifierBytes { bytes: [0u8; 32] };
    let mut recipient_id = IdentifierBytes { bytes: [0u8; 32] };

    // Outgoing: from alice to bob
    contact_request_get_sender_id(outgoing_handle, &mut sender_id, &mut error);
    contact_request_get_recipient_id(outgoing_handle, &mut recipient_id, &mut error);
    assert_eq!(sender_id.bytes, IdentifierBytes::from(alice_id).bytes);
    assert_eq!(recipient_id.bytes, bob_id_bytes.bytes);

    // Incoming: from bob to alice
    contact_request_get_sender_id(incoming_handle, &mut sender_id, &mut error);
    contact_request_get_recipient_id(incoming_handle, &mut recipient_id, &mut error);
    assert_eq!(sender_id.bytes, bob_id_bytes.bytes);
    assert_eq!(recipient_id.bytes, IdentifierBytes::from(alice_id).bytes);

    // Cleanup
    contact_request_destroy(outgoing_handle);
    contact_request_destroy(incoming_handle);
    established_contact_destroy(contact_handle);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_established_contact_request_fields() {
    use dpp::identity::accessors::IdentityGettersV0;
    
    let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

    let bob_id = test_data::identities::bob().identity.id();
    let bob_id_bytes: IdentifierBytes = bob_id.into();

    let mut contact_handle: Handle = NULL_HANDLE;
    let mut error = PlatformWalletFFIError::success();

    managed_identity_get_established_contact(
        alice_handle,
        bob_id_bytes,
        &mut contact_handle,
        &mut error,
    );

    // Get outgoing request and verify all fields
    let mut outgoing_handle: Handle = NULL_HANDLE;
    established_contact_get_outgoing_request(contact_handle, &mut outgoing_handle, &mut error);

    let mut sender_key_idx: u32 = 0;
    let mut recipient_key_idx: u32 = 0;
    let mut account_ref: u32 = 0;
    let mut created_at: u64 = 0;

    contact_request_get_sender_key_index(outgoing_handle, &mut sender_key_idx, &mut error);
    contact_request_get_recipient_key_index(outgoing_handle, &mut recipient_key_idx, &mut error);
    contact_request_get_account_reference(outgoing_handle, &mut account_ref, &mut error);
    contact_request_get_created_at(outgoing_handle, &mut created_at, &mut error);

    // The test data should have specific values
    assert_eq!(sender_key_idx, 0);
    assert_eq!(recipient_key_idx, 1);
    assert_eq!(account_ref, 0);
    assert!(created_at > 0);

    // Get encrypted public key
    let mut bytes_ptr: *mut std::os::raw::c_uchar = std::ptr::null_mut();
    let mut len: usize = 0;
    let result = contact_request_get_encrypted_public_key(
        outgoing_handle,
        &mut bytes_ptr,
        &mut len,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);
    assert_eq!(len, 96); // Standard encrypted key length
    assert!(!bytes_ptr.is_null());

    // Cleanup
    platform_wallet_bytes_free(bytes_ptr, len);
    contact_request_destroy(outgoing_handle);
    established_contact_destroy(contact_handle);
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_get_nonexistent_established_contact() {
    let alice = test_data::identities::alice();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

    let nonexistent_id = IdentifierBytes { bytes: [99u8; 32] };

    let mut contact_handle: Handle = NULL_HANDLE;
    let mut error = PlatformWalletFFIError::success();

    let result = managed_identity_get_established_contact(
        alice_handle,
        nonexistent_id,
        &mut contact_handle,
        &mut error,
    );

    assert_eq!(result, PlatformWalletFFIResult::ErrorContactNotFound);
    assert_eq!(contact_handle, NULL_HANDLE);

    // Cleanup
    managed_identity_destroy(alice_handle);
}

#[test]
fn test_established_contact_destroy_invalid_handle() {
    let result = established_contact_destroy(9999);
    assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
}

#[test]
fn test_multiple_established_contacts() {
    use dpp::identity::accessors::IdentityGettersV0;
    
    let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
    let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

    let bob_id = test_data::identities::bob().identity.id();
    let carol_id = test_data::identities::carol().identity.id();

    let mut error = PlatformWalletFFIError::success();

    // Get Bob contact
    let mut bob_contact_handle: Handle = NULL_HANDLE;
    let result = managed_identity_get_established_contact(
        alice_handle,
        bob_id.into(),
        &mut bob_contact_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Get Carol contact
    let mut carol_contact_handle: Handle = NULL_HANDLE;
    let result = managed_identity_get_established_contact(
        alice_handle,
        carol_id.into(),
        &mut carol_contact_handle,
        &mut error,
    );
    assert_eq!(result, PlatformWalletFFIResult::Success);

    // Verify Bob's contact ID
    let mut retrieved_bob_id = IdentifierBytes { bytes: [0u8; 32] };
    established_contact_get_contact_id(bob_contact_handle, &mut retrieved_bob_id, &mut error);
    assert_eq!(retrieved_bob_id.bytes, IdentifierBytes::from(bob_id).bytes);

    // Verify Carol's contact ID
    let mut retrieved_carol_id = IdentifierBytes { bytes: [0u8; 32] };
    established_contact_get_contact_id(carol_contact_handle, &mut retrieved_carol_id, &mut error);
    assert_eq!(retrieved_carol_id.bytes, IdentifierBytes::from(carol_id).bytes);

    // Cleanup
    established_contact_destroy(bob_contact_handle);
    established_contact_destroy(carol_contact_handle);
    managed_identity_destroy(alice_handle);
}
