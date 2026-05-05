//! Comprehensive unit tests for platform-wallet-ffi
//!
//! These tests cover all functionality with realistic fake data.
//!
//! Updated to the pointer-passing FFI ABI (every former
//! `IdentifierBytes` slot is a `*const u8` / `*mut u8` to a 32-byte
//! buffer; every former by-value free is `&mut`). See the EXC_BAD_ACCESS
//! sweep for the rationale.
//!
//! Also updated to the unified-result FFI ABI: every entry point returns
//! a `PlatformWalletFFIResult` (with `.code` + `.message`) instead of
//! taking a separate `&mut PlatformWalletFFIError` out-parameter. Storage
//! misses surface as `NotFound` via `unwrap_option_or_return!`; only the
//! handful of destroy entry points still return `ErrorInvalidHandle`
//! directly.

mod test_data;

use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet_ffi::*;
use std::ffi::CString;
use test_data::identities;
use test_data::scenarios;

#[test]
fn test_contact_request_field_access() {
    unsafe {
        // Create Alice with outgoing requests
        let (alice, _requests) = scenarios::alice_with_pending_sent_requests();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        let bob = identities::bob();
        let bob_id_bytes: [u8; 32] = bob.identity.id().to_buffer();

        // Get the contact request for Bob
        let mut request_handle: Handle = NULL_HANDLE;
        let result = managed_identity_get_sent_contact_request(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut request_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(request_handle, NULL_HANDLE);

        // Verify sender ID
        let mut sender_id = [0u8; 32];
        let result = contact_request_get_sender_id(request_handle, sender_id.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(sender_id, [1u8; 32]); // Alice's ID

        // Verify recipient ID
        let mut recipient_id = [0u8; 32];
        let result = contact_request_get_recipient_id(request_handle, recipient_id.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(recipient_id, [2u8; 32]); // Bob's ID

        // Verify sender key index
        let mut sender_key_idx = 999u32;
        let result = contact_request_get_sender_key_index(request_handle, &mut sender_key_idx);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(sender_key_idx, 0);

        // Verify recipient key index
        let mut recipient_key_idx = 999u32;
        let result =
            contact_request_get_recipient_key_index(request_handle, &mut recipient_key_idx);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(recipient_key_idx, 1);

        // Verify account reference
        let mut account_ref = 999u32;
        let result = contact_request_get_account_reference(request_handle, &mut account_ref);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(account_ref, 0);

        // Verify timestamp
        let mut created_at = 0u64;
        let result = contact_request_get_created_at(request_handle, &mut created_at);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(created_at, 1_700_000_000);

        // Verify encrypted public key
        let mut bytes_ptr: *mut u8 = std::ptr::null_mut();
        let mut len: usize = 0;
        let result =
            contact_request_get_encrypted_public_key(request_handle, &mut bytes_ptr, &mut len);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!bytes_ptr.is_null());
        assert_eq!(len, 96);

        // Cleanup
        platform_wallet_bytes_free(bytes_ptr, len);
        contact_request_destroy(request_handle);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_incoming_contact_request_retrieval() {
    unsafe {
        // Create Alice with incoming requests
        let (alice, _requests) = scenarios::alice_with_pending_incoming_requests();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        let bob = identities::bob();
        let bob_id_bytes: [u8; 32] = bob.identity.id().to_buffer();

        // Get the incoming contact request from Bob
        let mut request_handle: Handle = NULL_HANDLE;
        let result = managed_identity_get_incoming_contact_request(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut request_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(request_handle, NULL_HANDLE);

        // Verify it's from Bob to Alice
        let mut sender_id = [0u8; 32];
        let result = contact_request_get_sender_id(request_handle, sender_id.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(sender_id, [2u8; 32]); // Bob's ID

        let mut recipient_id = [0u8; 32];
        let result = contact_request_get_recipient_id(request_handle, recipient_id.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(recipient_id, [1u8; 32]); // Alice's ID

        // Cleanup
        contact_request_destroy(request_handle);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_multiple_contact_requests() {
    unsafe {
        // Create Alice with 3 outgoing requests
        let (alice, _requests) = scenarios::alice_with_pending_sent_requests();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        // Get all sent contact request IDs
        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = managed_identity_get_sent_contact_request_ids(alice_handle, &mut array);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(array.count, 3);

        // Verify we can retrieve each request
        for i in 0..array.count {
            // Each row is a flat 32-byte buffer; take a pointer into
            // the contiguous array storage.
            let row_ptr: *const u8 = (*array.items.add(i)).as_ptr();
            let mut request_handle: Handle = NULL_HANDLE;
            let result = managed_identity_get_sent_contact_request(
                alice_handle,
                row_ptr,
                &mut request_handle,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_ne!(request_handle, NULL_HANDLE);

            contact_request_destroy(request_handle);
        }

        // Cleanup
        platform_wallet_identifier_array_free(&mut array);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_established_contacts() {
    unsafe {
        // Create Alice with established contacts
        let (alice, _contacts) = scenarios::alice_with_established_contacts();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        // Get all established contact IDs
        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = managed_identity_get_established_contact_ids(alice_handle, &mut array);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(array.count, 2); // Bob and Carol

        // Check if Bob is established
        let bob_id_bytes: [u8; 32] = identities::bob().identity.id().to_buffer();
        let mut is_established = false;
        let result = managed_identity_is_contact_established(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut is_established,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(is_established);

        // Check if Dave is NOT established
        let dave_id_bytes: [u8; 32] = identities::dave().identity.id().to_buffer();
        let mut is_established = true;
        let result = managed_identity_is_contact_established(
            alice_handle,
            dave_id_bytes.as_ptr(),
            &mut is_established,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!is_established);

        // Cleanup
        platform_wallet_identifier_array_free(&mut array);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_mixed_contact_scenario() {
    unsafe {
        // Create Alice with all types of contacts
        let alice = scenarios::alice_with_mixed_contacts();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        // Verify established contacts count
        let mut established_array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_established_contact_ids(alice_handle, &mut established_array);
        assert_eq!(established_array.count, 1); // Only Bob

        // Verify sent requests count
        let mut sent_array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_sent_contact_request_ids(alice_handle, &mut sent_array);
        assert_eq!(sent_array.count, 1); // Only Carol

        // Verify incoming requests count
        let mut incoming_array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_incoming_contact_request_ids(alice_handle, &mut incoming_array);
        assert_eq!(incoming_array.count, 2); // Dave and Eve

        // Cleanup
        platform_wallet_identifier_array_free(&mut established_array);
        platform_wallet_identifier_array_free(&mut sent_array);
        platform_wallet_identifier_array_free(&mut incoming_array);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_identity_manager_with_multiple_identities() {
    unsafe {
        use dpp::identity::accessors::IdentityGettersV0;

        // Create identity manager
        let mut manager_handle: Handle = NULL_HANDLE;
        let result = identity_manager_create(&mut manager_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Add Alice, Bob, and Carol
        let alice = identities::alice();
        let bob = identities::bob();
        let carol = identities::carol();

        let alice_id = alice.identity.id();
        let _bob_id = bob.identity.id();
        let _carol_id = carol.identity.id();

        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);
        let bob_handle = MANAGED_IDENTITY_STORAGE.insert(bob);
        let carol_handle = MANAGED_IDENTITY_STORAGE.insert(carol);

        identity_manager_add_identity(manager_handle, alice_handle);
        identity_manager_add_identity(manager_handle, bob_handle);
        identity_manager_add_identity(manager_handle, carol_handle);

        // Verify count
        let mut count: usize = 0;
        let result = identity_manager_get_identity_count(manager_handle, &mut count);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(count, 3);

        // Get all identity IDs
        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = identity_manager_get_all_identity_ids(manager_handle, &mut array);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(array.count, 3);

        // Primary-identity FFI was dropped along with the field;
        // the test_data fixture's `alice_id` is no longer relevant
        // here.
        let alice_id_bytes: [u8; 32] = alice_id.to_buffer();
        let _ = alice_id_bytes;

        // Cleanup
        platform_wallet_identifier_array_free(&mut array);
        identity_manager_destroy(manager_handle);
    }
}

#[test]
fn test_managed_identity_label_operations() {
    // `ManagedIdentity` no longer carries a `label` field — the FFI
    // get/set entry points return null / no-op success respectively.
    // Verify the stubs still link and behave consistently.
    unsafe {
        let alice = identities::alice();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        let mut label_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = managed_identity_get_label(alice_handle, &mut label_ptr);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        // Stub returns null — labels live on `PersistentIdentity.alias`.
        assert!(label_ptr.is_null());

        let new_label = CString::new("Alice the Great").unwrap();
        let result = managed_identity_set_label(alice_handle, new_label.as_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Cleanup
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_managed_identity_balance_and_block_time() {
    unsafe {
        use dpp::identity::accessors::IdentityGettersV0;

        let alice = identities::alice();
        let expected_balance = alice.identity.balance();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        // Get balance
        let mut balance: u64 = 0;
        let result = managed_identity_get_balance(alice_handle, &mut balance);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(balance, expected_balance);

        // Set balance block time
        let block_time = BlockTime {
            height: 123_456,
            core_height: 987_654,
            timestamp: 1_700_000_000,
        };
        let result =
            managed_identity_set_last_updated_balance_block_time(alice_handle, &block_time);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Get balance block time
        let mut retrieved_bt = BlockTime {
            height: 0,
            core_height: 0,
            timestamp: 0,
        };
        let result =
            managed_identity_get_last_updated_balance_block_time(alice_handle, &mut retrieved_bt);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(retrieved_bt.height, 123_456);
        assert_eq!(retrieved_bt.core_height, 987_654);
        assert_eq!(retrieved_bt.timestamp, 1_700_000_000);

        // Cleanup
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_error_handling_invalid_handles() {
    unsafe {
        let invalid_handle = 99999;

        // Storage misses now route through `unwrap_option_or_return!`
        // and surface as `NotFound` (with a diagnostic message on the
        // result). The diagnostic remains accessible via `result.message`.
        let mut id_bytes = [0u8; 32];
        let result = managed_identity_get_id(invalid_handle, id_bytes.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert!(!result.message.is_null());

        let result = contact_request_get_sender_id(invalid_handle, id_bytes.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);

        // `managed_identity_destroy` retains its bespoke error mapping
        // because it doesn't use the option macro.
        let result = managed_identity_destroy(invalid_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
    }
}

#[test]
fn test_error_handling_null_pointers() {
    unsafe {
        let alice = identities::alice();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        // Try to get ID with null output pointer
        let result = managed_identity_get_id(alice_handle, std::ptr::null_mut());
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        // Try to get balance with null output pointer
        let result = managed_identity_get_balance(alice_handle, std::ptr::null_mut());
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        // Try to get sent requests with null output pointer
        let result =
            managed_identity_get_sent_contact_request_ids(alice_handle, std::ptr::null_mut());
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        // Cleanup
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_contact_request_not_found() {
    unsafe {
        let alice = identities::alice(); // Has no contacts by default
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        let eve_id_bytes: [u8; 32] = identities::eve().identity.id().to_buffer();

        // Try to get non-existent sent request — option lookup misses
        // surface as `NotFound` (the helper drives off
        // `unwrap_option_or_return!`).
        let mut request_handle: Handle = NULL_HANDLE;
        let result = managed_identity_get_sent_contact_request(
            alice_handle,
            eve_id_bytes.as_ptr(),
            &mut request_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert!(!result.message.is_null());

        // Cleanup
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_identifier_operations() {
    unsafe {
        // Generate random identifier
        let mut id = [0u8; 32];
        let result = platform_wallet_generate_random_identifier(id.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        // Should not be all zeros
        assert_ne!(id, [0u8; 32]);

        // Convert to string (actually Base58, despite function name)
        let mut id_string: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = platform_wallet_identifier_to_hex(id.as_ptr(), &mut id_string);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!id_string.is_null());

        let id_str = std::ffi::CStr::from_ptr(id_string).to_str().unwrap();
        // Base58-encoded 32-byte identifier is 43-44 chars (variable length encoding)
        assert!(
            id_str.len() == 43 || id_str.len() == 44,
            "Expected Base58 identifier length 43-44, got {}",
            id_str.len()
        );

        // Convert back from string
        let mut id2 = [0u8; 32];
        let result = platform_wallet_identifier_from_hex(id_string, id2.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Should match original
        assert_eq!(id, id2);

        // Cleanup
        platform_wallet_string_free(id_string);
    }
}

#[test]
fn test_memory_lifecycle() {
    unsafe {
        // Test proper creation and destruction of multiple objects

        // Create multiple managed identities
        let alice = identities::alice();
        let bob = identities::bob();
        let carol = identities::carol();

        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);
        let bob_handle = MANAGED_IDENTITY_STORAGE.insert(bob);
        let carol_handle = MANAGED_IDENTITY_STORAGE.insert(carol);

        // Verify they exist
        let mut id = [0u8; 32];

        assert_eq!(
            managed_identity_get_id(alice_handle, id.as_mut_ptr()).code,
            PlatformWalletFFIResultCode::Success
        );
        assert_eq!(
            managed_identity_get_id(bob_handle, id.as_mut_ptr()).code,
            PlatformWalletFFIResultCode::Success
        );
        assert_eq!(
            managed_identity_get_id(carol_handle, id.as_mut_ptr()).code,
            PlatformWalletFFIResultCode::Success
        );

        // Destroy Alice
        assert_eq!(
            managed_identity_destroy(alice_handle).code,
            PlatformWalletFFIResultCode::Success
        );

        // Alice should be gone — get_id surfaces the storage miss as
        // `NotFound`.
        assert_eq!(
            managed_identity_get_id(alice_handle, id.as_mut_ptr()).code,
            PlatformWalletFFIResultCode::NotFound
        );

        assert_eq!(
            managed_identity_get_id(bob_handle, id.as_mut_ptr()).code,
            PlatformWalletFFIResultCode::Success
        );
        assert_eq!(
            managed_identity_get_id(carol_handle, id.as_mut_ptr()).code,
            PlatformWalletFFIResultCode::Success
        );

        // Cleanup remaining
        managed_identity_destroy(bob_handle);
        managed_identity_destroy(carol_handle);

        // Double destroy still hits the bespoke ErrorInvalidHandle path.
        assert_eq!(
            managed_identity_destroy(bob_handle).code,
            PlatformWalletFFIResultCode::ErrorInvalidHandle
        );
    }
}

#[test]
fn test_concurrent_identity_operations() {
    unsafe {
        // Test that operations on different identities don't interfere

        let (alice, _alice_requests) = scenarios::alice_with_pending_sent_requests();
        let (bob, _bob_requests) = scenarios::alice_with_pending_incoming_requests(); // Reuse for Bob
        let carol = scenarios::alice_with_mixed_contacts(); // Reuse for Carol

        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);
        let bob_handle = MANAGED_IDENTITY_STORAGE.insert(bob);
        let carol_handle = MANAGED_IDENTITY_STORAGE.insert(carol);

        // Verify Alice has 3 sent requests
        let mut alice_array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_sent_contact_request_ids(alice_handle, &mut alice_array);
        assert_eq!(alice_array.count, 3);

        // Verify Bob has 3 incoming requests (we reused the scenario)
        let mut bob_array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_incoming_contact_request_ids(bob_handle, &mut bob_array);
        assert_eq!(bob_array.count, 3);

        // Verify Carol has mixed contacts
        let mut carol_sent = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_sent_contact_request_ids(carol_handle, &mut carol_sent);
        assert_eq!(carol_sent.count, 1);

        // Operations on Alice shouldn't affect Bob or Carol
        let new_label = CString::new("Alice Updated").unwrap();
        managed_identity_set_label(alice_handle, new_label.as_ptr());

        // Bob's incoming requests should still be 3
        let mut bob_array2 = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        managed_identity_get_incoming_contact_request_ids(bob_handle, &mut bob_array2);
        assert_eq!(bob_array2.count, 3);

        // Cleanup
        platform_wallet_identifier_array_free(&mut alice_array);
        platform_wallet_identifier_array_free(&mut bob_array);
        platform_wallet_identifier_array_free(&mut bob_array2);
        platform_wallet_identifier_array_free(&mut carol_sent);
        managed_identity_destroy(alice_handle);
        managed_identity_destroy(bob_handle);
        managed_identity_destroy(carol_handle);
    }
}

// ============================================================================
// EstablishedContact FFI Tests
// ============================================================================

#[test]
fn test_get_established_contact_and_fields() {
    unsafe {
        use dpp::identity::accessors::IdentityGettersV0;

        // Create Alice with established contacts
        let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

        let bob_id = test_data::identities::bob().identity.id();
        let bob_id_bytes: [u8; 32] = bob_id.to_buffer();

        let mut contact_handle: Handle = NULL_HANDLE;

        // Get established contact
        let result = managed_identity_get_established_contact(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut contact_handle,
        );

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(contact_handle, NULL_HANDLE);

        // Get contact ID
        let mut retrieved_id = [0u8; 32];
        let result = established_contact_get_contact_id(contact_handle, retrieved_id.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(retrieved_id, bob_id_bytes);

        // Cleanup
        established_contact_destroy(contact_handle);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_established_contact_outgoing_and_incoming_requests() {
    unsafe {
        use dpp::identity::accessors::IdentityGettersV0;

        let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

        let bob_id = test_data::identities::bob().identity.id();
        let bob_id_bytes: [u8; 32] = bob_id.to_buffer();

        let mut contact_handle: Handle = NULL_HANDLE;

        managed_identity_get_established_contact(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut contact_handle,
        );

        // Get outgoing request
        let mut outgoing_handle: Handle = NULL_HANDLE;
        let result = established_contact_get_outgoing_request(contact_handle, &mut outgoing_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(outgoing_handle, NULL_HANDLE);

        // Get incoming request
        let mut incoming_handle: Handle = NULL_HANDLE;
        let result = established_contact_get_incoming_request(contact_handle, &mut incoming_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(incoming_handle, NULL_HANDLE);

        // Verify the requests have correct sender/recipient
        let alice_id = alice.identity.id();
        let alice_id_bytes: [u8; 32] = alice_id.to_buffer();
        let mut sender_id = [0u8; 32];
        let mut recipient_id = [0u8; 32];

        // Outgoing: from alice to bob
        contact_request_get_sender_id(outgoing_handle, sender_id.as_mut_ptr());
        contact_request_get_recipient_id(outgoing_handle, recipient_id.as_mut_ptr());
        assert_eq!(sender_id, alice_id_bytes);
        assert_eq!(recipient_id, bob_id_bytes);

        // Incoming: from bob to alice
        contact_request_get_sender_id(incoming_handle, sender_id.as_mut_ptr());
        contact_request_get_recipient_id(incoming_handle, recipient_id.as_mut_ptr());
        assert_eq!(sender_id, bob_id_bytes);
        assert_eq!(recipient_id, alice_id_bytes);

        // Cleanup
        contact_request_destroy(outgoing_handle);
        contact_request_destroy(incoming_handle);
        established_contact_destroy(contact_handle);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_established_contact_request_fields() {
    unsafe {
        use dpp::identity::accessors::IdentityGettersV0;

        let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

        let bob_id = test_data::identities::bob().identity.id();
        let bob_id_bytes: [u8; 32] = bob_id.to_buffer();

        let mut contact_handle: Handle = NULL_HANDLE;

        managed_identity_get_established_contact(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut contact_handle,
        );

        // Get outgoing request and verify all fields
        let mut outgoing_handle: Handle = NULL_HANDLE;
        established_contact_get_outgoing_request(contact_handle, &mut outgoing_handle);

        let mut sender_key_idx: u32 = 0;
        let mut recipient_key_idx: u32 = 0;
        let mut account_ref: u32 = 0;
        let mut created_at: u64 = 0;

        contact_request_get_sender_key_index(outgoing_handle, &mut sender_key_idx);
        contact_request_get_recipient_key_index(outgoing_handle, &mut recipient_key_idx);
        contact_request_get_account_reference(outgoing_handle, &mut account_ref);
        contact_request_get_created_at(outgoing_handle, &mut created_at);

        // The test data should have specific values
        assert_eq!(sender_key_idx, 0);
        assert_eq!(recipient_key_idx, 1);
        assert_eq!(account_ref, 0);
        assert!(created_at > 0);

        // Get encrypted public key
        let mut bytes_ptr: *mut std::os::raw::c_uchar = std::ptr::null_mut();
        let mut len: usize = 0;
        let result =
            contact_request_get_encrypted_public_key(outgoing_handle, &mut bytes_ptr, &mut len);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(len, 96); // Standard encrypted key length
        assert!(!bytes_ptr.is_null());

        // Cleanup
        platform_wallet_bytes_free(bytes_ptr, len);
        contact_request_destroy(outgoing_handle);
        established_contact_destroy(contact_handle);
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_get_nonexistent_established_contact() {
    unsafe {
        let alice = test_data::identities::alice();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice);

        let nonexistent_id: [u8; 32] = [99u8; 32];

        let mut contact_handle: Handle = NULL_HANDLE;

        let result = managed_identity_get_established_contact(
            alice_handle,
            nonexistent_id.as_ptr(),
            &mut contact_handle,
        );

        // The contact lookup unwraps an `Option` via the macro, so the
        // miss surfaces as `NotFound`.
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(contact_handle, NULL_HANDLE);

        // Cleanup
        managed_identity_destroy(alice_handle);
    }
}

#[test]
fn test_established_contact_destroy_invalid_handle() {
    unsafe {
        // `established_contact_destroy` runs `unwrap_option_or_return!`
        // on the storage remove, so a bogus handle yields `NotFound`.
        let result = established_contact_destroy(9999);
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
    }
}

#[test]
fn test_multiple_established_contacts() {
    unsafe {
        use dpp::identity::accessors::IdentityGettersV0;

        let (alice, _contacts) = test_data::scenarios::alice_with_established_contacts();
        let alice_handle = MANAGED_IDENTITY_STORAGE.insert(alice.clone());

        let bob_id = test_data::identities::bob().identity.id();
        let carol_id = test_data::identities::carol().identity.id();
        let bob_id_bytes: [u8; 32] = bob_id.to_buffer();
        let carol_id_bytes: [u8; 32] = carol_id.to_buffer();

        // Get Bob contact
        let mut bob_contact_handle: Handle = NULL_HANDLE;
        let result = managed_identity_get_established_contact(
            alice_handle,
            bob_id_bytes.as_ptr(),
            &mut bob_contact_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Get Carol contact
        let mut carol_contact_handle: Handle = NULL_HANDLE;
        let result = managed_identity_get_established_contact(
            alice_handle,
            carol_id_bytes.as_ptr(),
            &mut carol_contact_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Verify Bob's contact ID
        let mut retrieved_bob_id = [0u8; 32];
        established_contact_get_contact_id(bob_contact_handle, retrieved_bob_id.as_mut_ptr());
        assert_eq!(retrieved_bob_id, bob_id_bytes);

        // Verify Carol's contact ID
        let mut retrieved_carol_id = [0u8; 32];
        established_contact_get_contact_id(carol_contact_handle, retrieved_carol_id.as_mut_ptr());
        assert_eq!(retrieved_carol_id, carol_id_bytes);

        // Cleanup
        established_contact_destroy(bob_contact_handle);
        established_contact_destroy(carol_contact_handle);
        managed_identity_destroy(alice_handle);
    }
}
