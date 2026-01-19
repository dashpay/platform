//! Platform address sync FFI bindings
//!
//! This module provides FFI functions for synchronizing platform payment
//! address balances using the privacy-preserving sync protocol.

use crate::error::*;
use crate::handle::*;
use once_cell::sync::Lazy;
use platform_wallet::platform_wallet_info::sync_state::PlatformSyncState;
use platform_wallet::platform_wallet_info::PlatformSyncResult;
use rs_sdk_ffi::{SDKHandle, SDKWrapper};
use std::sync::Arc;

/// FFI-compatible platform sync state
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformSyncStateFFI {
    /// Timestamp of the last full sync (seconds since Unix epoch)
    pub last_full_sync_timestamp: u64,
    /// Platform block height at last checkpoint
    pub checkpoint_height: u64,
    /// Last terminal block processed
    pub last_terminal_block: u64,
    /// Highest address index found with a balance (u32::MAX if none)
    pub highest_found_index: u32,
}

impl From<&PlatformSyncState> for PlatformSyncStateFFI {
    fn from(state: &PlatformSyncState) -> Self {
        Self {
            last_full_sync_timestamp: state.last_full_sync_timestamp,
            checkpoint_height: state.checkpoint_height,
            last_terminal_block: state.last_terminal_block,
            highest_found_index: state.highest_found_index.unwrap_or(u32::MAX),
        }
    }
}

impl From<PlatformSyncStateFFI> for PlatformSyncState {
    fn from(ffi: PlatformSyncStateFFI) -> Self {
        Self {
            last_full_sync_timestamp: ffi.last_full_sync_timestamp,
            checkpoint_height: ffi.checkpoint_height,
            last_terminal_block: ffi.last_terminal_block,
            highest_found_index: if ffi.highest_found_index == u32::MAX {
                None
            } else {
                Some(ffi.highest_found_index)
            },
        }
    }
}

/// FFI-compatible platform sync result
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PlatformSyncResultFFI {
    /// Whether a full sync was performed
    pub full_sync_performed: bool,
    /// Checkpoint height from full sync (Platform block height)
    pub checkpoint_height: u64,
    /// Highest terminal block processed
    pub highest_terminal_block: u64,
    /// Total credits found across all addresses
    pub total_balance: u64,
    /// Number of addresses with non-zero balance
    pub funded_address_count: u32,
    /// Highest address index found with a balance (u32::MAX if none)
    pub highest_found_index: u32,
}

impl From<&PlatformSyncResult> for PlatformSyncResultFFI {
    fn from(result: &PlatformSyncResult) -> Self {
        Self {
            full_sync_performed: result.full_sync_performed,
            checkpoint_height: result.checkpoint_height,
            highest_terminal_block: result.highest_terminal_block,
            total_balance: result.total_balance,
            funded_address_count: result.funded_address_count as u32,
            highest_found_index: result.highest_found_index.unwrap_or(u32::MAX),
        }
    }
}

/// Storage for PlatformSyncState handles
pub static SYNC_STATE_STORAGE: Lazy<HandleStorage<PlatformSyncState>> =
    Lazy::new(HandleStorage::new);

