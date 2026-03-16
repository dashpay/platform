//! Address info query operations

use dash_sdk::dpp::address_funds::PlatformAddress;
use dash_sdk::platform::Fetch;
use drive_proof_verifier::types::AddressInfo;
use std::panic::{self, AssertUnwindSafe};

use crate::sdk::SDKWrapper;
use crate::types::{DashSDKAddressInfo, SDKHandle};
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};

/// Fetch information about a Platform address including its nonce and balance.
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `address_bytes`: Address bytes (variable length, typically 20-32 bytes)
/// - `address_len`: Length of address bytes
///
/// # Returns
/// DashSDKResult with data_type = AddressInfo containing address, nonce, and balance
///
/// # Safety
/// - `sdk_handle` and `address_bytes` must be valid, non-null pointers.
/// - `address_bytes` must point to a valid byte array of length `address_len` and remain valid for the duration of the call.
/// - On success, returns a DashSDKAddressInfo pointer inside `DashSDKResult`; caller must free it using `dash_sdk_address_info_free`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_fetch_info(
    sdk_handle: *const SDKHandle,
    address_bytes: *const u8,
    address_len: usize,
) -> DashSDKResult {
    // Wrap the entire function in catch_unwind to prevent panics from crashing the app
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        dash_sdk_address_fetch_info_inner(sdk_handle, address_bytes, address_len)
    }));

    match result {
        Ok(result) => result,
        Err(panic_info) => {
            let panic_message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic occurred during address fetch".to_string()
            };
            DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InternalError,
                format!("Panic during address fetch: {}", panic_message),
            ))
        }
    }
}

unsafe fn dash_sdk_address_fetch_info_inner(
    sdk_handle: *const SDKHandle,
    address_bytes: *const u8,
    address_len: usize,
) -> DashSDKResult {
    if sdk_handle.is_null() || address_bytes.is_null() || address_len == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle, address bytes, or address length is invalid".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Convert address bytes to PlatformAddress
    let address_slice = std::slice::from_raw_parts(address_bytes, address_len);

    // Log the address bytes for debugging
    let address_hex: String = address_slice.iter().map(|b| format!("{:02x}", b)).collect();
    tracing::debug!(address_hex = %address_hex, address_len = address_len, "Attempting to fetch address info");

    let address = match PlatformAddress::from_bytes(address_slice) {
        Ok(addr) => addr,
        Err(e) => {
            tracing::error!(error = %e, address_hex = %address_hex, "Failed to parse PlatformAddress");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("Invalid address bytes (len={}): {}", address_len, e),
            ));
        }
    };

    let result: Result<DashSDKAddressInfo, FFIError> = wrapper.runtime.block_on(async {
        // Fetch address info using Fetch trait
        let address_info = AddressInfo::fetch(&wrapper.sdk, address)
            .await
            .map_err(FFIError::from)?;

        match address_info {
            Some(info) => {
                // Convert address to bytes
                let address_bytes_vec = info.address.to_bytes();
                let address_len = address_bytes_vec.len();
                let address_ptr = Box::into_raw(address_bytes_vec.into_boxed_slice()) as *mut u8;

                Ok(DashSDKAddressInfo {
                    address: address_ptr,
                    address_len,
                    nonce: info.nonce,
                    balance: info.balance,
                })
            }
            None => {
                // Address not found - return with max values to indicate not found
                let address_bytes_vec = address.to_bytes();
                let address_len = address_bytes_vec.len();
                let address_ptr = Box::into_raw(address_bytes_vec.into_boxed_slice()) as *mut u8;

                Ok(DashSDKAddressInfo {
                    address: address_ptr,
                    address_len,
                    nonce: u32::MAX,
                    balance: u64::MAX,
                })
            }
        }
    });

    match result {
        Ok(info) => DashSDKResult::success_address_info(info),
        Err(e) => DashSDKResult::error(e.into()),
    }
}
