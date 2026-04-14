//! FFI bindings for CoreWallet transaction building and broadcasting.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use std::os::raw::c_char;
use std::str::FromStr;

/// Broadcast a signed transaction to the network.
///
/// `tx_bytes` is the raw serialized transaction.
///
/// On success, `out_txid` is set to a heap-allocated hex string of the txid.
/// Free with `core_wallet_free_address`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_broadcast_transaction(
    handle: Handle,
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    out_txid: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if tx_bytes.is_null() || out_txid.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let bytes = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: dashcore::Transaction = match dashcore::consensus::deserialize(bytes) {
        Ok(tx) => tx,
        Err(e) => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorDeserialization,
                    format!("Failed to deserialize transaction: {}", e),
                );
            }
            return PlatformWalletFFIResult::ErrorDeserialization;
        }
    };

    CORE_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.broadcast_transaction(&tx)) {
                Ok(txid) => {
                    let txid_hex = txid.to_string();
                    match std::ffi::CString::new(txid_hex) {
                        Ok(c_str) => {
                            *out_txid = c_str.into_raw();
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

/// Build, sign, and broadcast a payment to the given addresses.
///
/// Uses key-wallet's TransactionBuilder for UTXO selection and signing.
///
/// `addresses` is an array of C strings (recipient addresses).
/// `amounts` is an array of u64 values (amounts in duffs).
/// Both arrays must have `count` elements.
///
/// On success, `out_tx_bytes` and `out_tx_len` are set to the serialized
/// signed transaction. Free with `core_wallet_free_tx_bytes`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_send_to_addresses(
    handle: Handle,
    account_type: u32,
    account_index: u32,
    addresses: *const *const c_char,
    amounts: *const u64,
    count: usize,
    out_tx_bytes: *mut *mut u8,
    out_tx_len: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if (addresses.is_null() || amounts.is_null()) && count > 0 {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    if out_tx_bytes.is_null() || out_tx_len.is_null() {
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    // Parse addresses and amounts into Vec<(Address, u64)>.
    let mut outputs = Vec::with_capacity(count);
    let addr_ptrs = std::slice::from_raw_parts(addresses, count);
    let amount_slice = std::slice::from_raw_parts(amounts, count);

    for i in 0..count {
        let c_str = match std::ffi::CStr::from_ptr(addr_ptrs[i]).to_str() {
            Ok(s) => s,
            Err(_) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorUtf8Conversion,
                        format!("Invalid UTF-8 in address at index {}", i),
                    );
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        };

        let addr = match dashcore::Address::from_str(c_str) {
            Ok(a) => a.assume_checked(),
            Err(e) => {
                if !out_error.is_null() {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidParameter,
                        format!("Invalid address at index {}: {}", i, e),
                    );
                }
                return PlatformWalletFFIResult::ErrorInvalidParameter;
            }
        };

        outputs.push((addr, amount_slice[i]));
    }

    use key_wallet::account::account_type::StandardAccountType;
    let std_account_type = match account_type {
        0 => StandardAccountType::BIP44Account,
        1 => StandardAccountType::BIP32Account,
        _ => {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidParameter,
                    format!("Unknown account type: {}", account_type),
                );
            }
            return PlatformWalletFFIResult::ErrorInvalidParameter;
        }
    };

    CORE_WALLET_STORAGE
        .with_item(handle, |wallet| {
            match runtime().block_on(wallet.send_to_addresses(
                std_account_type,
                account_index,
                outputs,
            )) {
                Ok(tx) => {
                    let serialized = dashcore::consensus::serialize(&tx);
                    let len = serialized.len();
                    let boxed = serialized.into_boxed_slice();
                    *out_tx_bytes = Box::into_raw(boxed) as *mut u8;
                    *out_tx_len = len;
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

/// Free transaction bytes returned by `core_wallet_send_to_addresses`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_free_tx_bytes(bytes: *mut u8, len: usize) {
    if !bytes.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, len));
    }
}
