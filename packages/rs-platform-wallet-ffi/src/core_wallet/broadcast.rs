//! FFI bindings for CoreWallet transaction building and broadcasting.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use std::os::raw::c_char;
use std::str::FromStr;

/// Broadcast a signed transaction to the network.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_broadcast_transaction(
    handle: Handle,
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    out_txid: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(tx_bytes);
    check_ptr!(out_txid);

    let bytes = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: dashcore::Transaction =
        unwrap_result_or_return!(dashcore::consensus::deserialize(bytes));

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.broadcast_transaction(&tx))
    });
    let result = unwrap_option_or_return!(option);
    let txid = unwrap_result_or_return!(result);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(txid.to_string()));
    *out_txid = c_str.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Build, sign, and broadcast a payment to the given addresses.
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
) -> PlatformWalletFFIResult {
    if count > 0 {
        check_ptr!(addresses);
        check_ptr!(amounts);
    }
    check_ptr!(out_tx_bytes);
    check_ptr!(out_tx_len);

    let mut outputs = Vec::with_capacity(count);
    let addr_ptrs = std::slice::from_raw_parts(addresses, count);
    let amount_slice = std::slice::from_raw_parts(amounts, count);

    for i in 0..count {
        let c_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(addr_ptrs[i]).to_str());

        let addr = unwrap_result_or_return!(dashcore::Address::from_str(c_str)).assume_checked();

        outputs.push((addr, amount_slice[i]));
    }

    use key_wallet::account::account_type::StandardAccountType;
    let std_account_type = match account_type {
        0 => StandardAccountType::BIP44Account,
        1 => StandardAccountType::BIP32Account,
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("Unknown account type: {account_type}"),
            );
        }
    };

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.send_to_addresses(std_account_type, account_index, outputs))
    });
    let result = unwrap_option_or_return!(option);
    let tx = unwrap_result_or_return!(result);
    let serialized = dashcore::consensus::serialize(&tx);
    let len = serialized.len();
    let boxed = serialized.into_boxed_slice();
    *out_tx_bytes = Box::into_raw(boxed) as *mut u8;
    *out_tx_len = len;
    PlatformWalletFFIResult::ok()
}

/// Free transaction bytes returned by `core_wallet_send_to_addresses`.
#[no_mangle]
pub unsafe extern "C" fn core_wallet_free_tx_bytes(bytes: *mut u8, len: usize) {
    if !bytes.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, len));
    }
}
