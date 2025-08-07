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

/// Create an identity public key handle from key data
///
/// This function creates an identity public key handle from the raw key data
/// without needing to fetch the identity from the network.
///
/// # Parameters
/// - `key_id`: The key ID
/// - `key_type`: The key type (0 = ECDSA_SECP256K1, 1 = BLS12_381, 2 = ECDSA_HASH160, 3 = BIP13_SCRIPT_HASH, 4 = ED25519_HASH160)
/// - `purpose`: The key purpose (0 = Authentication, 1 = Encryption, 2 = Decryption, 3 = Transfer, 4 = SystemTransfer, 5 = Voting)
/// - `security_level`: The security level (0 = Master, 1 = Critical, 2 = High, 3 = Medium)
/// - `public_key_data`: The public key data
/// - `public_key_data_len`: Length of the public key data
/// - `read_only`: Whether the key is read-only
/// - `disabled_at`: Optional timestamp when the key was disabled (0 if not disabled)
///
/// # Returns
/// - Handle to the identity public key on success
/// - Error if parameters are invalid
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_public_key_create_from_data(
    key_id: u32,
    key_type: u8,
    purpose: u8,
    security_level: u8, 
    public_key_data: *const u8,
    public_key_data_len: usize,
    read_only: bool,
    disabled_at: u64,
) -> DashSDKResult {
    use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dash_sdk::dpp::identity::{KeyType, Purpose as DPPPurpose, SecurityLevel as DPPSecurityLevel};
    
    if public_key_data.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Public key data is null".to_string(),
        ));
    }

    // Convert key type
    let key_type = match key_type {
        0 => KeyType::ECDSA_SECP256K1,
        1 => KeyType::BLS12_381,
        2 => KeyType::ECDSA_HASH160,
        3 => KeyType::BIP13_SCRIPT_HASH,
        4 => KeyType::EDDSA_25519_HASH160,
        _ => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid key type: {}", key_type),
            ))
        }
    };

    // Convert purpose
    let purpose = match purpose {
        0 => DPPPurpose::AUTHENTICATION,
        1 => DPPPurpose::ENCRYPTION,
        2 => DPPPurpose::DECRYPTION,
        3 => DPPPurpose::TRANSFER,
        4 => DPPPurpose::SYSTEM,
        5 => DPPPurpose::VOTING,
        _ => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid purpose: {}", purpose),
            ))
        }
    };

    // Convert security level
    let security_level = match security_level {
        0 => DPPSecurityLevel::MASTER,
        1 => DPPSecurityLevel::CRITICAL,
        2 => DPPSecurityLevel::HIGH,
        3 => DPPSecurityLevel::MEDIUM,
        _ => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid security level: {}", security_level),
            ))
        }
    };

    // Copy public key data
    let key_data = std::slice::from_raw_parts(public_key_data, public_key_data_len).to_vec();

    // Create the identity public key
    let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
        id: key_id.into(),
        key_type,
        purpose,
        security_level,
        data: key_data.into(),
        read_only,
        disabled_at: if disabled_at > 0 { Some(disabled_at) } else { None },
        contract_bounds: None,
    });

    let handle = Box::into_raw(Box::new(public_key)) as *mut IdentityPublicKeyHandle;
    DashSDKResult::success(handle as *mut std::os::raw::c_void)
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
