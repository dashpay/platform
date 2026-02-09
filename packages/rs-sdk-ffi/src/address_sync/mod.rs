//! Address synchronization FFI bindings
//!
//! This module provides C-compatible FFI bindings for the address_sync functionality
//! from rs-sdk. It allows Swift/iOS to synchronize address balances using
//! privacy-preserving trunk/branch chunk queries.

mod provider;
mod types;

pub use provider::*;
pub use types::*;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::platform::address_sync::{AddressSyncConfig, AddressSyncResult};
use dash_sdk::RequestSettings;
use tracing::{debug, error, info};

/// Synchronize address balances using trunk/branch chunk queries.
///
/// This function discovers address balances for addresses supplied by the provider,
/// using privacy-preserving chunk queries. It supports HD wallet gap limit behavior
/// where finding a used address extends the search range.
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle created by this SDK
/// - `provider` must be a valid pointer to an AddressProviderFFI structure
/// - `config` may be null (uses defaults) or a valid pointer to DashSDKAddressSyncConfig
/// - The returned result must be freed with `dash_sdk_address_sync_result_free`
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_sync_address_balances(
    sdk_handle: *const SDKHandle,
    provider: *mut AddressProviderFFI,
    config: *const DashSDKAddressSyncConfig,
) -> *mut DashSDKAddressSyncResult {
    info!("dash_sdk_sync_address_balances: called");

    if sdk_handle.is_null() {
        error!("dash_sdk_sync_address_balances: SDK handle is null");
        return std::ptr::null_mut();
    }

    if provider.is_null() {
        error!("dash_sdk_sync_address_balances: provider is null");
        return std::ptr::null_mut();
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let provider_ffi = &mut *provider;

    // Convert config
    let rust_config = if config.is_null() {
        None
    } else {
        Some(AddressSyncConfig {
            min_privacy_count: (*config).min_privacy_count,
            max_concurrent_requests: (*config).max_concurrent_requests as usize,
            max_iterations: (*config).max_iterations as usize,
            request_settings: RequestSettings::default(),
        })
    };

    debug!(
        "dash_sdk_sync_address_balances: running sync with config: {:?}",
        rust_config
    );

    // Create the callback-based provider wrapper
    let mut callback_provider = CallbackAddressProvider::new(provider_ffi);

    // Execute the sync
    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_address_balances(&mut callback_provider, rust_config)
            .await
    });

    match result {
        Ok(sync_result) => {
            info!(
                "dash_sdk_sync_address_balances: success - found {} addresses, {} absent",
                sync_result.found.len(),
                sync_result.absent.len()
            );
            Box::into_raw(Box::new(convert_sync_result(sync_result)))
        }
        Err(e) => {
            error!("dash_sdk_sync_address_balances: error - {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Synchronize address balances and return result with error information.
///
/// This is an alternative version that returns a DashSDKResult for better error handling.
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle created by this SDK
/// - `provider` must be a valid pointer to an AddressProviderFFI structure
/// - `config` may be null (uses defaults) or a valid pointer to DashSDKAddressSyncConfig
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_sync_address_balances_with_result(
    sdk_handle: *const SDKHandle,
    provider: *mut AddressProviderFFI,
    config: *const DashSDKAddressSyncConfig,
) -> DashSDKResult {
    info!("dash_sdk_sync_address_balances_with_result: called");

    if sdk_handle.is_null() {
        error!("dash_sdk_sync_address_balances_with_result: SDK handle is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if provider.is_null() {
        error!("dash_sdk_sync_address_balances_with_result: provider is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Address provider is null".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);
    let provider_ffi = &mut *provider;

    // Convert config
    let rust_config = if config.is_null() {
        None
    } else {
        Some(AddressSyncConfig {
            min_privacy_count: (*config).min_privacy_count,
            max_concurrent_requests: (*config).max_concurrent_requests as usize,
            max_iterations: (*config).max_iterations as usize,
            request_settings: RequestSettings::default(),
        })
    };

    // Create the callback-based provider wrapper
    let mut callback_provider = CallbackAddressProvider::new(provider_ffi);

    // Execute the sync
    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_address_balances(&mut callback_provider, rust_config)
            .await
    });

    match result {
        Ok(sync_result) => {
            let ffi_result = Box::new(convert_sync_result(sync_result));
            DashSDKResult::success(Box::into_raw(ffi_result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(FFIError::SDKError(e).into()),
    }
}

/// Convert Rust AddressSyncResult to FFI-compatible result
fn convert_sync_result(result: AddressSyncResult) -> DashSDKAddressSyncResult {
    // Convert found addresses
    let mut found_entries: Vec<DashSDKFoundAddress> = Vec::with_capacity(result.found.len());
    for ((index, key), funds) in result.found.iter() {
        let key_data = key.clone().into_boxed_slice();
        let key_len = key_data.len();
        let key_ptr = Box::into_raw(key_data) as *mut u8;

        found_entries.push(DashSDKFoundAddress {
            index: *index,
            key: key_ptr,
            key_len,
            nonce: funds.nonce,
            balance: funds.balance,
        });
    }

    let found_count = found_entries.len();
    let found_ptr = if found_entries.is_empty() {
        std::ptr::null_mut()
    } else {
        let boxed_slice = found_entries.into_boxed_slice();
        Box::into_raw(boxed_slice) as *mut DashSDKFoundAddress
    };

    // Convert absent addresses
    let mut absent_entries: Vec<DashSDKAbsentAddress> = Vec::with_capacity(result.absent.len());
    for (index, key) in result.absent.iter() {
        let key_data = key.clone().into_boxed_slice();
        let key_len = key_data.len();
        let key_ptr = Box::into_raw(key_data) as *mut u8;

        absent_entries.push(DashSDKAbsentAddress {
            index: *index,
            key: key_ptr,
            key_len,
        });
    }

    let absent_count = absent_entries.len();
    let absent_ptr = if absent_entries.is_empty() {
        std::ptr::null_mut()
    } else {
        let boxed_slice = absent_entries.into_boxed_slice();
        Box::into_raw(boxed_slice) as *mut DashSDKAbsentAddress
    };

    // Convert metrics
    let metrics = DashSDKAddressSyncMetrics {
        trunk_queries: result.metrics.trunk_queries as u32,
        branch_queries: result.metrics.branch_queries as u32,
        total_elements_seen: result.metrics.total_elements_seen as u32,
        total_proof_bytes: result.metrics.total_proof_bytes as u32,
        iterations: result.metrics.iterations as u32,
    };

    DashSDKAddressSyncResult {
        found: found_ptr,
        found_count,
        absent: absent_ptr,
        absent_count,
        highest_found_index: result.highest_found_index.unwrap_or(u32::MAX),
        has_highest_found_index: result.highest_found_index.is_some(),
        metrics,
    }
}

/// Free an address sync result
///
/// # Safety
/// - `result` must be a valid pointer returned by `dash_sdk_sync_address_balances`
///   or null (no-op)
/// - After this call, the result must not be used again
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_sync_result_free(result: *mut DashSDKAddressSyncResult) {
    if result.is_null() {
        return;
    }

    let result = Box::from_raw(result);

    // Free found addresses
    if !result.found.is_null() && result.found_count > 0 {
        let found_slice = std::slice::from_raw_parts_mut(result.found, result.found_count);
        for entry in found_slice.iter() {
            if !entry.key.is_null() && entry.key_len > 0 {
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entry.key, entry.key_len));
            }
        }
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.found,
            result.found_count,
        ));
    }

    // Free absent addresses
    if !result.absent.is_null() && result.absent_count > 0 {
        let absent_slice = std::slice::from_raw_parts_mut(result.absent, result.absent_count);
        for entry in absent_slice.iter() {
            if !entry.key.is_null() && entry.key_len > 0 {
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entry.key, entry.key_len));
            }
        }
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.absent,
            result.absent_count,
        ));
    }
}

