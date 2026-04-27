//! FFI bindings for platform address sync operations.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;

use super::runtime;

/// Sync platform address balances across every platform payment account
/// on the wallet in a single trunk/branch scan.
///
/// The changeset is persisted internally by the wallet. The returned
/// `AddressSyncResultFFI` aggregates results from every account — per-
/// account detail can be rebuilt by the caller using each found
/// address's derivation context.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_sync_balances(
    handle: Handle,
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
            match runtime().block_on(wallet.sync_balances(config_opt)) {
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
