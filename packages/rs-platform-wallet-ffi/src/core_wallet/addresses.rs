//! FFI bindings for CoreWallet address derivation.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use std::ffi::CString;
use std::os::raw::c_char;

/// Get the next unused BIP-44 external (receive) address for a specific account.
///
/// On success, `out_address` is set to a heap-allocated C string.
/// Free with `core_wallet_free_address`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_next_receive_address(
    handle: Handle,
    account_index: u32,
    out_address: *mut *mut c_char,
) -> PlatformWalletFfiResult {
    check_ptr!(out_address);

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.next_receive_address_for_account(account_index))
    });
    let result = unwrap_option_or_return!(option);
    let addr = unwrap_result_or_return!(result);
    let c_str = unwrap_result_or_return!(CString::new(addr.to_string()));
    *out_address = c_str.into_raw();
    PlatformWalletFfiResult::ok()
}

/// Get the next unused BIP-44 internal (change) address for a specific account.
///
/// On success, `out_address` is set to a heap-allocated C string.
/// Free with `core_wallet_free_address`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_next_change_address(
    handle: Handle,
    account_index: u32,
    out_address: *mut *mut c_char,
) -> PlatformWalletFfiResult {
    check_ptr!(out_address);

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.next_change_address_for_account(account_index))
    });
    let result = unwrap_option_or_return!(option);
    let addr = unwrap_result_or_return!(result);
    let c_str = unwrap_result_or_return!(CString::new(addr.to_string()));
    *out_address = c_str.into_raw();
    PlatformWalletFfiResult::ok()
}

/// Free an address string returned by `core_wallet_next_receive_address`
/// or `core_wallet_next_change_address`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_free_address(address: *mut c_char) {
    if !address.is_null() {
        let _ = CString::from_raw(address);
    }
}
