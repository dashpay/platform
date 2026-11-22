//! FFI bindings for the balance change history of a wallet's platform addresses.

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Balance changes of the wallet's platform addresses in Platform's recent
/// (not yet compacted) zone, newest block first.
///
/// Calls `PlatformAddressWallet::recent_balance_changes`, which pages the
/// recent zone, keeps the changes of every platform payment account's
/// addresses and orders them.
///
/// On success `out_entries` / `out_count` hold a heap array (null with a
/// count of 0 when empty). Free with
/// `platform_address_wallet_free_recent_balance_changes`.
///
/// # Safety
/// `out_entries` and `out_count` must be valid, writable pointers.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_recent_balance_changes(
    handle: Handle,
    out_entries: *mut *mut AddressBalanceChangeEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null_mut();
    *out_count = 0;

    // Clone the wallet out of handle storage so the storage lock is released
    // before the network round-trips, and poll on a worker (8 MB stack):
    // every page is proof-verified, which recurses past the small stacks of
    // iOS dispatch threads (see runtime.rs).
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.clone());
    let wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move { wallet.recent_balance_changes().await });
    let entries = unwrap_result_or_return!(result);

    let entries: Vec<AddressBalanceChangeEntryFFI> = entries
        .into_iter()
        .map(AddressBalanceChangeEntryFFI::from)
        .collect();
    *out_count = entries.len();
    if !entries.is_empty() {
        *out_entries =
            Box::into_raw(entries.into_boxed_slice()) as *mut AddressBalanceChangeEntryFFI;
    }
    PlatformWalletFFIResult::ok()
}

/// Compacted balance changes of the wallet's platform addresses, newest range
/// first.
///
/// Calls `PlatformAddressWallet::compacted_balance_changes`, which pages every
/// compacted range Platform still holds, keeps the changes of every platform
/// payment account's addresses and orders them.
///
/// On success `out_entries` / `out_count` hold a heap array (null with a
/// count of 0 when empty). Free with
/// `platform_address_wallet_free_compacted_balance_changes`, which also frees
/// each entry's `additions`.
///
/// # Safety
/// `out_entries` and `out_count` must be valid, writable pointers.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_compacted_balance_changes(
    handle: Handle,
    out_entries: *mut *mut CompactedAddressBalanceChangeEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_entries);
    check_ptr!(out_count);
    *out_entries = std::ptr::null_mut();
    *out_count = 0;

    // Same worker polling as `platform_address_wallet_recent_balance_changes`.
    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| wallet.clone());
    let wallet = unwrap_option_or_return!(option);
    let result = block_on_worker(async move { wallet.compacted_balance_changes().await });
    let entries = unwrap_result_or_return!(result);

    let entries: Vec<CompactedAddressBalanceChangeEntryFFI> = entries
        .into_iter()
        .map(CompactedAddressBalanceChangeEntryFFI::from)
        .collect();
    *out_count = entries.len();
    if !entries.is_empty() {
        *out_entries =
            Box::into_raw(entries.into_boxed_slice()) as *mut CompactedAddressBalanceChangeEntryFFI;
    }
    PlatformWalletFFIResult::ok()
}

/// Free an array returned by `platform_address_wallet_recent_balance_changes`.
///
/// # Safety
/// `entries` / `count` must be exactly what that call returned, freed once.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_recent_balance_changes(
    entries: *mut AddressBalanceChangeEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        drop(Vec::from_raw_parts(entries, count, count));
    }
}

/// Free an array returned by `platform_address_wallet_compacted_balance_changes`,
/// including every entry's `additions`.
///
/// # Safety
/// `entries` / `count` must be exactly what that call returned, freed once.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_free_compacted_balance_changes(
    entries: *mut CompactedAddressBalanceChangeEntryFFI,
    count: usize,
) {
    if entries.is_null() || count == 0 {
        return;
    }
    let entries = Vec::from_raw_parts(entries, count, count);
    for entry in &entries {
        if !entry.additions.is_null() && entry.additions_count > 0 {
            drop(Vec::from_raw_parts(
                entry.additions,
                entry.additions_count,
                entry.additions_count,
            ));
        }
    }
    drop(entries);
}

#[cfg(test)]
mod tests {
    use dpp::address_funds::PlatformAddress;
    use platform_wallet::wallet::platform_addresses::{
        CompactedAddressBalanceChange, CompactedAddressBalanceChangeEntry,
    };

    use super::*;

    #[test]
    fn should_marshal_and_free_compacted_entries_with_additions() {
        let entries: Vec<CompactedAddressBalanceChangeEntryFFI> = vec![
            CompactedAddressBalanceChangeEntry {
                start_block_height: 6,
                end_block_height: 9,
                address: PlatformAddress::P2pkh([3; 20]),
                change: CompactedAddressBalanceChange::Additions(vec![(8, 20), (6, 10)]),
            },
            CompactedAddressBalanceChangeEntry {
                start_block_height: 1,
                end_block_height: 5,
                address: PlatformAddress::P2pkh([1; 20]),
                change: CompactedAddressBalanceChange::SetFinal(50),
            },
        ]
        .into_iter()
        .map(CompactedAddressBalanceChangeEntryFFI::from)
        .collect();

        assert_eq!(
            entries[0].kind,
            CompactedAddressBalanceChangeKindFFI::Additions
        );
        assert_eq!(entries[0].credits, 30);
        assert_eq!(entries[0].additions_count, 2);
        let additions =
            unsafe { std::slice::from_raw_parts(entries[0].additions, entries[0].additions_count) };
        assert_eq!(additions[0].block_height, 8);
        assert_eq!(additions[0].credits, 20);
        assert_eq!(
            entries[1].kind,
            CompactedAddressBalanceChangeKindFFI::SetFinal
        );
        assert_eq!(entries[1].credits, 50);
        assert!(entries[1].additions.is_null());
        assert_eq!(entries[1].additions_count, 0);

        let count = entries.len();
        let ptr =
            Box::into_raw(entries.into_boxed_slice()) as *mut CompactedAddressBalanceChangeEntryFFI;
        unsafe { platform_address_wallet_free_compacted_balance_changes(ptr, count) };
    }
}
