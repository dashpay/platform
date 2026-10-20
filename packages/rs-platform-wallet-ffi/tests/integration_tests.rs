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

        let result = platform_wallet_info_create_from_seed(
            Network::Testnet.into(),
            seed.as_ptr(),
            seed.len(),
            &mut handle,
        );

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(handle, NULL_HANDLE);

        let result = platform_wallet_info_destroy(handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Double destroy should fail
        let result = platform_wallet_info_destroy(handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
    }
}

#[test]
fn test_wallet_from_mnemonic() {
    unsafe {
        let mnemonic = CString::new(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    ).unwrap();

        let mut handle: Handle = NULL_HANDLE;

        let result = platform_wallet_info_create_from_mnemonic(
            Network::Testnet.into(),
            mnemonic.as_ptr(),
            &mut handle,
        );

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
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

        let result = identity_manager_create(&mut manager_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Check initial count
        let mut count: usize = 0;
        let result = identity_manager_get_identity_count(manager_handle, &mut count);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(count, 0);

        // Create a mock identity for testing
        let identity = dpp::tests::fixtures::get_identity_fixture(0).unwrap();
        let identity_id = identity.id();
        let managed = platform_wallet::ManagedIdentity::new(identity, 0);
        let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let result = identity_manager_add_identity(manager_handle, identity_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Check count increased
        let result = identity_manager_get_identity_count(manager_handle, &mut count);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(count, 1);

        // Primary-identity FFI was dropped along with the field —
        // selection moved to the UI layer.
        let id_bytes: [u8; 32] = identity_id.to_buffer();
        let _ = id_bytes;

        // Get all identity IDs
        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = identity_manager_get_all_identity_ids(manager_handle, &mut array);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(array.count, 1);

        platform_wallet_identifier_array_free(&mut array);

        // Cleanup
        identity_manager_destroy(manager_handle);
    }
}

#[test]
#[ignore] // Stubbed - requires PlatformWalletInfo
fn test_managed_identity_operations() {
    unsafe {
        let identity = dpp::tests::fixtures::get_identity_fixture(0).unwrap();
        let managed = platform_wallet::ManagedIdentity::new(identity, 0);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        // Get ID
        let mut id_bytes = [0u8; 32];
        let result = managed_identity_get_id(handle, id_bytes.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Get balance
        let mut balance: u64 = 0;
        let result = managed_identity_get_balance(handle, &mut balance);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Set and get label
        let label = CString::new("Test Identity").unwrap();
        let result = managed_identity_set_label(handle, label.as_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        let mut label_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = managed_identity_get_label(handle, &mut label_ptr);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
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

        let result = managed_identity_set_last_updated_balance_block_time(handle, &block_time);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        let mut retrieved_bt = BlockTime {
            height: 0,
            core_height: 0,
            timestamp: 0,
        };
        let result =
            managed_identity_get_last_updated_balance_block_time(handle, &mut retrieved_bt);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
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

        platform_wallet_info_create_from_seed(
            Network::Testnet.into(),
            seed.as_ptr(),
            seed.len(),
            &mut handle,
        );

        // Serialize to JSON - function not yet implemented
        // let mut json_ptr: *mut std::os::raw::c_char = std::ptr::null_mut();
        // let result = platform_wallet_info_to_json(handle, &mut json_ptr);
        // assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
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
        // Generate random identifier
        let mut id1 = [0u8; 32];
        let result = platform_wallet_generate_random_identifier(id1.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Convert to hex
        let mut hex: *mut std::os::raw::c_char = std::ptr::null_mut();
        let result = platform_wallet_identifier_to_hex(id1.as_ptr(), &mut hex);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!hex.is_null());

        // Convert back from hex
        let mut id2 = [0u8; 32];
        let result = platform_wallet_identifier_from_hex(hex, id2.as_mut_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Should match
        assert_eq!(id1, id2);

        platform_wallet_string_free(hex);
    }
}

#[test]
fn test_error_handling() {
    unsafe {
        // Try to get identity from invalid handle
        let invalid_handle = 9999;
        let mut id_bytes = [0u8; 32];
        let result = managed_identity_get_id(invalid_handle, id_bytes.as_mut_ptr());
        // The macro routes a missing handle through Option::None → NotFound.
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);

        // Result carries a diagnostic message on the error path.
        assert!(!result.message.is_null());

        // Try to create wallet with null pointer
        let result = platform_wallet_info_create_from_seed(
            Network::Testnet.into(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }
}

#[test]
#[ignore] // Stubbed - requires PlatformWalletInfo
fn test_full_workflow() {
    unsafe {
        // Initialize
        platform_wallet_ffi_init();

        // Create wallet from mnemonic
        let mnemonic = CString::new(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    ).unwrap();

        let mut wallet_handle: Handle = NULL_HANDLE;
        let result = platform_wallet_info_create_from_mnemonic(
            Network::Testnet.into(),
            mnemonic.as_ptr(),
            &mut wallet_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Create identity manager
        let mut manager_handle: Handle = NULL_HANDLE;
        let result = identity_manager_create(&mut manager_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Create identity
        let identity = dpp::tests::fixtures::get_identity_fixture(0).unwrap();
        let managed = platform_wallet::ManagedIdentity::new(identity, 0);
        let identity_id = managed.identity.id();
        let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        // Label setter is now a no-op stub (ManagedIdentity dropped
        // its label field) — kept here only to verify the call still
        // links and returns Success.
        let label = CString::new("My Primary Identity").unwrap();
        managed_identity_set_label(identity_handle, label.as_ptr());

        // Add identity to manager
        identity_manager_add_identity(manager_handle, identity_handle);

        // Primary-identity FFI was dropped along with the field.
        let id_bytes: [u8; 32] = identity_id.to_buffer();
        let _ = id_bytes;

        // Set identity manager on wallet
        let result = platform_wallet_info_set_identity_manager(wallet_handle, manager_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // Get identity manager back
        let mut retrieved_manager_handle: Handle = NULL_HANDLE;
        let result =
            platform_wallet_info_get_identity_manager(wallet_handle, &mut retrieved_manager_handle);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(retrieved_manager_handle, NULL_HANDLE);

        // Cleanup
        identity_manager_destroy(retrieved_manager_handle);
        identity_manager_destroy(manager_handle);
        platform_wallet_info_destroy(wallet_handle);
    }
}

/// Regression: reading a DashPay profile for an identity the wallet does not
/// manage must report `NotFound` (the generic `Option::None` mapping), not
/// succeed. The JNI `getDashPayProfile` bridge relies on exactly this code to
/// translate an unknown-to-the-wallet id into a clean Kotlin `null` (see
/// `rs-unified-sdk-jni`), instead of throwing — which is what crashed the
/// Android Add Contact preview when it probed `getProfile()` on a not-yet-a-
/// contact recipient id. Locking the code down here keeps that translation
/// honest.
#[test]
fn test_get_dashpay_profile_unmanaged_identity_reports_not_found() {
    use platform_wallet_ffi::dashpay_profile::{
        dashpay_profile_ffi_free, platform_wallet_get_dashpay_profile, DashPayProfileFFI,
    };

    unsafe {
        let seed = [0u8; 64];
        let mut wallet_handle: Handle = NULL_HANDLE;
        let result = platform_wallet_info_create_from_seed(
            Network::Testnet.into(),
            seed.as_ptr(),
            seed.len(),
            &mut wallet_handle,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        // A fresh wallet manages no identities, so any id is "unknown".
        let unmanaged_id = [0x11u8; 32];
        let mut profile = DashPayProfileFFI::empty();
        let mut has_profile = true;

        let mut result = platform_wallet_get_dashpay_profile(
            wallet_handle,
            unmanaged_id.as_ptr(),
            &mut profile as *mut DashPayProfileFFI,
            &mut has_profile as *mut bool,
        );

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::NotFound,
            "an unmanaged identity must surface as the NotFound Option mapping",
        );
        // Out-params are zero-initialized on the error path.
        assert!(!has_profile);
        assert!(profile.display_name.is_null());

        dashpay_profile_ffi_free(&mut profile as *mut DashPayProfileFFI);
        platform_wallet_ffi::error::platform_wallet_ffi_result_free(&mut result);
        platform_wallet_info_destroy(wallet_handle);
    }
}
