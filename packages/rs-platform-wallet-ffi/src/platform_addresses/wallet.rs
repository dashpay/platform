//! Handle management, queries, and memory deallocation for PlatformAddressWallet.

use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;

use super::runtime;

// ---------------------------------------------------------------------------
// Handle management
// ---------------------------------------------------------------------------

/// Destroy a PlatformAddressWallet handle, releasing its resources.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_destroy(
    handle: Handle,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_ADDRESS_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::Success
}

/// Add a provider for a new account index.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_add_provider(
    handle: Handle,
    account_index: u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| match wallet.add_provider(account_index) {
            Ok(()) => PlatformWalletFFIResult::Success,
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorWalletOperation,
                        e.to_string(),
                    );
                }
                PlatformWalletFFIResult::ErrorWalletOperation
            }
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get total platform credits across all addresses.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_total_credits(
    handle: Handle,
    out_credits: *mut u64,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_credits.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let credits = runtime().block_on(wallet.total_credits());
            *out_credits = credits;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

/// Get all platform addresses with their cached balances.
///
/// On success, `out_entries` and `out_count` are set to a heap-allocated array.
/// Free with `platform_address_wallet_free_address_balances`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_addresses_with_balances(
    handle: Handle,
    out_entries: *mut *mut AddressBalanceEntryFFI,
    out_count: *mut usize,
    _out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_entries.is_null() || out_count.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| {
            let balances = runtime().block_on(wallet.addresses_with_balances());
            let entries: Vec<AddressBalanceEntryFFI> = balances
                .into_iter()
                .map(|(address, balance)| AddressBalanceEntryFFI {
                    address: address.into(),
                    balance,
                })
                .collect();
            *out_count = entries.len();
            if entries.is_empty() {
                *out_entries = std::ptr::null_mut();
            } else {
                *out_entries =
                    Box::into_raw(entries.into_boxed_slice()) as *mut AddressBalanceEntryFFI;
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or(PlatformWalletFFIResult::ErrorInvalidHandle)
}

// ---------------------------------------------------------------------------
// Memory deallocation
// ---------------------------------------------------------------------------

/// Free an address balances array returned by `platform_address_wallet_addresses_with_balances`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_address_balances(
    entries: *mut AddressBalanceEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
    }
}

/// Free a changeset returned by transfer/withdraw/fund/sync operations.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_changeset(
    changeset: PlatformAddressChangeSetFFI,
) {
    if !changeset.updated.is_null() && changeset.updated_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            changeset.updated,
            changeset.updated_count,
        ));
    }
    if !changeset.removed.is_null() && changeset.removed_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            changeset.removed,
            changeset.removed_count,
        ));
    }
}

/// Free a single sync result.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_sync_result(result: AddressSyncResultFFI) {
    if !result.found.is_null() && result.found_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.found,
            result.found_count,
        ));
    }
    if !result.absent.is_null() && result.absent_count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            result.absent,
            result.absent_count,
        ));
    }
}

/// Free a sync result array returned by `platform_address_wallet_sync_balances`.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_sync_result_array(
    array: AddressSyncResultArrayFFI,
) {
    if !array.results.is_null() && array.count > 0 {
        let results = std::slice::from_raw_parts(array.results, array.count);
        for result in results {
            if !result.found.is_null() && result.found_count > 0 {
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    result.found as *mut FoundAddressEntryFFI,
                    result.found_count,
                ));
            }
            if !result.absent.is_null() && result.absent_count > 0 {
                let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                    result.absent as *mut AbsentAddressEntryFFI,
                    result.absent_count,
                ));
            }
        }
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            array.results,
            array.count,
        ));
    }
}
