//! FFI bindings for CoreWallet address derivation.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
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
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_address.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    CORE_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.next_receive_address_for_account(account_index)) {
                Ok(addr) => {
                    let addr_str = addr.to_string();
                    match CString::new(addr_str) {
                        Ok(c_str) => {
                            *out_address = c_str.into_raw();
                            PlatformWalletFFIResult::Success
                        }
                        Err(_) => PlatformWalletFFIResult::ErrorUtf8Conversion,
                    }
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

/// Get the next unused BIP-44 internal (change) address for a specific account.
///
/// On success, `out_address` is set to a heap-allocated C string.
/// Free with `core_wallet_free_address`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_next_change_address(
    handle: Handle,
    account_index: u32,
    out_address: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_address.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    CORE_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.next_change_address_for_account(account_index)) {
                Ok(addr) => {
                    let addr_str = addr.to_string();
                    match CString::new(addr_str) {
                        Ok(c_str) => {
                            *out_address = c_str.into_raw();
                            PlatformWalletFFIResult::Success
                        }
                        Err(_) => PlatformWalletFFIResult::ErrorUtf8Conversion,
                    }
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

/// Free an address string returned by `core_wallet_next_receive_address`
/// or `core_wallet_next_change_address`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_free_address(address: *mut c_char) {
    if !address.is_null() {
        let _ = CString::from_raw(address);
    }
}
