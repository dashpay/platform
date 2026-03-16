//! Identity credit transfer to addresses operations
//!
//! Note: `catch_unwind` intentionally not used -- `panic = "abort"` in the release profile
//! makes it a no-op, and dereferencing invalid pointers is UB regardless of `catch_unwind`.

use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::Identity;
use dash_sdk::platform::transition::transfer_to_addresses::TransferToAddresses;
use std::collections::BTreeMap;

use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{
    dash_sdk_address_info_map_free, DashSDKAddressInfoEntry, DashSDKAddressInfoMap,
    DashSDKAddressTransferOutput, DashSDKPutSettings, DashSDKResultDataType, IdentityHandle,
    SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Result for identity transfer to addresses
#[repr(C)]
pub struct DashSDKIdentityTransferToAddressesResult {
    /// Updated identity balance
    pub identity_balance: u64,
    /// Address info map
    pub address_info_map: DashSDKAddressInfoMap,
}

/// Transfer credits from an identity to Platform addresses
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_handle`: Identity to transfer from
/// - `outputs`: Array of output addresses with amounts
/// - `outputs_count`: Number of output entries
/// - `public_key_id`: ID of the public key to use for signing (0 = auto-select TRANSFER key)
/// - `signer_handle`: Cryptographic signer for identity
/// - `put_settings`: Optional settings for the operation (can be null for defaults)
///
/// # Returns
/// DashSDKResult with custom data type containing identity balance and address infos
///
/// # Safety
/// - All pointers must be valid and non-null (except put_settings which can be null).
/// - Arrays must contain at least the specified count of elements.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_transfer_credits_to_addresses(
    sdk_handle: *const SDKHandle,
    identity_handle: *const IdentityHandle,
    outputs: *const DashSDKAddressTransferOutput,
    outputs_count: usize,
    public_key_id: u32,
    signer_handle: *const crate::types::SignerHandle,
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

    if outputs.is_null() || outputs_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Outputs array is null or empty".to_string(),
        ));
    }

    if signer_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Signer handle is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let identity = &*(identity_handle as *const Identity);
    let signer = &*(signer_handle as *const crate::signer::VTableSigner);

    // Parse outputs
    let mut recipient_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let outputs_slice = std::slice::from_raw_parts(outputs, outputs_count);
    for (i, output) in outputs_slice.iter().enumerate() {
        if output.address.is_null() || output.address_len == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Output {} has null or empty address", i),
            ));
        }

        let address_bytes = std::slice::from_raw_parts(output.address, output.address_len);
        let address = match PlatformAddress::from_bytes(address_bytes) {
            Ok(addr) => addr,
            Err(e) => {
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Failed to parse output address {}: {}", i, e),
                ))
            }
        };

        if output.amount == 0 {
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Output {} has zero amount", i),
            ));
        }

        recipient_map.insert(address, output.amount);
    }

    // Get signing key (if public_key_id is 0, use auto-select)
    use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    let signing_key = if public_key_id == 0 {
        None
    } else {
        // Find the public key by ID
        identity
            .public_keys()
            .iter()
            .find(|(_, pk)| pk.id() == public_key_id)
            .map(|(_, pk)| pk)
    };

    // Convert settings
    let settings = convert_put_settings(put_settings);

    // Execute the transfer
    let result: Result<DashSDKIdentityTransferToAddressesResult, FFIError> =
        wrapper.runtime.block_on(async {
            let (address_infos, identity_balance) = identity
                .transfer_credits_to_addresses(
                    &wrapper.sdk,
                    recipient_map,
                    signing_key,
                    signer,
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

            Ok(DashSDKIdentityTransferToAddressesResult {
                identity_balance,
                address_info_map: DashSDKAddressInfoMap {
                    entries: entries_ptr,
                    count: entries_len,
                },
            })
        });

    match result {
        Ok(transfer_result) => {
            let boxed = Box::new(transfer_result);
            DashSDKResult {
                data_type: DashSDKResultDataType::IdentityTransferToAddressesResult,
                data: Box::into_raw(boxed) as *mut std::os::raw::c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free the result from identity transfer to addresses
///
/// # Safety
/// - `result` must be a valid pointer returned by `dash_sdk_identity_transfer_credits_to_addresses` and not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_transfer_to_addresses_result_free(
    result: *mut DashSDKIdentityTransferToAddressesResult,
) {
    if !result.is_null() {
        let result = Box::from_raw(result);
        // Free the address info map using the function from types.rs
        dash_sdk_address_info_map_free(&result.address_info_map as *const _ as *mut _);
    }
}
