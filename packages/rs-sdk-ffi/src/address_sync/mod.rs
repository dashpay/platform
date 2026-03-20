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
use dash_sdk::platform::address_sync::{
    AddressFunds, AddressIndex, AddressKey, AddressProvider, AddressSyncConfig, AddressSyncResult,
};
use dash_sdk::RequestSettings;
use std::collections::{BTreeMap, BTreeSet};
use tracing::{debug, error, info};

/// Synchronize address balances using trunk/branch chunk queries.
///
/// This function discovers address balances for addresses supplied by the provider,
/// using privacy-preserving chunk queries. It supports HD wallet gap limit behavior
/// where finding a used address extends the search range.
///
/// The `last_sync_timestamp` parameter controls incremental vs full-scan behavior:
/// - Pass 0 to force a full tree scan (first sync, or when no previous timestamp exists).
/// - Pass a non-zero unix timestamp (seconds since epoch) from the previous sync to
///   enable incremental-only mode when the elapsed time is within
///   `config.full_rescan_after_time_s`.
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
    last_sync_timestamp: u64,
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
            full_rescan_after_time_s: (*config).full_rescan_after_time_s,
            request_settings: RequestSettings::default(),
        })
    };

    // Convert timestamp: 0 means no previous sync (full scan)
    let last_sync_ts = if last_sync_timestamp == 0 {
        None
    } else {
        Some(last_sync_timestamp)
    };

    debug!(
        "dash_sdk_sync_address_balances: running sync with config: {:?}, last_sync_timestamp: {:?}",
        rust_config, last_sync_ts
    );

    // Create the callback-based provider wrapper
    let mut callback_provider = CallbackAddressProvider::new(provider_ffi);

    // Execute the sync
    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_address_balances(&mut callback_provider, rust_config, last_sync_ts)
            .await
    });

    match result {
        Ok(sync_result) => {
            debug!(
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
/// The `last_sync_timestamp` parameter controls incremental vs full-scan behavior:
/// - Pass 0 to force a full tree scan (first sync, or when no previous timestamp exists).
/// - Pass a non-zero unix timestamp (seconds since epoch) from the previous sync to
///   enable incremental-only mode when the elapsed time is within
///   `config.full_rescan_after_time_s`.
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
    last_sync_timestamp: u64,
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
            full_rescan_after_time_s: (*config).full_rescan_after_time_s,
            request_settings: RequestSettings::default(),
        })
    };

    // Convert timestamp: 0 means no previous sync (full scan)
    let last_sync_ts = if last_sync_timestamp == 0 {
        None
    } else {
        Some(last_sync_timestamp)
    };

    // Create the callback-based provider wrapper
    let mut callback_provider = CallbackAddressProvider::new(provider_ffi);

    // Execute the sync
    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_address_balances(&mut callback_provider, rust_config, last_sync_ts)
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

/// Simple AddressProvider backed by a pre-supplied list of addresses.
///
/// Used by the batch FFI function — no callbacks needed. The caller provides
/// all addresses upfront as a flat array, and results are tracked internally.
struct BatchAddressProvider {
    gap_limit: u32,
    pending: BTreeMap<AddressIndex, AddressKey>,
    found: BTreeMap<(AddressIndex, AddressKey), AddressFunds>,
    absent: BTreeSet<(AddressIndex, AddressKey)>,
    highest_found: Option<AddressIndex>,
    /// Previously known balances from the last sync (for incremental-only mode).
    known_balances: Vec<(AddressIndex, AddressKey, AddressFunds)>,
    /// Last sync height from the previous sync (for incremental catch-up resume).
    sync_height: u64,
    /// Last known recent block height from the previous sync (for compaction detection).
    last_known_recent_block: u64,
}

impl AddressProvider for BatchAddressProvider {
    fn gap_limit(&self) -> AddressIndex {
        self.gap_limit
    }

    fn pending_addresses(&self) -> Vec<(AddressIndex, AddressKey)> {
        self.pending.iter().map(|(i, k)| (*i, k.clone())).collect()
    }

