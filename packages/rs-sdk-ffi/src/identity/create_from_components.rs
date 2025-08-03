//! Create identity from components

use dash_sdk::dpp::identity::{IdentityV0, IdentityPublicKey};
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::prelude::{Identity, Identifier};
use std::collections::BTreeMap;
use std::slice;

use crate::types::{DashSDKResultDataType, IdentityHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult};

/// Public key data for creating identity
#[repr(C)]
pub struct DashSDKPublicKeyData {
    /// Key ID (0-255)
    pub id: u8,
    /// Key purpose (0-6)
    pub purpose: u8,
    /// Security level (0-3)
    pub security_level: u8,
    /// Key type (0-4)
    pub key_type: u8,
    /// Whether key is read-only
    pub read_only: bool,
    /// Public key data pointer
    pub data: *const u8,
    /// Public key data length
    pub data_len: usize,
    /// Disabled timestamp (0 if not disabled)
    pub disabled_at: u64,
}

/// Create an identity handle from components
/// 
/// This function creates an identity handle from basic components without
/// requiring JSON serialization/deserialization.
///
/// # Parameters
/// - `identity_id`: 32-byte identity ID
/// - `public_keys`: Array of public key data
/// - `public_keys_count`: Number of public keys in the array
/// - `balance`: Identity balance in credits
/// - `revision`: Identity revision number
///
/// # Returns
/// - Handle to the created identity on success
/// - Error if creation fails
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_create_from_components(
    identity_id: *const u8,
    public_keys: *const DashSDKPublicKeyData,
    public_keys_count: usize,
    balance: u64,
    revision: u64,
) -> DashSDKResult {
    // Validate parameters
    if identity_id.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity ID is null".to_string(),
        ));
    }

    if public_keys_count > 0 && public_keys.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Public keys array is null but count is non-zero".to_string(),
        ));
    }

    // Create identifier from 32-byte array
    let id_bytes = slice::from_raw_parts(identity_id, 32);
    let identifier = match Identifier::from_bytes(id_bytes) {
        Ok(id) => id,
        Err(e) => {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid identity ID: {}", e),
            ));
        }
    };

    // Convert public keys
    let mut keys_map = BTreeMap::new();
    
    if public_keys_count > 0 {
        let keys_slice = slice::from_raw_parts(public_keys, public_keys_count);
        
        for key_data in keys_slice {
            if key_data.data.is_null() {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Public key {} has null data", key_data.id),
                ));
            }

            let key_bytes = slice::from_raw_parts(key_data.data, key_data.data_len);
            
            // Create IdentityPublicKey from the data
            // Note: This is a simplified version. In production, you'd properly
            // construct the key with all fields and proper validation
            use dash_sdk::dpp::identity::{
                Purpose,
                SecurityLevel,
                KeyType,
            };

            let purpose = match key_data.purpose {
                0 => Purpose::AUTHENTICATION,
                1 => Purpose::ENCRYPTION,
                2 => Purpose::DECRYPTION,
                3 => Purpose::TRANSFER,
                4 => Purpose::SYSTEM,
                5 => Purpose::VOTING,
                6 => Purpose::OWNER,
                _ => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid key purpose: {}", key_data.purpose),
                    ));
                }
            };

            let security_level = match key_data.security_level {
                0 => SecurityLevel::MASTER,
                1 => SecurityLevel::CRITICAL,
                2 => SecurityLevel::HIGH,
                3 => SecurityLevel::MEDIUM,
                _ => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid security level: {}", key_data.security_level),
                    ));
                }
            };

            let key_type = match key_data.key_type {
                0 => KeyType::ECDSA_SECP256K1,
                1 => KeyType::BLS12_381,
                2 => KeyType::ECDSA_HASH160,
                3 => KeyType::BIP13_SCRIPT_HASH,
                4 => KeyType::EDDSA_25519_HASH160,
                _ => {
                    return DashSDKResult::error(DashSDKError::new(
                        DashSDKErrorCode::InvalidParameter,
                        format!("Invalid key type: {}", key_data.key_type),
                    ));
                }
            };

            let disabled_at = if key_data.disabled_at == 0 {
                None
            } else {
                Some(key_data.disabled_at)
            };

            let public_key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                id: key_data.id as u32,
                purpose,
                security_level,
                contract_bounds: None, // Not supported in this simple version
                key_type,
                read_only: key_data.read_only,
                data: dash_sdk::dpp::platform_value::BinaryData::new(key_bytes.to_vec()),
                disabled_at,
            });

            keys_map.insert(key_data.id as u32, public_key);
        }
    }

    // Create the identity
    let identity = Identity::V0(IdentityV0 {
        id: identifier,
        public_keys: keys_map,
        balance,
        revision,
    });

    // Return the handle
    let handle = Box::into_raw(Box::new(identity)) as *mut IdentityHandle;
    DashSDKResult::success_handle(
        handle as *mut std::os::raw::c_void,
        DashSDKResultDataType::ResultIdentityHandle,
    )
}