/// Create a new PlatformSyncState
///
/// Returns a handle to the newly created sync state.
#[no_mangle]
pub extern "C" fn platform_sync_state_create(out_handle: *mut Handle) -> PlatformWalletFFIResult {
    if out_handle.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let state = PlatformSyncState::new();
    let handle = SYNC_STATE_STORAGE.insert(state);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Create a PlatformSyncState from FFI struct
///
/// This allows restoring state from persistent storage.
#[no_mangle]
pub extern "C" fn platform_sync_state_create_from_ffi(
    state: PlatformSyncStateFFI,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    if out_handle.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let rust_state: PlatformSyncState = state.into();
    let handle = SYNC_STATE_STORAGE.insert(rust_state);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Get the current state as an FFI struct
///
/// This allows saving state to persistent storage.
#[no_mangle]
pub extern "C" fn platform_sync_state_get(
    state_handle: Handle,
    out_state: *mut PlatformSyncStateFFI,
) -> PlatformWalletFFIResult {
    if out_state.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    SYNC_STATE_STORAGE
        .with_item(state_handle, |state| {
            unsafe { *out_state = PlatformSyncStateFFI::from(state) };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Destroy a PlatformSyncState
#[no_mangle]
pub extern "C" fn platform_sync_state_destroy(state_handle: Handle) -> PlatformWalletFFIResult {
    if SYNC_STATE_STORAGE.remove(state_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

/// Synchronize platform payment addresses
///
/// This performs privacy-preserving address balance synchronization using
/// addresses already generated in the account's address pool. No key material required.
///
/// 1. Full sync (if needed): trunk + branch queries with privacy protection
/// 2. Terminal sync: fetch balance changes after checkpoint
///
/// # Arguments
/// - `wallet_handle`: Handle to the PlatformWalletInfo
/// - `sdk_handle`: Handle to the Dash SDK (pointer from rs-sdk-ffi's dash_sdk_create)
/// - `state_handle`: Handle to the PlatformSyncState (will be updated)
/// - `out_result`: Output parameter for the sync result
/// - `out_error`: Output parameter for error details
///
/// # Returns
/// - `Success` on successful sync
/// - Error code on failure
#[no_mangle]
pub extern "C" fn platform_wallet_sync_addresses(
    wallet_handle: Handle,
    sdk_handle: *mut SDKHandle,
    state_handle: Handle,
    out_result: *mut PlatformSyncResultFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    // Validate required pointers
    if sdk_handle.is_null() || out_result.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided for required parameter",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Get SDK wrapper
    let wrapper = unsafe { &*(sdk_handle as *const SDKWrapper) };
    let sdk = Arc::new(wrapper.sdk.clone());

    // Get sync state
    let sync_state_result = SYNC_STATE_STORAGE.with_item(state_handle, |state| state.clone());
    let mut sync_state = match sync_state_result {
        Some(state) => state,
        None => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid sync state handle",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidHandle;
        }
    };

    // Execute sync in the SDK's runtime
    let result = wrapper.runtime.block_on(async {
        WALLET_INFO_STORAGE
            .with_item_mut(wallet_handle, |wallet_info| wallet_info.clone())
            .ok_or_else(|| {
                PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid wallet handle",
                )
            })
    });

    let mut wallet_info = match result {
        Ok(info) => info,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = e;
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidHandle;
        }
    };

    // Execute the sync (uses pre-generated addresses, no key material needed)
    let sync_result = wrapper.runtime.block_on(async {
        wallet_info
            .sync_platform_addresses(&sdk, &mut sync_state)
            .await
    });

    match sync_result {
        Ok(result) => {
            // Update the sync state in storage
            SYNC_STATE_STORAGE.with_item_mut(state_handle, |state| {
                *state = sync_state.clone();
            });

            // Update the wallet info in storage
            WALLET_INFO_STORAGE.with_item_mut(wallet_handle, |info| {
                *info = wallet_info.clone();
            });

            // Write result
            unsafe { *out_result = PlatformSyncResultFFI::from(&result) };
            PlatformWalletFFIResult::Success
        }
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        format!("Platform sync failed: {}", e),
                    );
                }
            }
            PlatformWalletFFIResult::ErrorWalletOperation
        }
    }
}

/// Check if a full sync is needed
///
/// Returns true if more than 6 days 20 hours have passed since the last full sync,
/// or if no full sync has ever been performed.
#[no_mangle]
pub extern "C" fn platform_sync_state_needs_full_sync(
    state_handle: Handle,
    out_needs_sync: *mut bool,
) -> PlatformWalletFFIResult {
    if out_needs_sync.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    SYNC_STATE_STORAGE
        .with_item(state_handle, |state| {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            unsafe { *out_needs_sync = state.needs_full_sync(current_time) };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_create_destroy() {
        let mut handle: Handle = NULL_HANDLE;
        let result = platform_sync_state_create(&mut handle);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(handle, NULL_HANDLE);

        let result = platform_sync_state_destroy(handle);
        assert_eq!(result, PlatformWalletFFIResult::Success);
    }

    #[test]
    fn test_sync_state_ffi_conversion() {
        let ffi_state = PlatformSyncStateFFI {
            last_full_sync_timestamp: 1234567890,
            checkpoint_height: 100,
            last_terminal_block: 150,
            highest_found_index: 5,
        };

        let mut handle: Handle = NULL_HANDLE;
        let result = platform_sync_state_create_from_ffi(ffi_state, &mut handle);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        let mut retrieved_state = PlatformSyncStateFFI {
            last_full_sync_timestamp: 0,
            checkpoint_height: 0,
            last_terminal_block: 0,
            highest_found_index: 0,
        };
        let result = platform_sync_state_get(handle, &mut retrieved_state);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(retrieved_state.last_full_sync_timestamp, 1234567890);
        assert_eq!(retrieved_state.checkpoint_height, 100);
        assert_eq!(retrieved_state.last_terminal_block, 150);
        assert_eq!(retrieved_state.highest_found_index, 5);

        platform_sync_state_destroy(handle);
    }

    #[test]
    fn test_sync_state_none_index() {
        let ffi_state = PlatformSyncStateFFI {
            last_full_sync_timestamp: 0,
            checkpoint_height: 0,
            last_terminal_block: 0,
            highest_found_index: u32::MAX, // Represents None
        };

        let rust_state: PlatformSyncState = ffi_state.into();
        assert!(rust_state.highest_found_index.is_none());

        let back_to_ffi = PlatformSyncStateFFI::from(&rust_state);
        assert_eq!(back_to_ffi.highest_found_index, u32::MAX);
    }
}
