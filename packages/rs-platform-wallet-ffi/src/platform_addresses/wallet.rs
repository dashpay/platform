//! Handle management, queries, and memory deallocation for PlatformAddressWallet.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

use super::runtime;

// ---------------------------------------------------------------------------
// Handle management
// ---------------------------------------------------------------------------

/// Destroy a PlatformAddressWallet handle, releasing its resources.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_destroy(
    handle: Handle,
) -> PlatformWalletFFIResult {
    PLATFORM_ADDRESS_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::ok()
}

/// Add a provider for a new account index.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_add_provider(
    handle: Handle,
    account_index: u32,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.add_provider(account_index))
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Restore sync state from persisted values.
///
/// Call after wallet creation and before the first sync to resume
/// incremental mode. Without this, every app launch does a full
/// trunk/branch/compact rescan.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_restore_sync_state(
    handle: Handle,
    sync_height: u64,
    sync_timestamp: u64,
    last_known_recent_block: u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.restore_sync_state(
            sync_height,
            sync_timestamp,
            last_known_recent_block,
        ));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get total platform credits across all addresses.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_total_credits(
    handle: Handle,
    out_credits: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_credits);

    let option = PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| runtime().block_on(wallet.total_credits()));
    *out_credits = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get the per-input minimum credit amount (`min_input_amount`) the
/// chain enforces for address-funds transitions, read from the wallet's
/// current platform version.
///
/// Pure getter: resolve the handle, read
/// `PlatformAddressWallet::min_input_amount()` (which reads the constant
/// off the wallet's SDK-resolved `PlatformVersion`), write it to
/// `out_min_input_amount`. This is the same floor the transfer/withdraw
/// auto-selectors use to drop sub-minimum dust inputs, so a UI gate that
/// sums only balances ≥ this stays in step with what Rust will spend.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_min_input_amount(
    handle: Handle,
    out_min_input_amount: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_min_input_amount);

    let option =
        PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.min_input_amount());
    *out_min_input_amount = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get the per-output minimum credit amount (`min_output_amount`) the
/// chain enforces for address-funds transitions, read from the wallet's
/// current platform version.
///
/// Pure getter: resolve the handle, read
/// `PlatformAddressWallet::min_output_amount()` (which reads the constant
/// off the wallet's SDK-resolved `PlatformVersion`), write it to
/// `out_min_output_amount`. DPP rejects any address-funds output below
/// this floor, so a transfer UI gate that requires the requested amount to
/// reach it stays in step with what DPP will accept.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_min_output_amount(
    handle: Handle,
    out_min_output_amount: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_min_output_amount);

    let option =
        PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.min_output_amount());
    *out_min_output_amount = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
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
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        let balances = runtime().block_on(wallet.addresses_with_balances());
        balances
            .into_iter()
            .map(|(address, balance)| AddressBalanceEntryFFI {
                address: address.into(),
                balance,
                nonce: 0,
                account_index: 0,
                address_index: 0,
                as_of_height: 0,
            })
            .collect::<Vec<_>>()
    });
    let entries = unwrap_option_or_return!(option);
    *out_count = entries.len();
    if entries.is_empty() {
        *out_entries = std::ptr::null_mut();
    } else {
        *out_entries = Box::into_raw(entries.into_boxed_slice()) as *mut AddressBalanceEntryFFI;
    }
    PlatformWalletFFIResult::ok()
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
        drop(Vec::from_raw_parts(entries, count, count));
    }
}

/// Free a changeset returned by transfer/withdraw/fund operations.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_changeset(
    changeset: *const PlatformAddressChangeSetFFI,
) {
    if changeset.is_null() {
        return;
    }
    let cs = &*changeset;
    if !cs.updated.is_null() && cs.updated_count > 0 {
        drop(Vec::from_raw_parts(
            cs.updated,
            cs.updated_count,
            cs.updated_count,
        ));
    }
}

/// Free a single sync result.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_sync_result(
    result: *const AddressSyncResultFFI,
) {
    if result.is_null() {
        return;
    }
    let r = &*result;
    if !r.found.is_null() && r.found_count > 0 {
        drop(Vec::from_raw_parts(r.found, r.found_count, r.found_count));
    }
    if !r.absent.is_null() && r.absent_count > 0 {
        drop(Vec::from_raw_parts(
            r.absent,
            r.absent_count,
            r.absent_count,
        ));
    }
}
