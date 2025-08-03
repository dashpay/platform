//! Identity credit transfer operations

use dash_sdk::dpp::platform_value::string_encoding::Encoding;
use dash_sdk::dpp::prelude::{Identifier, Identity};
use dash_sdk::platform::IdentityPublicKey;
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{DashSDKPutSettings, IdentityHandle, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError, IOSSigner};

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
/// - `identity_public_key_handle`: Public key for signing (optional, pass null to auto-select TRANSFER key)
/// - `signer_handle`: Cryptographic signer
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// DashSDKTransferCreditsResult with sender and receiver final balances on success
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_transfer_credits(
    sdk_handle: *mut SDKHandle,
    from_identity_handle: *const IdentityHandle,
    to_identity_id: *const c_char,
    amount: u64,
    identity_public_key_handle: *const crate::types::IdentityPublicKeyHandle,
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

    let wrapper = &mut *(sdk_handle as *mut SDKWrapper);
    let from_identity = &*(from_identity_handle as *const Identity);
    let signer = &*(signer_handle as *const IOSSigner);

    let to_identity_id_str = match CStr::from_ptr(to_identity_id).to_str() {
        Ok(s) => s,
        Err(e) => return DashSDKResult::error(FFIError::from(e).into()),
    };

    let to_id = match Identifier::from_string(to_identity_id_str, Encoding::Base58) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid to_identity_id: {}", e),
            ))
        }
    };

    // Optional public key for signing
    let signing_key = if identity_public_key_handle.is_null() {
        None
    } else {
        Some(&*(identity_public_key_handle as *const IdentityPublicKey))
    };

    let result: Result<DashSDKTransferCreditsResult, FFIError> = wrapper.runtime.block_on(async {
        // Convert settings
        let settings = convert_put_settings(put_settings);

        // Use TransferToIdentity trait to transfer credits
        use dash_sdk::platform::transition::transfer::TransferToIdentity;

        let (sender_balance, receiver_balance) = from_identity
            .transfer_credits(&wrapper.sdk, to_id, amount, signing_key, *signer, settings)
            .await
            .map_err(|e| FFIError::InternalError(format!("Failed to transfer credits: {}", e)))?;

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
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_transfer_credits_result_free(
    result: *mut DashSDKTransferCreditsResult,
) {
    if !result.is_null() {
        let _ = Box::from_raw(result);
    }
}
