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
) -> PlatformWalletFfiResult {
    PLATFORM_ADDRESS_WALLET_STORAGE.remove(handle);
    PlatformWalletFfiResult::ok()
}

/// Add a provider for a new account index.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_add_provider(
    handle: Handle,
    account_index: u32,
) -> PlatformWalletFfiResult {
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.add_provider(account_index))
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFfiResult::ok()
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
) -> PlatformWalletFfiResult {
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.restore_sync_state(
            sync_height,
            sync_timestamp,
            last_known_recent_block,
        ));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFfiResult::ok()
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Get total platform credits across all addresses.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_total_credits(
    handle: Handle,
    out_credits: *mut u64,
) -> PlatformWalletFfiResult {
    check_ptr!(out_credits);

    let option = PLATFORM_ADDRESS_WALLET_STORAGE
        .with_item(handle, |wallet| runtime().block_on(wallet.total_credits()));
    *out_credits = unwrap_option_or_return!(option);
    PlatformWalletFfiResult::ok()
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
) -> PlatformWalletFfiResult {
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
    PlatformWalletFfiResult::ok()
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
