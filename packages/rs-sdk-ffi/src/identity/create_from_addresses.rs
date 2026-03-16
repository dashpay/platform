//! Identity creation from addresses operations
//!
//! Note: `catch_unwind` intentionally not used -- `panic = "abort"` in the release profile
//! makes it a no-op, and dereferencing invalid pointers is UB regardless of `catch_unwind`.

use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::dpp::dashcore::secp256k1::SecretKey;
use dash_sdk::dpp::dashcore::{Network, PrivateKey};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::Identity;
use dash_sdk::dpp::prelude::AddressNonce;
use dash_sdk::platform::transition::put_identity::PutIdentity;
use std::collections::BTreeMap;

use crate::address::transitions::AddressSigner;
use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{
    dash_sdk_address_info_map_free, DashSDKAddressInfoEntry, DashSDKAddressInfoMap,
    DashSDKAddressTransferInput, DashSDKAddressTransferOutput, DashSDKPutSettings,
    DashSDKResultDataType, IdentityHandle, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Result for identity creation from addresses
#[repr(C)]
pub struct DashSDKIdentityCreateFromAddressesResult {
    /// Created identity handle (must be freed separately)
    pub identity_handle: *mut IdentityHandle,
    /// Address info map
    pub address_info_map: DashSDKAddressInfoMap,
}

/// Create an identity funded by Platform addresses
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_handle`: Identity to create (must be prepared with public keys)
/// - `inputs`: Array of input addresses with amounts, nonces, and private keys
/// - `inputs_count`: Number of input entries
/// - `output`: Optional output address with amount (for change)
/// - `identity_signer_handle`: Cryptographic signer for identity
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// DashSDKResult with custom data type containing created identity handle and address infos
///
/// # Safety
/// - All pointers must be valid and non-null (except put_settings and output which can be null).
/// - Arrays must contain at least the specified count of elements.
/// - Private keys must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_create_from_addresses(
    sdk_handle: *const SDKHandle,
    identity_handle: *const IdentityHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
    output: *const DashSDKAddressTransferOutput,
    identity_signer_handle: *const crate::types::SignerHandle,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // Validate parameters
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if identity_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity handle is null".to_string(),
        ));
    }

    if inputs.is_null() || inputs_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Inputs array is null or empty".to_string(),
        ));
    }

    if identity_signer_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Identity signer handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let identity_signer = &*(identity_signer_handle as *const crate::signer::VTableSigner);

    // Parse inputs and create address signer (same as address transfer)
    let mut input_map: BTreeMap<PlatformAddress, (AddressNonce, Credits)> = BTreeMap::new();
    let mut address_signer = AddressSigner::new();

    let inputs_slice = std::slice::from_raw_parts(inputs, inputs_count);
    for (i, input) in inputs_slice.iter().enumerate() {
        if input.address.is_null() || input.address_len == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Input {} has null or empty address", i),
            ));
        }

        if input.private_key.is_null() {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Input {} has null private key", i),
            ));
        }

        let address_bytes = std::slice::from_raw_parts(input.address, input.address_len);
        let address = match PlatformAddress::from_bytes(address_bytes) {
            Ok(addr) => addr,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse input address {}: {}", i, e),
                ))
            }
        };

        // Parse private key (32 bytes)
        let pk_bytes = std::slice::from_raw_parts(input.private_key, 32);
        let secret_key = match SecretKey::from_slice(pk_bytes) {
            Ok(sk) => sk,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse private key for input {}: {}", i, e),
                ))
            }
        };

        // Create PrivateKey (network doesn't matter for signing)
        let private_key = PrivateKey::new(secret_key, Network::Testnet);

        address_signer.add_key(&address, private_key);
        // Use nonce from input, or fetch if 0
        let nonce = if input.nonce == 0 {
            // For now, we'll require nonce to be provided
            // In a full implementation, we'd fetch it here
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!(
                    "Input {} requires non-zero nonce (auto-fetch not yet implemented)",
                    i
                ),
            ));
        } else {
            AddressNonce::from(input.nonce)
        };
        input_map.insert(address, (nonce, input.amount));
    }

    // Parse optional output
    let output_option = if output.is_null() {
        None
    } else {
        let out = &*output;
        if out.address.is_null() || out.address_len == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "Output address is null or empty".to_string(),
            ));
        }

        let address_bytes = std::slice::from_raw_parts(out.address, out.address_len);
        let address = match PlatformAddress::from_bytes(address_bytes) {
            Ok(addr) => addr,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse output address: {}", e),
                ))
            }
        };

        Some((address, out.amount))
    };

    // Convert settings
    let settings = convert_put_settings(put_settings);

    // Execute the creation
    let result: Result<DashSDKIdentityCreateFromAddressesResult, FFIError> =
        wrapper.runtime.block_on(async {
            let (created_identity, address_infos) = identity
                .put_with_address_funding(
                    &wrapper.sdk,
                    input_map,
                    output_option,
                    identity_signer,
                    &address_signer,
                    settings,
                )
                .await
                .map_err(FFIError::from)?;

            // Convert address infos to FFI type
            let entries: Vec<DashSDKAddressInfoEntry> = address_infos
                .iter()
                .map(|(address, info_opt)| {
                    let address_bytes = address.to_bytes().into_boxed_slice();
                    let address_len = address_bytes.len();
                    let address_ptr = Box::into_raw(address_bytes) as *mut u8;

                    // Handle Option<AddressInfo>
                    let (nonce, balance) = match info_opt {
                        Some(info) => (info.nonce, info.balance),
                        None => (u32::MAX, u64::MAX), // Sentinel values for not found
                    };

                    DashSDKAddressInfoEntry {
                        address: address_ptr,
                        address_len,
                        nonce,
                        balance,
                    }
                })
                .collect();

            let entries_len = entries.len();
            let entries_ptr = if entries.is_empty() {
                std::ptr::null_mut()
            } else {
                let boxed = entries.into_boxed_slice();
                Box::into_raw(boxed) as *mut DashSDKAddressInfoEntry
            };

            // Box the created identity to create a handle
            let identity_handle = Box::into_raw(Box::new(created_identity)) as *mut IdentityHandle;

            Ok(DashSDKIdentityCreateFromAddressesResult {
                identity_handle,
                address_info_map: DashSDKAddressInfoMap {
                    entries: entries_ptr,
                    count: entries_len,
                },
            })
        });

    match result {
        Ok(create_result) => {
            let boxed = Box::new(create_result);
            DashSDKResult {
                data_type: DashSDKResultDataType::IdentityCreateFromAddressesResult,
                data: Box::into_raw(boxed) as *mut std::os::raw::c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free the result from identity creation from addresses
///
/// # Safety
/// - `result` must be a valid pointer returned by `dash_sdk_identity_create_from_addresses` and not previously freed.
/// - The identity handle inside the result must be freed separately using `dash_sdk_identity_destroy`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_create_from_addresses_result_free(
    result: *mut DashSDKIdentityCreateFromAddressesResult,
) {
    if !result.is_null() {
        let result = Box::from_raw(result);
        // Free the address info map using the function from types.rs
        dash_sdk_address_info_map_free(&result.address_info_map as *const _ as *mut _);
        // Note: identity_handle must be freed separately by the caller using dash_sdk_identity_destroy
    }
}
