//! Nullifier BLAST sync FFI bindings.
//!
//! Privacy-preserving nullifier status checking using trunk/branch chunk queries,
//! mirroring the `address_sync` pattern.

mod types;

pub use types::*;

use crate::sdk::SDKWrapper;
use crate::types::SDKHandle;
use crate::{DashSDKError, DashSDKErrorCode, DashSDKResult, FFIError};
use dash_sdk::platform::shielded::nullifier_sync::{
    NullifierSyncCheckpoint, NullifierSyncConfig, NullifierSyncResult,
};
use dash_sdk::RequestSettings;
use tracing::{debug, error, info};

/// Synchronize nullifier statuses using privacy-preserving BLAST sync.
///
/// Discovers which of the supplied nullifiers have been spent in the shielded pool.
/// Uses trunk/branch chunk queries for privacy (hides which specific nullifiers
/// the wallet cares about).
///
/// # Parameters
/// - `sdk_handle`: SDK handle
/// - `nullifiers_ptr`: Contiguous array of 32-byte nullifier hashes
/// - `nullifiers_count`: Number of nullifiers (total bytes = count × 32)
/// - `config`: Optional sync config (null for defaults)
/// - `last_sync_height`: Height from previous sync (0 = full scan)
/// - `last_sync_timestamp`: Timestamp from previous sync (0 = full scan)
///
/// # Returns
/// Pointer to `DashSDKNullifierSyncResult`. Free with `dash_sdk_nullifier_sync_result_free`.
/// Returns null on error.
///
/// # Safety
/// - `sdk_handle` must be a valid SDK handle.
/// - `nullifiers_ptr` must point to `nullifiers_count × 32` valid bytes.
/// - `config` may be null (uses defaults).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_sync_nullifiers(
    sdk_handle: *const SDKHandle,
    nullifiers_ptr: *const u8,
    nullifiers_count: u32,
    config: *const DashSDKNullifierSyncConfig,
    last_sync_height: u64,
    last_sync_timestamp: u64,
) -> *mut DashSDKNullifierSyncResult {
    info!("dash_sdk_sync_nullifiers: called");

    if sdk_handle.is_null() {
        error!("dash_sdk_sync_nullifiers: SDK handle is null");
        return std::ptr::null_mut();
    }

    if nullifiers_ptr.is_null() || nullifiers_count == 0 {
        error!("dash_sdk_sync_nullifiers: nullifiers pointer is null or count is zero");
        return std::ptr::null_mut();
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    // Parse nullifiers from raw bytes
    let total_bytes = (nullifiers_count as usize) * 32;
    let raw_bytes = std::slice::from_raw_parts(nullifiers_ptr, total_bytes);
    let nullifiers: Vec<[u8; 32]> = raw_bytes
        .chunks_exact(32)
        .map(|chunk| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(chunk);
            arr
        })
        .collect();

    // Convert config
    let rust_config = if config.is_null() {
        None
    } else {
        let c = &*config;
        let pool_identifier = if c.has_pool_identifier {
            Some(c.pool_identifier)
        } else {
            None
        };
        Some(NullifierSyncConfig {
            min_privacy_count: c.min_privacy_count,
            max_concurrent_requests: c.max_concurrent_requests as usize,
            max_iterations: c.max_iterations as usize,
            pool_type: c.pool_type,
            pool_identifier,
            full_rescan_after_time_s: c.full_rescan_after_time_s,
            request_settings: RequestSettings::default(),
        })
    };

    // Convert checkpoint: 0 means no previous sync
    let last_sync = if last_sync_height == 0 && last_sync_timestamp == 0 {
        None
    } else {
        Some(NullifierSyncCheckpoint {
            height: last_sync_height,
            timestamp: last_sync_timestamp,
        })
    };

    debug!(
        "dash_sdk_sync_nullifiers: syncing {} nullifiers, checkpoint: {:?}",
        nullifiers_count, last_sync
    );

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_nullifiers(&nullifiers, rust_config, last_sync)
            .await
    });

    match result {
        Ok(sync_result) => {
            info!(
                "dash_sdk_sync_nullifiers: success - {} found, {} absent",
                sync_result.found.len(),
                sync_result.absent.len()
            );
            Box::into_raw(Box::new(convert_nullifier_sync_result(sync_result)))
        }
        Err(e) => {
            error!("dash_sdk_sync_nullifiers: error - {}", e);
            std::ptr::null_mut()
        }
    }
}

