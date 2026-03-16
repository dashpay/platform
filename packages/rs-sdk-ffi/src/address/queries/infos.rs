//! Multiple addresses info query operations
//!
//! Note: `catch_unwind` intentionally not used -- `panic = "abort"` in the release profile
//! makes it a no-op, and dereferencing invalid pointers is UB regardless of `catch_unwind`.

use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::platform::FetchMany;
use drive_proof_verifier::types::{AddressInfo, AddressInfos};
use std::collections::BTreeSet;

use crate::sdk::SDKWrapper;
use crate::types::{DashSDKAddressInfoEntry, DashSDKAddressInfoMap, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch information about multiple Platform addresses including their nonces and balances.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `addresses`: Array of address byte arrays
/// - `address_lengths`: Array of lengths for each address (must match addresses array length)
/// - `addresses_count`: Number of addresses in the array
///
/// # Returns
/// DashSDKResult with data_type = AddressInfoMap containing addresses mapped to their info
///
/// # Safety
/// - `sdk_handle`, `addresses`, and `address_lengths` must be valid, non-null pointers.
/// - `addresses` must point to an array of `*const u8` of length `addresses_count`.
/// - `address_lengths` must point to an array of `usize` of length `addresses_count`.
/// - Each address pointer in `addresses` must point to valid bytes of the corresponding length.
/// - All pointers must remain valid for the duration of the call.
/// - On success, returns a DashSDKAddressInfoMap pointer inside `DashSDKResult`; caller must free it using `dash_sdk_address_info_map_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_addresses_fetch_infos(
    sdk_handle: *const SDKHandle,
    addresses: *const *const u8,
    address_lengths: *const usize,
    addresses_count: usize,
) -> DashSDKResult {
    if sdk_handle.is_null()
        || (addresses.is_null() && addresses_count > 0)
        || (address_lengths.is_null() && addresses_count > 0)
    {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, addresses, or address lengths is null".to_string(),
        ));
    }

    if addresses_count == 0 {
        // Return empty map for empty input
        let map = DashSDKAddressInfoMap {
            entries: std::ptr::null_mut(),
            count: 0,
        };
        return DashSDKResult::success_address_info_map(map);
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    tracing::debug!(
        addresses_count = addresses_count,
        "Attempting to fetch multiple addresses info"
    );

    // Convert raw pointers to PlatformAddress set
    let addresses_slice = std::slice::from_raw_parts(addresses, addresses_count);
    let lengths_slice = std::slice::from_raw_parts(address_lengths, addresses_count);

    let platform_addresses: Result<BTreeSet<PlatformAddress>, DashSDKError> = addresses_slice
        .iter()
        .enumerate()
        .map(|(i, &addr_ptr)| {
            if addr_ptr.is_null() {
                return Err(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Address pointer at index {} is null", i),
                ));
            }

            let len = lengths_slice[i];
            if len == 0 {
                return Err(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Address length at index {} is zero", i),
                ));
            }

            let address_slice = std::slice::from_raw_parts(addr_ptr, len);
            PlatformAddress::from_bytes(address_slice).map_err(|e| {
                DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    format!("Invalid address bytes at index {}: {}", i, e),
                )
            })
        })
        .collect();

    let platform_addresses = match platform_addresses {
        Ok(addrs) => addrs,
        Err(e) => return DashSDKResult::error(e),
    };

    // Keep copies of original addresses and their corresponding PlatformAddress for result mapping
    let original_addresses_with_platform: Result<Vec<(Vec<u8>, PlatformAddress)>, DashSDKError> =
        addresses_slice
            .iter()
            .enumerate()
            .map(|(i, &addr_ptr)| {
                let len = lengths_slice[i];
                let address_slice = std::slice::from_raw_parts(addr_ptr, len);
                let address_bytes = address_slice.to_vec();
                // Find the corresponding PlatformAddress from the set
                let platform_address = platform_addresses
                    .iter()
                    .find(|&addr| addr.to_bytes() == address_bytes)
                    .copied()
                    .ok_or_else(|| {
                        DashSDKError::new(
                            DashSDKErrorCode::InvalidParameter,
                            format!(
                                "Could not find address at index {} in platform_addresses",
                                i
                            ),
                        )
                    })?;
                Ok((address_bytes, platform_address))
            })
            .collect();

    let original_addresses_with_platform = match original_addresses_with_platform {
        Ok(addrs) => addrs,
        Err(e) => return DashSDKResult::error(e),
    };

    let result: Result<DashSDKAddressInfoMap, FFIError> = wrapper.runtime.block_on(async {
        // Fetch addresses infos
        let address_infos: AddressInfos = AddressInfo::fetch_many(&wrapper.sdk, platform_addresses)
            .await
            .map_err(FFIError::from)?;

        // Convert to entries array
        let mut entries: Vec<DashSDKAddressInfoEntry> = Vec::with_capacity(addresses_count);

        // Process results in the same order as input
        for (address_bytes, platform_address) in original_addresses_with_platform.iter() {
            let address_info = address_infos
                .get(platform_address)
                .and_then(|opt| opt.as_ref());

            let (nonce, balance) = match address_info {
                Some(info) => (info.nonce, info.balance),
                None => (u32::MAX, u64::MAX), // Not found
            };

            // Allocate address bytes
            let address_bytes_vec = address_bytes.clone();
            let address_len = address_bytes_vec.len();
            let address_ptr = address_bytes_vec.as_ptr() as *mut u8;
            std::mem::forget(address_bytes_vec); // Prevent deallocation

            entries.push(DashSDKAddressInfoEntry {
                address: address_ptr,
                address_len,
                nonce,
                balance,
            });
        }

        let count = entries.len();
        let entries_ptr = entries.as_mut_ptr();
        std::mem::forget(entries); // Prevent deallocation

        Ok(DashSDKAddressInfoMap {
            entries: entries_ptr,
            count,
        })
    });

    match result {
        Ok(map) => DashSDKResult::success_address_info_map(map),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
