//! FFI bindings for platform address sync operations.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;

use super::runtime;

/// Sync platform address balances across all accounts.
///
/// Changesets are persisted internally by the wallet.
/// Free results with `platform_address_wallet_free_sync_result_array`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_sync_balances(
    handle: Handle,
    has_config: bool,
    config: *const AddressSyncConfigFFI,
    out_results: *mut AddressSyncResultArrayFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_results.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let config_opt = if has_config && !config.is_null() {
        Some(dash_sdk::platform::address_sync::AddressSyncConfig::from(
            *config,
        ))
    } else {
        None
    };

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.sync_balances(config_opt)) {
                Ok(results) => {
                    let ffi_results: Vec<AddressSyncResultFFI> =
                        results.values().map(AddressSyncResultFFI::from).collect();
                    let count = ffi_results.len();
                    let ptr = if ffi_results.is_empty() {
                        std::ptr::null_mut()
                    } else {
                        Box::into_raw(ffi_results.into_boxed_slice()) as *mut AddressSyncResultFFI
                    };
                    *out_results = AddressSyncResultArrayFFI {
                        results: ptr,
                        count,
                    };
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Sync platform address balances for a single account.
///
/// Changesets are persisted internally by the wallet.
/// Free results with `platform_address_wallet_free_sync_result`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_sync_balances_on_account(
    handle: Handle,
    account_index: u32,
    has_config: bool,
    config: *const AddressSyncConfigFFI,
    out_result: *mut AddressSyncResultFFI,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_result.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let config_opt = if has_config && !config.is_null() {
        Some(dash_sdk::platform::address_sync::AddressSyncConfig::from(
            *config,
        ))
    } else {
        None
    };

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime()
                .block_on(wallet.sync_balances_on_account_index(account_index, config_opt))
            {
                Ok(result) => {
                    *out_result = AddressSyncResultFFI::from(&result);
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            e.to_string(),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}
