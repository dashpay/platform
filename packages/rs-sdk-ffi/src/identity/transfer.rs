//! Identity credit transfer operations
//!
//! # Safety note on pointer validation
//!
//! Previous versions of this code used `std::panic::catch_unwind` around raw pointer
//! dereferences in an attempt to detect invalid (dangling) pointers. This was removed
//! because:
//!
//! 1. Dereferencing an invalid pointer is **undefined behavior** in Rust, regardless of
//!    whether it is wrapped in `catch_unwind`. The UB occurs at the dereference itself,
//!    before any panic could be raised.
//! 2. The release profile sets `panic = "abort"`, which means `catch_unwind` is a
//!    complete no-op in release builds -- the compiler eliminates it entirely.
//! 3. `AssertUnwindSafe` further suppressed any soundness diagnostics.
//!
//! The only sound validation we can perform on a raw pointer without a handle registry
//! is a null check, which is what we do now. Callers are responsible for ensuring that
//! non-null pointers are valid and properly aligned as documented in each function's
//! `# Safety` section.

use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Result structure for credit transfer operations
#[repr(C)]
pub struct DashSDKTransferCreditsResult {
    /// Sender's final balance after transfer
    pub sender_balance: u64,
    /// Receiver's final balance after transfer
    pub receiver_balance: u64,
}