    fn on_address_found(&mut self, index: AddressIndex, key: &[u8], funds: AddressFunds) {
        self.pending.remove(&index);
        self.found.insert((index, key.to_vec()), funds);
        self.highest_found = Some(self.highest_found.map_or(index, |v| v.max(index)));
    }

    fn on_address_absent(&mut self, index: AddressIndex, key: &[u8]) {
        self.pending.remove(&index);
        self.absent.insert((index, key.to_vec()));
    }

    fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    fn highest_found_index(&self) -> Option<AddressIndex> {
        self.highest_found
    }

    fn current_balances(&self) -> Vec<(AddressIndex, AddressKey, AddressFunds)> {
        self.known_balances.clone()
    }

    fn last_sync_height(&self) -> u64 {
        self.sync_height
    }

    fn last_known_recent_block_height(&self) -> u64 {
        self.last_known_recent_block
    }
}

/// Synchronize address balances using a flat array of addresses (no callbacks).
///
/// This is a simpler alternative to the callback-based `dash_sdk_sync_address_balances`.
/// The caller provides all addresses upfront as a flat array. Ideal when addresses
/// are already derived (e.g., from the wallet's platform payment accounts).
///
/// # Parameters
/// - `sdk_handle`: Valid SDK handle
/// - `address_keys`: Pointer to concatenated address key bytes (each key is `key_size` bytes)
/// - `address_indices`: Pointer to array of derivation indices (one per address)
/// - `address_count`: Number of addresses
/// - `key_size`: Size of each address key in bytes (typically 20)
/// - `gap_limit`: HD wallet gap limit
/// - `known_balance_keys`: Pointer to concatenated keys of previously found addresses (each `key_size` bytes), or null
/// - `known_balance_indices`: Pointer to array of derivation indices for known balances, or null
/// - `known_balance_nonces`: Pointer to array of nonces for known balances, or null
/// - `known_balance_amounts`: Pointer to array of u64 balances for known addresses, or null
/// - `known_balance_count`: Number of known balance entries (0 if none)
/// - `config`: Optional sync config (NULL for defaults)
/// - `last_sync_height`: Height from previous sync result (0 if first sync)
/// - `last_sync_timestamp`: 0 for full scan, or timestamp from previous sync
/// - `last_known_recent_block`: Highest block from previous recent entries (0 if first sync).
///   Enables compaction detection via boundary checks on the hot path.
///
/// # Returns
/// DashSDKResult containing DashSDKAddressSyncResult on success
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle created by this SDK
/// - `address_keys` must point to `address_count * key_size` contiguous bytes
/// - `address_indices` must point to `address_count` u32 values
/// - `config` may be null (uses defaults) or a valid pointer to DashSDKAddressSyncConfig
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_sync_addresses_batch_with_result(
    sdk_handle: *const SDKHandle,
    address_keys: *const u8,
    address_indices: *const u32,
    address_count: u32,
    key_size: u32,
    gap_limit: u32,
    known_balance_keys: *const u8,
    known_balance_indices: *const u32,
    known_balance_nonces: *const u32,
    known_balance_amounts: *const u64,
    known_balance_count: u32,
    config: *const DashSDKAddressSyncConfig,
    last_sync_height: u64,
    last_sync_timestamp: u64,
    last_known_recent_block: u64,
) -> DashSDKResult {
    info!(
        "dash_sdk_sync_addresses_batch_with_result: called with {} addresses, key_size={}",
        address_count, key_size
    );

    if sdk_handle.is_null() {
        error!("dash_sdk_sync_addresses_batch_with_result: SDK handle is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if address_count > 0 && address_keys.is_null() {
        error!("dash_sdk_sync_addresses_batch_with_result: address_keys is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "address_keys is null".to_string(),
        ));
    }

    if address_count > 0 && address_indices.is_null() {
        error!("dash_sdk_sync_addresses_batch_with_result: address_indices is null");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "address_indices is null".to_string(),
        ));
    }

    if key_size == 0 && address_count > 0 {
        error!("dash_sdk_sync_addresses_batch_with_result: key_size is 0");
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "key_size must be > 0 when address_count > 0".to_string(),
        ));
    }

    // Validate known balance pointers when known_balance_count > 0
    if known_balance_count > 0 {
        if key_size == 0 {
            error!("dash_sdk_sync_addresses_batch_with_result: key_size is 0 but known_balance_count > 0");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "key_size must be > 0 when known_balance_count > 0".to_string(),
            ));
        }
        if known_balance_keys.is_null() {
            error!("dash_sdk_sync_addresses_batch_with_result: known_balance_keys is null but known_balance_count > 0");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "known_balance_keys must not be null when known_balance_count > 0".to_string(),
            ));
        }
        if known_balance_indices.is_null() {
            error!("dash_sdk_sync_addresses_batch_with_result: known_balance_indices is null but known_balance_count > 0");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "known_balance_indices must not be null when known_balance_count > 0".to_string(),
            ));
        }
        if known_balance_nonces.is_null() {
            error!("dash_sdk_sync_addresses_batch_with_result: known_balance_nonces is null but known_balance_count > 0");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "known_balance_nonces must not be null when known_balance_count > 0".to_string(),
            ));
        }
        if known_balance_amounts.is_null() {
            error!("dash_sdk_sync_addresses_batch_with_result: known_balance_amounts is null but known_balance_count > 0");
            return DashSDKResult::error(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                "known_balance_amounts must not be null when known_balance_count > 0".to_string(),
            ));
        }
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Parse flat arrays into BTreeMap
    let mut pending = BTreeMap::new();
    if address_count > 0 {
        let total_key_bytes = match (address_count as usize).checked_mul(key_size as usize) {
            Some(n) => n,
            None => {
                error!(
                    "dash_sdk_sync_addresses_batch_with_result: address_count * key_size overflow"
                );
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "address_count * key_size overflows usize".to_string(),
                ));
            }
        };
        let keys_slice = std::slice::from_raw_parts(address_keys, total_key_bytes);
        let indices_slice = std::slice::from_raw_parts(address_indices, address_count as usize);

        for (i, &index) in indices_slice.iter().enumerate() {
            let key_start = i * key_size as usize;
            let key_end = key_start + key_size as usize;
            let key = keys_slice[key_start..key_end].to_vec();
            pending.insert(index, key);
        }
    }

    // Parse known balances from previous sync (for incremental-only mode)
    let mut known_balances = Vec::new();
    if known_balance_count > 0
        && !known_balance_keys.is_null()
        && !known_balance_indices.is_null()
        && !known_balance_nonces.is_null()
        && !known_balance_amounts.is_null()
    {
        let total_kb_key_bytes = match (known_balance_count as usize).checked_mul(key_size as usize)
        {
            Some(n) => n,
            None => {
                error!("dash_sdk_sync_addresses_batch_with_result: known_balance_count * key_size overflow");
                return DashSDKResult::error(DashSDKError::new(
                    DashSDKErrorCode::InvalidParameter,
                    "known_balance_count * key_size overflows usize".to_string(),
                ));
            }
        };
        let kb_keys_slice = std::slice::from_raw_parts(known_balance_keys, total_kb_key_bytes);
        let kb_indices =
            std::slice::from_raw_parts(known_balance_indices, known_balance_count as usize);
        let kb_nonces =
            std::slice::from_raw_parts(known_balance_nonces, known_balance_count as usize);
        let kb_amounts =
            std::slice::from_raw_parts(known_balance_amounts, known_balance_count as usize);

        for (i, &index) in kb_indices.iter().enumerate() {
            let key_start = i * key_size as usize;
            let key_end = key_start + key_size as usize;
            let key = kb_keys_slice[key_start..key_end].to_vec();
            known_balances.push((
                index,
                key,
                AddressFunds {
                    nonce: kb_nonces[i],
                    balance: kb_amounts[i],
                },
            ));
        }
    }

    debug!(
        "dash_sdk_sync_addresses_batch_with_result: parsed {} addresses, {} known balances, gap_limit={}, last_sync_height={}",
        pending.len(),
        known_balances.len(),
        gap_limit,
        last_sync_height
    );

    // Seed highest_found from known balances so we don't lose the high-water mark
    let initial_highest_found = known_balances.iter().map(|(index, _, _)| *index).max();

    let mut provider = BatchAddressProvider {
        gap_limit,
        pending,
        found: BTreeMap::new(),
        absent: BTreeSet::new(),
        highest_found: initial_highest_found,
        known_balances,
        sync_height: last_sync_height,
        last_known_recent_block,
    };

    // Build request settings with shorter timeouts for mobile
    let mobile_settings = RequestSettings {
        connect_timeout: Some(std::time::Duration::from_secs(3)),
        timeout: Some(std::time::Duration::from_secs(5)),
        retries: Some(2),
        ..RequestSettings::default()
    };

    // Convert config
    let rust_config = if config.is_null() {
        Some(AddressSyncConfig {
            request_settings: mobile_settings,
            ..AddressSyncConfig::default()
        })
    } else {
        Some(AddressSyncConfig {
            min_privacy_count: (*config).min_privacy_count,
            max_concurrent_requests: (*config).max_concurrent_requests as usize,
            max_iterations: (*config).max_iterations as usize,
            full_rescan_after_time_s: (*config).full_rescan_after_time_s,
            request_settings: mobile_settings,
        })
    };

    // Convert timestamp: 0 means no previous sync (full scan)
    let last_sync_ts = if last_sync_timestamp == 0 {
        None
    } else {
        Some(last_sync_timestamp)
    };

    // Execute the sync
    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_address_balances(&mut provider, rust_config, last_sync_ts)
            .await
    });

    // Build set of previously known indices for change detection
    let known_indices: std::collections::BTreeSet<u32> = provider
        .known_balances
        .iter()
        .map(|(index, _, _)| *index)
        .collect();

    match result {
        Ok(sync_result) => {
            let newly_found = sync_result
                .found
                .keys()
                .filter(|(index, _)| !known_indices.contains(index))
                .count();
            if newly_found > 0 {
                info!(
                    "dash_sdk_sync_addresses_batch_with_result: {} newly found addresses ({} total found, {} absent)",
                    newly_found,
                    sync_result.found.len(),
                    sync_result.absent.len()
                );
            } else {
                debug!(
                    "dash_sdk_sync_addresses_batch_with_result: no new addresses ({} known, {} absent)",
                    sync_result.found.len(),
                    sync_result.absent.len()
                );
            }
            let ffi_result = Box::new(convert_sync_result(sync_result));
            DashSDKResult::success(Box::into_raw(ffi_result) as *mut std::os::raw::c_void)
        }
        Err(e) => {
            error!("dash_sdk_sync_addresses_batch_with_result: error - {}", e);
            DashSDKResult::error(FFIError::SDKError(e).into())
        }
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
        compacted_queries: result.metrics.compacted_queries as u32,
        recent_queries: result.metrics.recent_queries as u32,
        recent_entries_returned: result.metrics.recent_entries_returned as u32,
        compacted_entries_returned: result.metrics.compacted_entries_returned as u32,
    };

    // Convert recent proof bytes
    let (recent_proof_ptr, recent_proof_len) = if result.recent_proof.is_empty() {
        (std::ptr::null_mut(), 0)
    } else {
        let proof_data = result.recent_proof.into_boxed_slice();
        let len = proof_data.len();
        let ptr = Box::into_raw(proof_data) as *mut u8;
        (ptr, len)
    };

    DashSDKAddressSyncResult {
        found: found_ptr,
        found_count,
        absent: absent_ptr,
        absent_count,
        highest_found_index: result.highest_found_index.unwrap_or(u32::MAX),
        has_highest_found_index: result.highest_found_index.is_some(),
        checkpoint_height: result.checkpoint_height,
        new_sync_height: result.new_sync_height,
        new_sync_timestamp: result.new_sync_timestamp,
        last_known_recent_block: result.last_known_recent_block,
        metrics,
        recent_proof: recent_proof_ptr,
        recent_proof_len,
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

    // Free recent proof bytes
    if !result.recent_proof.is_null() && result.recent_proof_len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.recent_proof,
            result.recent_proof_len,
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
        assert_eq!(config.full_rescan_after_time_s, 7 * 24 * 60 * 60);
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
