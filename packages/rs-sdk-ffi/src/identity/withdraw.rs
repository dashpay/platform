//! Identity withdrawal operations
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

use dash_sdk::dpp::dashcore::{self, Address};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::prelude::Identity;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use tracing::{debug, error, info, warn};

/// Withdraw credits from identity to a Dash address
///
/// # Parameters
/// - `identity_handle`: Identity to withdraw credits from
/// - `address`: Base58-encoded Dash address to withdraw to
/// - `amount`: Amount of credits to withdraw
/// - `core_fee_per_byte`: Core fee per byte (optional, pass 0 for default)
/// - `public_key_id`: ID of the public key to use for signing (pass 0 to auto-select TRANSFER key)
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// The new balance of the identity after withdrawal
///
/// # Safety
/// - `sdk_handle`, `identity_handle`, `address`, and `signer_handle` must be valid, non-null pointers.
/// - `address` must point to a NUL-terminated C string valid for the duration of the call.
/// - `put_settings` may be null; if non-null it must be valid for the duration of the call.
/// - On success, returns a C string pointer inside `DashSDKResult`; caller must free it using SDK routines.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_withdraw(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    address: *const c_char,
    amount: u64,
    core_fee_per_byte: u32,
    public_key_id: u32,
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || address.is_null()
        || signer_handle.is_null()
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    debug!(ptr = ?sdk_handle, "dash_sdk_identity_withdraw: validating handles");
    debug!(ptr = ?identity_handle, "dash_sdk_identity_withdraw: identity_handle");
    debug!(ptr = ?address, "dash_sdk_identity_withdraw: address ptr");
    debug!(ptr = ?signer_handle, "dash_sdk_identity_withdraw: signer_handle");
    debug!(
        amount,
        core_fee_per_byte, public_key_id, "dash_sdk_identity_withdraw: parameters"
    );

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);

    // SAFETY: Null check was performed above. Caller must guarantee the pointer is valid
    // and points to a live Identity. We cannot detect dangling pointers without a handle
    // registry; null checks are the only sound validation available.
    let identity = &*(identity_handle as *const Identity);
    let signer =
        crate::signer::VTableSignerRef(&*(signer_handle as *const crate::signer::VTableSigner));

    debug!("dash_sdk_identity_withdraw: handles dereferenced successfully");
    debug!(id = ?identity.id(), balance = identity.balance(), keys = identity.public_keys().len(), "dash_sdk_identity_withdraw: identity summary");

    let address_str = match CStr::from_ptr(address).to_str() {
        Ok(s) => {
            debug!(
                address = s,
                len = s.len(),
                "dash_sdk_identity_withdraw: address"
            );
            s
        }
        Err(e) => {
            error!(error = %e, "dash_sdk_identity_withdraw: failed to convert address C string");
            return DashSDKResult::error(FFIError::from(e).into());
        }
    };

    // Parse the address
    debug!("dash_sdk_identity_withdraw: parsing Dash address");
    let withdraw_address =
        match Address::<dashcore::address::NetworkUnchecked>::from_str(address_str) {
            Ok(addr) => {
                debug!("dash_sdk_identity_withdraw: address parsed successfully");
                addr.assume_checked()
            }
            Err(e) => {
                error!(error = %e, "dash_sdk_identity_withdraw: failed to parse address");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid Dash address: {}", e),
                ));
            }
        };

    // Get public key if specified (0 means auto-select TRANSFER key)
    debug!("dash_sdk_identity_withdraw: determining signing key");
    let signing_key = if public_key_id == 0 {
        debug!("dash_sdk_identity_withdraw: auto-select key (public_key_id = 0)");
        None
    } else {
        debug!(
            public_key_id,
            "dash_sdk_identity_withdraw: looking for key id"
        );
        match identity.get_public_key_by_id(public_key_id) {
            Some(key) => {
                debug!(found_key_id = public_key_id, purpose = ?key.purpose(), key_type = ?key.key_type(), "dash_sdk_identity_withdraw: found key");
                Some(key)
            }
            None => {
                error!(
                    public_key_id,
                    "dash_sdk_identity_withdraw: key id not found"
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Public key with ID {} not found in identity", public_key_id),
                ));
            }
        }
    };
    debug!("dash_sdk_identity_withdraw: signing key determined");

    // Optional core fee per byte
    let core_fee = if core_fee_per_byte > 0 {
        Some(core_fee_per_byte)
    } else {
        None
    };

    debug!("dash_sdk_identity_withdraw: entering async block");

    // Check for transfer keys before proceeding
    debug!("dash_sdk_identity_withdraw: iterating public keys");
    let mut transfer_key_found = false;
    for (key_id, key) in identity.public_keys() {
        debug!(key_id, purpose = ?key.purpose(), key_type = ?key.key_type(), "dash_sdk_identity_withdraw: found key");
        if key.purpose() == dash_sdk::dpp::identity::Purpose::TRANSFER {
            transfer_key_found = true;
            debug!(key_id, "dash_sdk_identity_withdraw: found TRANSFER key");
        }
    }

    if !transfer_key_found && signing_key.is_none() {
        warn!("dash_sdk_identity_withdraw: no TRANSFER key found and no signing key specified");
    }

    let result: Result<u64, FFIError> = wrapper.runtime.block_on(async {
        debug!("dash_sdk_identity_withdraw: inside async block");

        // Convert settings
        debug!("dash_sdk_identity_withdraw: converting put settings");
        let settings = convert_put_settings(put_settings);
        debug!(has_settings = settings.is_some(), "dash_sdk_identity_withdraw: settings converted");

        // Use Withdraw trait to withdraw credits
        debug!("dash_sdk_identity_withdraw: importing WithdrawFromIdentity trait");
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;
        debug!("dash_sdk_identity_withdraw: trait imported");

        debug!(?withdraw_address, amount, ?core_fee, has_signing_key = signing_key.is_some(), signer_ptr = ?(signer.0 as *const _), "dash_sdk_identity_withdraw: calling withdraw method");

        // Log signing key details for diagnostics
        if let Some(key) = signing_key {
            debug!(
                key_id = key.id(),
                purpose = ?key.purpose(),
                security_level = ?key.security_level(),
                key_type = ?key.key_type(),
                read_only = key.read_only(),
                key_data_len = key.data().len(),
                "dash_sdk_identity_withdraw: signing key details"
            );
        }

        debug!("dash_sdk_identity_withdraw: calling SDK withdraw");

        let new_balance = identity
            .withdraw(
                &wrapper.sdk,
                Some(withdraw_address),
                amount,
                core_fee,
                signing_key,
                signer,
                settings,
            )
            .await
            .map_err(|e| {
                error!(error = %e, "dash_sdk_identity_withdraw: withdraw failed");
                FFIError::InternalError(format!("Failed to withdraw credits: {}", e))
            })?;

        info!(new_balance, "dash_sdk_identity_withdraw: withdrawal successful");

        Ok(new_balance)
    });

    match result {
        Ok(new_balance) => {
            // Return the new balance as a string
            let balance_str = match CString::new(new_balance.to_string()) {
                Ok(s) => s,
                Err(e) => {
                    return DashSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            DashSDKResult::success_string(balance_str.into_raw())
        }
        Err(e) => DashSDKResult::error(e.into()),
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
    #[test]
    fn null_identity_handle_returns_error() {
        let sdk_handle = create_mock_sdk_handle();
        let address = create_c_string("yR9kXCN3fVjjMEB2R4F4kCU6GM93pBvVpz");
        let signer = create_mock_signer();
        let signer_ptr = Box::into_raw(signer) as *const crate::types::SignerHandle;

        let result = unsafe {
            dash_sdk_identity_withdraw(
                sdk_handle,
                std::ptr::null(), // null identity handle
                address,
                1000,
                0,
                0,
                signer_ptr,
                std::ptr::null(),
            )
        };

        assert!(
            !result.error.is_null(),
            "Expected error for null identity handle"
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
            let _ = CString::from_raw(address as *mut _);
            destroy_mock_sdk_handle(sdk_handle);
        }
    }

    /// Verify that passing a null SDK handle returns an error instead of crashing.
    #[test]
    fn null_sdk_handle_returns_error() {
        let address = create_c_string("yR9kXCN3fVjjMEB2R4F4kCU6GM93pBvVpz");
        let signer = create_mock_signer();
        let signer_ptr = Box::into_raw(signer) as *const crate::types::SignerHandle;

        let result = unsafe {
            dash_sdk_identity_withdraw(
                std::ptr::null_mut(), // null SDK handle
                std::ptr::dangling::<IdentityHandle>(),
                address,
                1000,
                0,
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
            let _ = CString::from_raw(address as *mut _);
        }
    }
}