/// Transfer credits from one identity to another
///
/// # Parameters
/// - `from_identity_handle`: Identity to transfer credits from
/// - `to_identity_id`: Base58-encoded ID of the identity to transfer credits to
/// - `amount`: Amount of credits to transfer
/// - `public_key_id`: ID of the public key to use for signing (pass 0 to auto-select TRANSFER key)
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// DashSDKTransferCreditsResult with sender and receiver final balances on success
///
/// # Safety
/// - `sdk_handle`, `from_identity_handle`, `to_identity_id`, and `signer_handle` must be valid, non-null pointers.
/// - `to_identity_id` must point to a NUL-terminated C string valid for the duration of the call.
/// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
/// - On success, any heap memory included in the result must be freed using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_transfer_credits(
    sdk_handle: *mut SDKHandle,
    from_identity_handle: *const IdentityHandle,
    to_identity_id: *const c_char,
    amount: u64,
    public_key_id: u32,
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || from_identity_handle.is_null()
        || to_identity_id.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    eprintln!("🔵 dash_sdk_identity_transfer_credits: Validating handles...");
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: sdk_handle = {:p}",
        sdk_handle
    );
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: from_identity_handle = {:p}",
        from_identity_handle
    );
    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: signer_handle = {:p}",
        signer_handle
    );

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    // SAFETY: Null check was performed above. Caller must guarantee the pointer is valid
    // and points to a live Identity. We cannot detect dangling pointers without a handle
    // registry; null checks are the only sound validation available.
    let from_identity = &*(from_identity_handle as *const Identity);
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    eprintln!(
        "🔵 dash_sdk_identity_transfer_credits: public_key_id = {}",
        public_key_id
    );

    let to_identity_id_str = match CStr::from_ptr(to_identity_id).to_str() {
        Ok(s) => {
            eprintln!(
                "🔵 dash_sdk_identity_transfer_credits: to_identity_id = '{}'",
                s
            );
            eprintln!(
                "🔵 dash_sdk_identity_transfer_credits: to_identity_id length = {}",
                s.len()
            );
            // Debug each character
            for (i, ch) in s.chars().enumerate() {
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: char[{}] = '{}' (U+{:04X})",
                    i, ch, ch as u32
                );
            }
            s
        }
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let to_id = match Identifier::from_string(to_identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            eprintln!(
                "❌ dash_sdk_identity_transfer_credits: Failed to parse to_identity_id: {}",
                e
            );
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid to_identity_id: {}", e),
            ));
        }
    };

    // Get public key if specified (0 means auto-select TRANSFER key)
    eprintln!("🔵 dash_sdk_identity_transfer_credits: Determining signing key...");
    let signing_key = if public_key_id == 0 {
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Using auto-select (public_key_id = 0)");
        None
    } else {
        eprintln!(
            "🔵 dash_sdk_identity_transfer_credits: Looking for key with ID {}",
            public_key_id
        );
        match from_identity.get_public_key_by_id(public_key_id) {
            Some(key) => {
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: Found key with ID {}",
                    public_key_id
                );
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: Key purpose: {:?}",
                    key.purpose()
                );
                eprintln!(
                    "🔵 dash_sdk_identity_transfer_credits: Key type: {:?}",
                    key.key_type()
                );
                Some(key)
            }
            None => {
                eprintln!(
                    "❌ dash_sdk_identity_transfer_credits: Key with ID {} not found!",
                    public_key_id
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Public key with ID {} not found in identity", public_key_id),
                ));
            }
        }
    };
    eprintln!("🔵 dash_sdk_identity_transfer_credits: Signing key determined");

    eprintln!("🔵 dash_sdk_identity_transfer_credits: About to enter async block");

    let result: Result<DashSDKTransferCreditsResult, FFIError> = wrapper.runtime.block_on(async {
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Inside async block");
        // Convert settings
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Converting put settings");
        let settings = convert_put_settings(put_settings);
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Settings converted: {:?}", settings.is_some());

        // Use TransferToIdentity trait to transfer credits
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Importing TransferToIdentity trait");
        use dash_sdk::platform::transition::transfer::TransferToIdentity;
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Trait imported");

        eprintln!("🔵 dash_sdk_identity_transfer_credits: About to call transfer_credits method");
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Parameters:");
        eprintln!("  - to_id: {:?}", to_id);
        eprintln!("  - amount: {}", amount);
        eprintln!("  - signing_key present: {}", signing_key.is_some());
        eprintln!("  - signer: {:p}", signer as *const _);

        // Additional defensive checks before calling transfer_credits
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Performing defensive checks...");

        // Check if we can iterate through public keys
        eprintln!("🔵 dash_sdk_identity_transfer_credits: Iterating through identity public keys...");
        let mut transfer_key_found = false;
        for (key_id, key) in from_identity.public_keys() {
            eprintln!("🔵 dash_sdk_identity_transfer_credits: Found key {}: purpose={:?}", key_id, key.purpose());
            if key.purpose() == dash_sdk::dpp::identity::Purpose::TRANSFER {
                transfer_key_found = true;
                eprintln!("🔵 dash_sdk_identity_transfer_credits: Found TRANSFER key with ID {}", key_id);
            }
        }

        if !transfer_key_found && signing_key.is_none() {
            eprintln!("⚠️ dash_sdk_identity_transfer_credits: WARNING - No transfer key found and no signing key specified!");
        }

        eprintln!("🔵 dash_sdk_identity_transfer_credits: Defensive checks complete");

        // Log signing key details for diagnostics
        if let Some(key) = signing_key {
            eprintln!("🔵 dash_sdk_identity_transfer_credits: Signing key details:");
            eprintln!("  - Key ID: {}", key.id());
            eprintln!("  - Purpose: {:?}", key.purpose());
            eprintln!("  - Security level: {:?}", key.security_level());
            eprintln!("  - Key type: {:?}", key.key_type());
            eprintln!("  - Read only: {}", key.read_only());
            eprintln!("  - Key data length: {} bytes", key.data().len());
        }

        eprintln!("🔵 dash_sdk_identity_transfer_credits: About to call SDK's transfer_credits method");
        eprintln!("🔵 dash_sdk_identity_transfer_credits: This will internally call IdentityCreditTransferTransition::try_from_identity");

        let transfer_result = from_identity
            .transfer_credits(&wrapper.sdk, to_id, amount, signing_key, *signer, settings)
            .await;

        eprintln!("🔵 dash_sdk_identity_transfer_credits: transfer_credits returned: {:?}", transfer_result.is_ok());

        let (sender_balance, receiver_balance) = transfer_result
            .map_err(|e| {
                eprintln!("❌ dash_sdk_identity_transfer_credits: transfer_credits failed: {}", e);
                FFIError::InternalError(format!("Failed to transfer credits: {}", e))
            })?;

        eprintln!("🔵 dash_sdk_identity_transfer_credits: Transfer successful!");
        eprintln!("  - sender_balance: {}", sender_balance);
        eprintln!("  - receiver_balance: {}", receiver_balance);

        Ok(DashSDKTransferCreditsResult {
            sender_balance,
            receiver_balance,
        })
    });

    match result {
        Ok(transfer_result) => {
            let result_ptr = Box::into_raw(Box::new(transfer_result));
            DashSDKResult::success(result_ptr as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free a transfer credits result structure
///
/// # Safety
/// - `result` must be a pointer previously returned by this SDK or null (no-op).
/// - After this call, `result` becomes invalid and must not be used again.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_transfer_credits_result_free(
    result: *mut DashSDKTransferCreditsResult,
) {
    if !result.is_null() {
        let _ = Box::from_raw(result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_utils::{
        create_c_string, create_mock_sdk_handle, create_mock_signer, destroy_mock_sdk_handle,
    };
    use std::ffi::CString;

    /// Verify that passing a null identity handle returns an error instead of crashing.
    ///
    /// This is the core test for the security fix: previous code wrapped the raw pointer
    /// dereference in `catch_unwind`, which was undefined behavior and a no-op under
    /// `panic = "abort"`. The replacement null check must correctly return an error.
    #[test]
    fn null_identity_handle_returns_error() {
        let sdk_handle = create_mock_sdk_handle();
        let to_id = create_c_string("11111111111111111111111111111111");
        let signer = create_mock_signer();
        let signer_ptr = Box::into_raw(signer) as *const crate::types::SignerHandle;

        let result = unsafe {
            dash_sdk_identity_transfer_credits(
                sdk_handle,
                std::ptr::null(), // null identity handle
                to_id,
                1000,
                0,
                signer_ptr,
                std::ptr::null(),
            )
        };

        // Should return an error, not crash
        assert!(
            !result.error.is_null(),
            "Expected error for null identity handle"
        );
        assert!(result.data.is_null(), "Expected null data on error");

        // Verify error code is InvalidParameter
        let error = unsafe { &*result.error };
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

        // Clean up error message
        if !error.message.is_null() {
            let msg = unsafe { CString::from_raw(error.message as *mut _) };
            let msg_str = msg.to_str().expect("valid utf-8");
            assert!(
                msg_str.contains("null"),
                "Error message should mention null, got: {}",
                msg_str
            );
        }

        // Clean up
        unsafe {
            let _ = Box::from_raw(result.error);
            let _ = Box::from_raw(signer_ptr as *mut crate::signer::VTableSigner);
            let _ = CString::from_raw(to_id as *mut _);
            destroy_mock_sdk_handle(sdk_handle);
        }
    }

    /// Verify that passing a null SDK handle returns an error instead of crashing.
    #[test]
    fn null_sdk_handle_returns_error() {
        let to_id = create_c_string("11111111111111111111111111111111");
        let signer = create_mock_signer();
        let signer_ptr = Box::into_raw(signer) as *const crate::types::SignerHandle;

        let result = unsafe {
            dash_sdk_identity_transfer_credits(
                std::ptr::null_mut(),         // null SDK handle
                0x1 as *const IdentityHandle, // non-null but unused due to early return
                to_id,
                1000,
                0,
                signer_ptr,
                std::ptr::null(),
            )
        };

        assert!(
            !result.error.is_null(),
            "Expected error for null SDK handle"
        );
        let error = unsafe { &*result.error };
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

        // Clean up
        unsafe {
            if !error.message.is_null() {
                let _ = CString::from_raw(error.message as *mut _);
            }
            let _ = Box::from_raw(result.error);
            let _ = Box::from_raw(signer_ptr as *mut crate::signer::VTableSigner);
            let _ = CString::from_raw(to_id as *mut _);
        }
    }

    /// Verify that passing a null signer handle returns an error instead of crashing.
    #[test]
    fn null_signer_handle_returns_error() {
        let sdk_handle = create_mock_sdk_handle();
        let to_id = create_c_string("11111111111111111111111111111111");

        let result = unsafe {
            dash_sdk_identity_transfer_credits(
                sdk_handle,
                0x1 as *const IdentityHandle, // non-null but unused due to early return
                to_id,
                1000,
                0,
                std::ptr::null(), // null signer handle
                std::ptr::null(),
            )
        };

        assert!(
            !result.error.is_null(),
            "Expected error for null signer handle"
        );
        let error = unsafe { &*result.error };
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

        // Clean up
        unsafe {
            if !error.message.is_null() {
                let _ = CString::from_raw(error.message as *mut _);
            }
            let _ = Box::from_raw(result.error);
            let _ = CString::from_raw(to_id as *mut _);
            destroy_mock_sdk_handle(sdk_handle);
        }
    }

    /// Verify that passing a null to_identity_id returns an error instead of crashing.
    #[test]
    fn null_to_identity_id_returns_error() {
        let sdk_handle = create_mock_sdk_handle();
        let signer = create_mock_signer();
        let signer_ptr = Box::into_raw(signer) as *const crate::types::SignerHandle;

        let result = unsafe {
            dash_sdk_identity_transfer_credits(
                sdk_handle,
                0x1 as *const IdentityHandle, // non-null but unused due to early return
                std::ptr::null(),             // null to_identity_id
                1000,
                0,
                signer_ptr,
                std::ptr::null(),
            )
        };

        assert!(
            !result.error.is_null(),
            "Expected error for null to_identity_id"
        );
        let error = unsafe { &*result.error };
        assert_eq!(error.code, DashSDKErrorCode::InvalidParameter);

        // Clean up
        unsafe {
            if !error.message.is_null() {
                let _ = CString::from_raw(error.message as *mut _);
            }
            let _ = Box::from_raw(result.error);
            let _ = Box::from_raw(signer_ptr as *mut crate::signer::VTableSigner);
            destroy_mock_sdk_handle(sdk_handle);
        }
    }
}
