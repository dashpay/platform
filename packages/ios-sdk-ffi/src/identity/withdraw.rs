//! Identity withdrawal operations

use dash_sdk::dpp::dashcore::{self, Address};
use dash_sdk::dpp::prelude::Identity;
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{IOSSDKPutSettings, IdentityHandle, SDKHandle};
use crate::{FFIError, IOSSDKError, IOSSDKErrorCode, IOSSDKResult, IOSSigner};

/// Withdraw credits from identity to a Dash address
///
/// # Parameters
/// - `identity_handle`: Identity to withdraw credits from
/// - `address`: Base58-encoded Dash address to withdraw to
/// - `amount`: Amount of credits to withdraw
/// - `core_fee_per_byte`: Core fee per byte (optional, pass 0 for default)
/// - `identity_public_key_handle`: Public key for signing (optional, pass null to auto-select)
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// The new balance of the identity after withdrawal
#[no_mangle]
pub unsafe extern "C" fn ios_sdk_identity_withdraw(
    sdk_handle: *mut SDKHandle,
    identity_handle: *const IdentityHandle,
    address: *const c_char,
    amount: u64,
    core_fee_per_byte: u32,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
    signer_handle: *const crate::types::SignerHandle,
    put_settings: *const IOSSDKPutSettings,
) -> IOSSDKResult {
    // Validate parameters
    if sdk_handle.is_null()
        || identity_handle.is_null()
        || address.is_null()
        || signer_handle.is_null()
    {
        return IOSSDKResult::error(IOSSDKError::new(
            IOSSDKErrorCode::InvalidParameter,
            "One or more required parameters is null".to_string(),
        ));
    }

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const IOSSigner);

    let address_str = match CStr::from_ptr(address).to_str() {
        Ok(s) => s,
        Err(e) => return IOSSDKResult::error(FFIError::from(e).into()),
    };

    // Parse the address
    let withdraw_address =
        match Address::<dashcore::address::NetworkUnchecked>::from_str(address_str) {
            Ok(addr) => addr.assume_checked(),
            Err(e) => {
                return IOSSDKResult::error(IOSSDKError::new(
                    IOSSDKErrorCode::InvalidParameter,
                    format!("Invalid Dash address: {}", e),
                ))
            }
        };

    // Optional public key for signing
    let signing_key = if identity_public_key_handle.is_null() {
        None
    } else {
        Some(&*(identity_public_key_handle as *const IdentityPublicKey))
    };

    // Optional core fee per byte
    let core_fee = if core_fee_per_byte > 0 {
        Some(core_fee_per_byte)
    } else {
        None
    };

    let result: Result<u64, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = convert_put_settings(put_settings);

        // Use Withdraw trait to withdraw credits
        use dash_sdk::platform::transition::withdraw_from_identity::WithdrawFromIdentity;

        let new_balance = identity
            .withdraw(
                &wrapper.sdk,
                Some(withdraw_address),
                amount,
                core_fee,
                signing_key,
                *signer,
                settings,
            )
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to withdraw credits: {}", e)))?;

        Ok(new_balance)
    });

    match result {
        Ok(new_balance) => {
            // Return the new balance as a string
            let balance_str = match CString::new(new_balance.to_string()) {
                Ok(s) => s,
                Err(e) => {
                    return IOSSDKResult::error(
                        FFIError::InternalError(format!("Failed to create CString: {}", e)).into(),
                    )
                }
            };
            IOSSDKResult::success_string(balance_str.into_raw())
        }
        Err(e) => IOSSDKResult::error(e.into()),
    }
}
