use dpp::identity::accessors::IdentityGettersV0;
use platform_wallet_ffi::*;
use std::ffi::CString;

#[test]
fn test_library_init_and_version() {
    platform_wallet_ffi_init();

    let version = platform_wallet_ffi_version();
    assert!(!version.is_null());

    let version_str = unsafe { std::ffi::CStr::from_ptr(version).to_str().unwrap() };
    assert!(!version_str.is_empty());
}

#[test]
fn test_wallet_creation_and_destruction() {
    unsafe {
        let seed = [0u8; 64];
        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        let result = platform_wallet_info_create_from_seed(
            Network::Testnet,
            seed.as_ptr(),
            seed.len(),
            &mut handle,
            &mut error,
        );

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(handle, NULL_HANDLE);

        let result = platform_wallet_info_destroy(handle);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Double destroy should fail
        let result = platform_wallet_info_destroy(handle);
        assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
    }
}

#[test]
fn test_wallet_from_mnemonic() {
    unsafe {
        let mnemonic = CString::new(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    ).unwrap();

        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        let result = platform_wallet_info_create_from_mnemonic(
            Network::Testnet,
            mnemonic.as_ptr(),
            std::ptr::null(),
            &mut handle,
            &mut error,
        );

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(handle, NULL_HANDLE);

        platform_wallet_info_destroy(handle);
    }
}

