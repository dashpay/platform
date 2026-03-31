//! Identity top-up from addresses operations

use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::dpp::dashcore::secp256k1::SecretKey;
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::prelude::Identity;
use dash_sdk::platform::transition::top_up_identity_from_addresses::TopUpIdentityFromAddresses;
use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};

use crate::address::transitions::AddressSigner;
use crate::identity::helpers::convert_put_settings;
use crate::sdk::SDKWrapper;
use crate::types::{
    DashSDKAddressInfoEntry, DashSDKAddressInfoMap, DashSDKAddressTransferInput,
    DashSDKPutSettings, DashSDKResultDataType, IdentityHandle, SDKHandle,
};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Result for identity top-up from addresses
#[repr(C)]
pub struct DashSDKIdentityTopUpFromAddressesResult {
    /// Updated identity balance
    pub identity_balance: u64,
    /// Address info map
    pub address_info_map: DashSDKAddressInfoMap,
}

/// Top up an identity using Platform address balances
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `identity_handle`: Identity to top up
/// - `inputs`: Array of input addresses with amounts and private keys
/// - `inputs_count`: Number of input entries
///
/// # Returns
/// DashSDKResult with custom data type containing identity balance and address infos
///
/// # Safety
/// - All pointers must be valid and non-null (except put_settings which can be null).
/// - Arrays must contain at least the specified count of elements.
/// - Private keys must be exactly 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_top_up_from_addresses(
    sdk_handle: *const SDKHandle,
    identity_handle: *const IdentityHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
    put_settings: *const DashSDKPutSettings,
) -> DashSDKResult {
    // SAFETY: catch_unwind is kept intentionally despite `panic = "abort"` in the release profile.
    // With panic=abort, catch_unwind is optimized away (zero cost). But keeping it:
    // 1. Acts as a safety net if the panic strategy is ever changed (e.g., for debugging)
    // 2. Documents the intent that panics must not cross this FFI boundary
    // 3. Follows defense-in-depth for FFI safety
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_identity_top_up_from_addresses_inner(
            sdk_handle,
            identity_handle,
            inputs,
            inputs_count,
            put_settings,
        )
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during identity top-up from addresses".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!(
                    "Panic during identity top-up from addresses: {}",
                    panic_message
                ),
            ))
        }
    }
}

unsafe fn dash_sdk_identity_top_up_from_addresses_inner(
    sdk_handle: *const SDKHandle,
    identity_handle: *const IdentityHandle,
    inputs: *const DashSDKAddressTransferInput,
    inputs_count: usize,
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

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let identity = &*(identity_handle as *const Identity);

    // Parse inputs and create signer (same as address transfer)
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    let mut signer = AddressSigner::new();

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

        signer.add_key(&address, private_key);
        input_map.insert(address, input.amount);
    }

    // Convert settings
    let settings = convert_put_settings(put_settings);

    // Execute the top-up
    let result: Result<DashSDKIdentityTopUpFromAddressesResult, FFIError> =
        wrapper.runtime.block_on(async {
            let (address_infos, identity_balance) = identity
                .top_up_from_addresses(&wrapper.sdk, input_map, &signer, settings)
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

            Ok(DashSDKIdentityTopUpFromAddressesResult {
                identity_balance,
                address_info_map: DashSDKAddressInfoMap {
                    entries: entries_ptr,
                    count: entries_len,
                },
            })
        });

    match result {
        Ok(top_up_result) => {
            let boxed = Box::new(top_up_result);
            DashSDKResult {
                data_type: DashSDKResultDataType::IdentityTopUpFromAddressesResult,
                data: Box::into_raw(boxed) as *mut std::os::raw::c_void,
                error: std::ptr::null_mut(),
            }
        }
        Err(e) => DashSDKResult::error(e.into()),
    }
}

/// Free the result from identity top-up from addresses
///
/// # Safety
/// - `result` must be a valid pointer returned by `dash_sdk_identity_top_up_from_addresses` and not previously freed.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_identity_top_up_from_addresses_result_free(
    result: *mut DashSDKIdentityTopUpFromAddressesResult,
) {
    if !result.is_null() {
        let result = Box::from_raw(result);
        // Free the address info map entries inline — do NOT call
        // dash_sdk_address_info_map_free here because that calls Box::from_raw
        // on the map pointer, but the map is an embedded field (not a separate
        // heap allocation), which would cause a double-free / heap corruption.
        crate::types::free_address_info_map_entries(&result.address_info_map);
    }
}
