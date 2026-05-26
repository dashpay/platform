//! FFI bindings for CoreWallet transaction building and broadcasting.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverCoreSigner;
use rs_sdk_ffi::MnemonicResolverHandle;
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

/// Build, sign, and broadcast a payment to the given addresses via
/// an external mnemonic-resolver-backed signer.
///
/// The Swift caller supplies a [`MnemonicResolverHandle`] — the same
/// vtable shape used by
/// [`crate::dash_sdk_sign_with_mnemonic_resolver_and_path`] — which
/// the FFI wraps in a [`MnemonicResolverCoreSigner`] for the
/// lifetime of this call. Private keys never cross the FFI as raw
/// bytes; every signature is produced inside the signer's atomic
/// derive-and-sign step.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`. Ownership is retained by the
///   caller — this function does NOT destroy it.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_send_to_addresses(
    handle: Handle,
    account_type: u32,
    account_index: u32,
    addresses: *const *const c_char,
    amounts: *const u64,
    count: usize,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_tx_bytes: *mut *mut u8,
    out_tx_len: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    if count > 0 {
        check_ptr!(addresses);
        check_ptr!(amounts);
    }
    check_ptr!(out_tx_bytes);
    check_ptr!(out_tx_len);

    // `std::slice::from_raw_parts` requires a non-null, properly
    // aligned pointer even for `len == 0`. Swift's empty
    // `Array.withUnsafeBufferPointer.baseAddress` returns `nil`, so
    // the `count == 0` path is allowed to ship null `addresses` /
    // `amounts` — guard against constructing the slice in that case.
    let outputs: Vec<(dashcore::Address, u64)> = if count == 0 {
        Vec::new()
    } else {
        let addr_ptrs = std::slice::from_raw_parts(addresses, count);
        let amount_slice = std::slice::from_raw_parts(amounts, count);

        let mut outputs = Vec::with_capacity(count);
        for i in 0..count {
            if addr_ptrs[i].is_null() {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorNullPointer,
                    format!("null address pointer at index {i}"),
                );
            }
            let c_str = unwrap_result_or_return!(std::ffi::CStr::from_ptr(addr_ptrs[i]).to_str());
            let addr =
                unwrap_result_or_return!(dashcore::Address::from_str(c_str)).assume_checked();
            outputs.push((addr, amount_slice[i]));
        }
        outputs
    };

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

    let signer_addr = core_signer_handle as usize;

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: the resolver handle is pinned alive for the duration
        // of this FFI call (see fn-level safety doc). The
        // `MnemonicResolverCoreSigner` lives on this stack frame and
        // is dropped before the function returns.
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        runtime().block_on(wallet.send_to_addresses(
            std_account_type,
            account_index,
            outputs,
            &signer,
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::NULL_HANDLE;

    /// `count == 0` MUST NOT touch `addresses` / `amounts` — Swift's
    /// empty `Array.withUnsafeBufferPointer.baseAddress` gives `nil`,
    /// and `slice::from_raw_parts` is UB on a null pointer regardless
    /// of length. Pass `NULL_HANDLE` so the storage lookup short-
    /// circuits to `NotFound` before any wallet code runs — we only
    /// care that input marshalling did not blow up.
    #[test]
    fn send_to_addresses_zero_count_null_pointers_is_safe() {
        // Use a non-null but fake signer pointer; the closure that
        // would dereference it is never entered because `NULL_HANDLE`
        // makes `with_item` return `None`.
        let fake_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let mut out_tx: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        let result = unsafe {
            core_wallet_send_to_addresses(
                NULL_HANDLE,
                0, // BIP44Account
                0,
                std::ptr::null(), // null addresses — allowed because count == 0
                std::ptr::null(), // null amounts — allowed because count == 0
                0,
                fake_signer,
                &mut out_tx,
                &mut out_len,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// Non-null `addresses` array with `count == 0` is also fine.
    #[test]
    fn send_to_addresses_zero_count_nonnull_pointers_is_safe() {
        let dummy_addr: *const c_char = std::ptr::null();
        let dummy_amount: u64 = 0;
        let addrs: [*const c_char; 1] = [dummy_addr];
        let amts: [u64; 1] = [dummy_amount];
        let fake_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let mut out_tx: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        let result = unsafe {
            core_wallet_send_to_addresses(
                NULL_HANDLE,
                0,
                0,
                addrs.as_ptr(),
                amts.as_ptr(),
                0, // count = 0 → array contents ignored
                fake_signer,
                &mut out_tx,
                &mut out_len,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// A null entry inside the address array must surface as
    /// `ErrorNullPointer`, not UB.
    #[test]
    fn send_to_addresses_null_element_is_rejected() {
        let addrs: [*const c_char; 2] = [std::ptr::null(), std::ptr::null()];
        let amts: [u64; 2] = [1, 2];
        let fake_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let mut out_tx: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        let result = unsafe {
            core_wallet_send_to_addresses(
                NULL_HANDLE,
                0,
                0,
                addrs.as_ptr(),
                amts.as_ptr(),
                2,
                fake_signer,
                &mut out_tx,
                &mut out_len,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }
}