/// Create an address provider with callbacks
///
/// This creates an FFI-compatible address provider that uses callbacks
/// for all operations.
///
/// # Safety
/// - `vtable` must be a valid pointer to an AddressProviderVTable structure
/// - `context` is an opaque pointer that will be passed to all callbacks
/// - The returned provider must be freed with `dash_sdk_address_provider_free`
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_provider_create(
    vtable: *const AddressProviderVTable,
    context: *mut std::os::raw::c_void,
) -> *mut AddressProviderFFI {
    if vtable.is_null() {
        return std::ptr::null_mut();
    }

    Box::into_raw(Box::new(AddressProviderFFI { context, vtable }))
}

/// Free an address provider
///
/// # Safety
/// - `provider` must be a valid pointer returned by `dash_sdk_address_provider_create`
///   or null (no-op)
/// - After this call, the provider must not be used again
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_provider_free(provider: *mut AddressProviderFFI) {
    if provider.is_null() {
        return;
    }

    let provider = Box::from_raw(provider);

    // Call the destroy callback if provided
    if !provider.vtable.is_null() {
        let vtable = &*provider.vtable;
        if let Some(destroy) = vtable.destroy {
            destroy(provider.context);
        }
    }
}

/// Get the total balance from a sync result
///
/// # Safety
/// - `result` must be a valid pointer to a DashSDKAddressSyncResult
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_sync_result_total_balance(
    result: *const DashSDKAddressSyncResult,
) -> u64 {
    if result.is_null() {
        return 0;
    }

    let result = &*result;
    if result.found.is_null() || result.found_count == 0 {
        return 0;
    }

    let found_slice = std::slice::from_raw_parts(result.found, result.found_count);
    found_slice.iter().map(|entry| entry.balance).sum()
}

/// Get the count of addresses with non-zero balance
///
/// # Safety
/// - `result` must be a valid pointer to a DashSDKAddressSyncResult
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_address_sync_result_non_zero_count(
    result: *const DashSDKAddressSyncResult,
) -> usize {
    if result.is_null() {
        return 0;
    }

    let result = &*result;
    if result.found.is_null() || result.found_count == 0 {
        return 0;
    }

    let found_slice = std::slice::from_raw_parts(result.found, result.found_count);
    found_slice.iter().filter(|entry| entry.balance > 0).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DashSDKAddressSyncConfig::default();
        assert_eq!(config.min_privacy_count, 32);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.max_iterations, 50);
    }

    #[test]
    fn test_metrics_default() {
        let metrics = DashSDKAddressSyncMetrics::default();
        assert_eq!(metrics.trunk_queries, 0);
        assert_eq!(metrics.branch_queries, 0);
        assert_eq!(metrics.total_elements_seen, 0);
        assert_eq!(metrics.iterations, 0);
    }
}
