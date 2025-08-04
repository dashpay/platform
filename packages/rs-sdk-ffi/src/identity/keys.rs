//! Identity key selection operations

use crate::types::{IdentityHandle, IdentityPublicKeyHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use dash_sdk::dpp::prelude::Identity;

/// State transition type for key selection
#[repr(C)]
pub enum StateTransitionType {
    IdentityUpdate = 0,
    IdentityTopUp = 1,
    IdentityCreditTransfer = 2,
    IdentityCreditWithdrawal = 3,
    DocumentsBatch = 4,
    DataContractCreate = 5,
    DataContractUpdate = 6,
}

/// Get the appropriate signing key for a state transition
///
/// This function finds a key that meets the purpose and security level requirements
/// for the specified state transition type.
///
/// # Parameters
/// - `identity_handle`: Handle to the identity
/// - `transition_type`: Type of state transition to be signed
///
/// # Returns
/// - Handle to the identity public key on success
/// - Error if no suitable key is found
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_get_signing_key_for_transition(
    identity_handle: *const IdentityHandle,
    transition_type: StateTransitionType,
) -> DashSDKResult {
    if identity_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity handle is null".to_string(),
        ));
    }

    let identity = &*(identity_handle as *const Identity);

    // Determine purpose and security level requirements based on transition type
    let (required_purposes, required_security_levels) = match transition_type {
        StateTransitionType::IdentityCreditTransfer
        | StateTransitionType::IdentityCreditWithdrawal => {
            // Transfer and withdrawal require TRANSFER purpose at CRITICAL level
            (vec![Purpose::TRANSFER], vec![SecurityLevel::CRITICAL])
        }
        _ => {
            // All other transitions use AUTHENTICATION purpose
            // and can use HIGH or CRITICAL security levels
            (
                vec![Purpose::AUTHENTICATION],
                vec![SecurityLevel::HIGH, SecurityLevel::CRITICAL],
            )
        }
    };

    // Search for keys matching the requirements, preferring lower security levels
    for security_level in required_security_levels.iter() {
        for purpose in required_purposes.iter() {
            let matching_keys: Vec<&IdentityPublicKey> = identity
                .public_keys()
                .values()
                .filter(|key| {
                    key.purpose() == *purpose
                        && key.security_level() == *security_level
                        && key.disabled_at().is_none() // Only consider enabled keys
                })
                .collect();

            if !matching_keys.is_empty() {
                // Return the first matching key found
                let key = matching_keys[0].clone();
                let handle = Box::into_raw(Box::new(key)) as *mut IdentityPublicKeyHandle;
                return DashSDKResult::success(handle as *mut std::os::raw::c_void);
            }
        }
    }

    // If no suitable key found, return error
    let error_msg = match transition_type {
        StateTransitionType::IdentityCreditTransfer
        | StateTransitionType::IdentityCreditWithdrawal => {
            "No TRANSFER key found at CRITICAL security level".to_string()
        }
        _ => "No AUTHENTICATION key found at HIGH or CRITICAL security level".to_string(),
    };

    DashSDKResult::error(DashSDKError::new(DashSDKErrorCode::NotFound, error_msg))
}

/// Get the private key data for a transfer key
///
/// This function retrieves the private key data that corresponds to the
/// lowest security level transfer key. In a real implementation, this would
/// interface with a secure key storage system.
///
/// # Parameters
/// - `identity_handle`: Handle to the identity
/// - `key_index`: The key index from the identity public key
///
/// # Returns
/// - 32-byte private key data on success
/// - Error if key not found or not accessible
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_get_transfer_private_key(
    identity_handle: *const IdentityHandle,
    key_index: u32,
) -> DashSDKResult {
    // TODO: This is a placeholder implementation
    // In a real implementation, this would:
    // 1. Verify the caller has access to the private keys
    // 2. Retrieve the private key from secure storage (keychain, hardware wallet, etc.)
    // 3. Return the private key data

    DashSDKResult::error(DashSDKError::new(
        DashSDKErrorCode::NotImplemented,
        "Private key retrieval not implemented. Keys should be managed by the wallet layer."
            .to_string(),
    ))
}

/// Get the key ID from an identity public key
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_public_key_get_id(
    key_handle: *const IdentityPublicKeyHandle,
) -> u32 {
    if key_handle.is_null() {
        return 0;
    }

    let key = &*(key_handle as *const IdentityPublicKey);
    key.id().into()
}

/// Free an identity public key handle
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_public_key_destroy(
    handle: *mut IdentityPublicKeyHandle,
) {
    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut IdentityPublicKey);
    }
}
