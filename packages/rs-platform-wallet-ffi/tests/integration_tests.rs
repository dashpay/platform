use platform_wallet_ffi::*;
use std::ffi::CString;

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
        // A handle the wallet storage does not know; the profile read resolves
        // through `PLATFORM_WALLET_STORAGE`, so this is the same `Option::None`
        // arm an unmanaged identity takes.
        let wallet_handle: Handle = 9999;

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
    }
}