/// Synchronize nullifiers and return result via `DashSDKResult` for better error handling.
///
/// Same as `dash_sdk_sync_nullifiers` but returns errors via `DashSDKResult`.
///
/// # Safety
/// Same safety requirements as `dash_sdk_sync_nullifiers`.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_sync_nullifiers_with_result(
    sdk_handle: *const SDKHandle,
    nullifiers_ptr: *const u8,
    nullifiers_count: u32,
    config: *const DashSDKNullifierSyncConfig,
    last_sync_height: u64,
    last_sync_timestamp: u64,
) -> DashSDKResult {
    if sdk_handle.is_null() {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "SDK handle is null".to_string(),
        ));
    }

    if nullifiers_ptr.is_null() || nullifiers_count == 0 {
        return DashSDKResult::error(DashSDKError::new(
            DashSDKErrorCode::InvalidParameter,
            "Nullifiers pointer is null or count is zero".to_string(),
        ));
    }

    let wrapper = &*(sdk_handle as *const SDKWrapper);

    let total_bytes = (nullifiers_count as usize) * 32;
    let raw_bytes = std::slice::from_raw_parts(nullifiers_ptr, total_bytes);
    let nullifiers: Vec<[u8; 32]> = raw_bytes
        .chunks_exact(32)
        .map(|chunk| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(chunk);
            arr
        })
        .collect();

    let rust_config = if config.is_null() {
        None
    } else {
        let c = &*config;
        let pool_identifier = if c.has_pool_identifier {
            Some(c.pool_identifier)
        } else {
            None
        };
        Some(NullifierSyncConfig {
            min_privacy_count: c.min_privacy_count,
            max_concurrent_requests: c.max_concurrent_requests as usize,
            max_iterations: c.max_iterations as usize,
            pool_type: c.pool_type,
            pool_identifier,
            full_rescan_after_time_s: c.full_rescan_after_time_s,
            request_settings: RequestSettings::default(),
        })
    };

    let last_sync = if last_sync_height == 0 && last_sync_timestamp == 0 {
        None
    } else {
        Some(NullifierSyncCheckpoint {
            height: last_sync_height,
            timestamp: last_sync_timestamp,
        })
    };

    let result = wrapper.runtime.block_on(async {
        wrapper
            .sdk
            .sync_nullifiers(&nullifiers, rust_config, last_sync)
            .await
    });

    match result {
        Ok(sync_result) => {
            let ffi_result = Box::new(convert_nullifier_sync_result(sync_result));
            DashSDKResult::success(Box::into_raw(ffi_result) as *mut std::os::raw::c_void)
        }
        Err(e) => DashSDKResult::error(FFIError::SDKError(e).into()),
    }
}

/// Free a nullifier sync result.
///
/// # Safety
/// - `result` must be a valid pointer from `dash_sdk_sync_nullifiers` or null (no-op).
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_nullifier_sync_result_free(
    result: *mut DashSDKNullifierSyncResult,
) {
    if result.is_null() {
        return;
    }

    let result = Box::from_raw(result);

    if !result.found.is_null() && result.found_count > 0 {
        let len = (result.found_count as usize) * 32;
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(result.found, len));
    }

    if !result.absent.is_null() && result.absent_count > 0 {
        let len = (result.absent_count as usize) * 32;
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(result.absent, len));
    }
}

fn convert_nullifier_sync_result(result: NullifierSyncResult) -> DashSDKNullifierSyncResult {
    // Convert found nullifiers to contiguous byte array
    let found_count = result.found.len() as u32;
    let found_ptr = if result.found.is_empty() {
        std::ptr::null_mut()
    } else {
        let mut bytes: Vec<u8> = Vec::with_capacity(result.found.len() * 32);
        for key in &result.found {
            bytes.extend_from_slice(key);
        }
        let boxed = bytes.into_boxed_slice();
        Box::into_raw(boxed) as *mut u8
    };

    // Convert absent nullifiers to contiguous byte array
    let absent_count = result.absent.len() as u32;
    let absent_ptr = if result.absent.is_empty() {
        std::ptr::null_mut()
    } else {
        let mut bytes: Vec<u8> = Vec::with_capacity(result.absent.len() * 32);
        for key in &result.absent {
            bytes.extend_from_slice(key);
        }
        let boxed = bytes.into_boxed_slice();
        Box::into_raw(boxed) as *mut u8
    };

    let metrics = DashSDKNullifierSyncMetrics {
        trunk_queries: result.metrics.trunk_queries as u32,
        branch_queries: result.metrics.branch_queries as u32,
        total_elements_seen: result.metrics.total_elements_seen as u32,
        total_proof_bytes: result.metrics.total_proof_bytes as u32,
        branch_query_failures: result.metrics.branch_query_failures as u32,
        iterations: result.metrics.iterations as u32,
        compacted_queries: result.metrics.compacted_queries as u32,
        recent_queries: result.metrics.recent_queries as u32,
    };

    DashSDKNullifierSyncResult {
        found: found_ptr,
        found_count,
        absent: absent_ptr,
        absent_count,
        checkpoint_height: result.checkpoint_height,
        new_sync_height: result.new_sync_height,
        new_sync_timestamp: result.new_sync_timestamp,
        metrics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DashSDKNullifierSyncConfig::default();
        assert_eq!(config.min_privacy_count, 32);
        assert_eq!(config.max_concurrent_requests, 10);
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.pool_type, 0);
        assert!(!config.has_pool_identifier);
        assert_eq!(config.full_rescan_after_time_s, 7 * 24 * 60 * 60);
    }

    #[test]
    fn test_null_sync_nullifiers() {
        unsafe {
            let result = dash_sdk_sync_nullifiers(
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                0,
            );
            assert!(result.is_null());
        }
    }
}
