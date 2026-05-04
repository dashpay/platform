//! FFI bindings for platform address sync operations.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

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
) -> PlatformWalletFFIResult {
    check_ptr!(out_result);

    let config_opt = if has_config && !config.is_null() {
        Some(dash_sdk::platform::address_sync::AddressSyncConfig::from(
            *config,
        ))
    } else {
        None
    };

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.sync_balances(config_opt))
    });
    let result = unwrap_option_or_return!(option);
    let sync = unwrap_result_or_return!(result);
    *out_result = AddressSyncResultFFI::from(&sync);
    PlatformWalletFFIResult::ok()
}