#[test]
#[ignore] // Stubbed - requires PlatformWalletInfo
fn test_identity_manager_workflow() {
    unsafe {
        // Create identity manager
        let mut manager_handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        let result = identity_manager_create(&mut manager_handle, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Check initial count
        let mut count: usize = 0;
        let result = identity_manager_get_identity_count(manager_handle, &mut count, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(count, 0);

        // Create a mock identity for testing
        let identity = dpp::tests::fixtures::get_identity_fixture(0).unwrap();
        let identity_id = identity.id();
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let result = identity_manager_add_identity(manager_handle, identity_handle, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Check count increased
        let result = identity_manager_get_identity_count(manager_handle, &mut count, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(count, 1);

        // Set as primary
        let id_bytes: IdentifierBytes = identity_id.into();
        let result = identity_manager_set_primary_identity(manager_handle, id_bytes, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Get primary
        let mut retrieved_id = IdentifierBytes { bytes: [0u8; 32] };
        let result =
            identity_manager_get_primary_identity_id(manager_handle, &mut retrieved_id, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(retrieved_id.bytes, id_bytes.bytes);

        // Get all identity IDs
        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = identity_manager_get_all_identity_ids(manager_handle, &mut array, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(array.count, 1);

        platform_wallet_identifier_array_free(array);

        // Cleanup
        identity_manager_destroy(manager_handle);
    }
}

#[test]
#[ignore] // Stubbed - requires PlatformWalletInfo
fn test_managed_identity_operations() {
    unsafe {
        let identity = dpp::tests::fixtures::get_identity_fixture(0).unwrap();
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let mut error = PlatformWalletFFIError::success();

        // Get ID
        let mut id_bytes = IdentifierBytes { bytes: [0u8; 32] };
        let result = managed_identity_get_id(handle, &mut id_bytes, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Get balance
        let mut balance: u64 = 0;
        let result = managed_identity_get_balance(handle, &mut balance, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Set and get label
        let label = CString::new("Test Identity").unwrap();
        let result = managed_identity_set_label(handle, label.as_ptr(), &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        let mut label_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = managed_identity_get_label(handle, &mut label_ptr, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert!(!label_ptr.is_null());

        let retrieved_label = std::ffi::CStr::from_ptr(label_ptr).to_str().unwrap();
        assert_eq!(retrieved_label, "Test Identity");

        platform_wallet_string_free(label_ptr);

        // Set and get block time
        let block_time = BlockTime {
            height: 100,
            core_height: 200,
            timestamp: 1234567890,
        };

        let result =
            managed_identity_set_last_updated_balance_block_time(handle, block_time, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        let mut retrieved_bt = BlockTime {
            height: 0,
            core_height: 0,
            timestamp: 0,
        };
        let result = managed_identity_get_last_updated_balance_block_time(
            handle,
            &mut retrieved_bt,
            &mut error,
        );
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(retrieved_bt.height, 100);
        assert_eq!(retrieved_bt.core_height, 200);

        // Cleanup
        managed_identity_destroy(handle);
    }
}

#[test]
#[ignore] // TODO: Requires serde support on PlatformWalletInfo
fn test_serialization() {
    unsafe {
        let seed = [0u8; 64];
        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        platform_wallet_info_create_from_seed(
            Network::Testnet,
            seed.as_ptr(),
            seed.len(),
            &mut handle,
            &mut error,
        );

        // Serialize to JSON - function not yet implemented
        // let mut json_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        // let result = platform_wallet_info_to_json(handle, &mut json_ptr, &mut error);
        // assert_eq!(result, PlatformWalletFFIResult::Success);
        // assert!(!json_ptr.is_null());

        // let json_str = unsafe { std::ffi::CStr::from_ptr(json_ptr).to_str().unwrap() };
        // assert!(!json_str.is_empty());
        // assert!(json_str.contains("wallet_info"));

        // platform_wallet_string_free(json_ptr);
        platform_wallet_info_destroy(handle);
    }
}

#[test]
fn test_utils_identifier_operations() {
    unsafe {
        let mut error = PlatformWalletFFIError::success();

        // Generate random identifier
        let mut id1 = IdentifierBytes { bytes: [0u8; 32] };
        let result = platform_wallet_generate_random_identifier(&mut id1, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Convert to hex
        let mut hex: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = platform_wallet_identifier_to_hex(id1, &mut hex, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert!(!hex.is_null());

        // Convert back from hex
        let mut id2 = IdentifierBytes { bytes: [0u8; 32] };
        let result = platform_wallet_identifier_from_hex(hex, &mut id2, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Should match
        assert_eq!(id1.bytes, id2.bytes);

        platform_wallet_string_free(hex);
    }
}

#[test]
fn test_error_handling() {
    unsafe {
        let mut error = PlatformWalletFFIError::success();

        // Try to get identity from invalid handle
        let invalid_handle = 9999;
        let mut id_bytes = IdentifierBytes { bytes: [0u8; 32] };
        let result = managed_identity_get_id(invalid_handle, &mut id_bytes, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);

        // Error should have message
        assert!(!error.message.is_null());
        platform_wallet_ffi_error_free(error);

        // Try to create wallet with null pointer
        let result = platform_wallet_info_create_from_seed(
            Network::Testnet,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(result, PlatformWalletFFIResult::ErrorNullPointer);
    }
}

#[test]
#[ignore] // Stubbed - requires PlatformWalletInfo
fn test_full_workflow() {
    unsafe {
        // Initialize
        platform_wallet_ffi_init();

        let mut error = PlatformWalletFFIError::success();

        // Create wallet from mnemonic
        let mnemonic = CString::new(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    ).unwrap();

        let mut wallet_handle: Handle = NULL_HANDLE;
        let result = platform_wallet_info_create_from_mnemonic(
            Network::Testnet,
            mnemonic.as_ptr(),
            std::ptr::null(),
            &mut wallet_handle,
            &mut error,
        );
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Create identity manager
        let mut manager_handle: Handle = NULL_HANDLE;
        let result = identity_manager_create(&mut manager_handle, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Create identity
        let identity = dpp::tests::fixtures::get_identity_fixture(0).unwrap();
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let identity_id = managed.identity.id();
        let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        // Set identity label
        let label = CString::new("My Primary Identity").unwrap();
        managed_identity_set_label(identity_handle, label.as_ptr(), &mut error);

        // Add identity to manager
        identity_manager_add_identity(manager_handle, identity_handle, &mut error);

        // Set as primary
        let id_bytes: IdentifierBytes = identity_id.into();
        identity_manager_set_primary_identity(manager_handle, id_bytes, &mut error);

        // Set identity manager on wallet
        let result =
            platform_wallet_info_set_identity_manager(wallet_handle, manager_handle, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Get identity manager back
        let mut retrieved_manager_handle: Handle = NULL_HANDLE;
        let result = platform_wallet_info_get_identity_manager(
            wallet_handle,
            &mut retrieved_manager_handle,
            &mut error,
        );
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(retrieved_manager_handle, NULL_HANDLE);

        // Cleanup
        identity_manager_destroy(retrieved_manager_handle);
        identity_manager_destroy(manager_handle);
        platform_wallet_info_destroy(wallet_handle);
    }
}
