//! C-ABI wrapper over [`platform_wallet::decode_transaction`].
//!
//! `txid` and `prev_txid` are in consensus (internal) byte order. Reverse them
//! for explorer-style display.

use std::ffi::CString;
use std::os::raw::c_char;
use std::{ptr, slice};

use dashcore::hashes::Hash;
use platform_wallet::decode_transaction;

use crate::core_wallet_types::vec_to_ptr;
use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::manager_diagnostics::optional_into_raw;
use crate::types::FFINetwork;
use crate::{check_ptr, unwrap_result_or_return};

/// One decoded transaction input.
#[repr(C)]
pub struct PlatformWalletDecodedTxInputFFI {
    /// Previous output's txid in consensus (internal) byte order.
    pub prev_txid: [u8; 32],
    /// Previous output's index.
    pub prev_vout: u32,
    /// Unauthenticated sender address from a P2PKH-shaped scriptSig, or null.
    /// See [`platform_wallet::DecodedTxInput::sender_address`]. Owned by the
    /// parent structure.
    pub address: *mut c_char,
}

/// One decoded transaction output.
#[repr(C)]
pub struct PlatformWalletDecodedTxOutputFFI {
    /// Destination address for standard scripts (P2PKH, P2SH), or null for
    /// non-standard scripts. Owned by the parent structure.
    pub address: *mut c_char,
    /// Output value in duffs.
    pub value_duffs: u64,
    /// Raw scriptPubKey bytes. Owned by the parent structure.
    pub script_pubkey: *mut u8,
    pub script_pubkey_len: usize,
}

/// A consensus-decoded transaction. Allocated by
/// [`platform_wallet_transaction_decode`] and released with
/// [`platform_wallet_decoded_transaction_free`].
#[repr(C)]
pub struct PlatformWalletDecodedTransactionFFI {
    /// Transaction id in consensus (internal) byte order.
    pub txid: [u8; 32],
    pub inputs: *mut PlatformWalletDecodedTxInputFFI,
    pub inputs_count: usize,
    pub outputs: *mut PlatformWalletDecodedTxOutputFFI,
    pub outputs_count: usize,
}

/// Consensus-decodes `tx_bytes` into a [`PlatformWalletDecodedTransactionFFI`].
///
/// # Safety
/// - `tx_bytes` must point to `tx_bytes_len` readable bytes.
/// - `out_decoded` must point to writable memory for one pointer. It is
///   nulled on entry and, on success, receives an owned pointer that must be
///   released with [`platform_wallet_decoded_transaction_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_transaction_decode(
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    network: FFINetwork,
    out_decoded: *mut *mut PlatformWalletDecodedTransactionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_decoded);
    *out_decoded = ptr::null_mut();
    check_ptr!(tx_bytes);
    if tx_bytes_len == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "tx_bytes_len is zero",
        );
    }

    let bytes = slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let decoded = unwrap_result_or_return!(decode_transaction(bytes, network.into()));

    let inputs: Vec<PlatformWalletDecodedTxInputFFI> = decoded
        .inputs
        .into_iter()
        .map(|input| PlatformWalletDecodedTxInputFFI {
            prev_txid: input.previous_output.txid.to_byte_array(),
            prev_vout: input.previous_output.vout,
            address: optional_into_raw(input.sender_address.map(|a| a.to_string())),
        })
        .collect();

    let outputs: Vec<PlatformWalletDecodedTxOutputFFI> = decoded
        .outputs
        .into_iter()
        .map(|output| {
            let script = output.script_pubkey.into_bytes();
            let script_len = script.len();
            PlatformWalletDecodedTxOutputFFI {
                address: optional_into_raw(output.address.map(|a| a.to_string())),
                value_duffs: output.value,
                script_pubkey: vec_to_ptr(script),
                script_pubkey_len: script_len,
            }
        })
        .collect();

    *out_decoded = Box::into_raw(Box::new(PlatformWalletDecodedTransactionFFI {
        txid: decoded.txid.to_byte_array(),
        inputs_count: inputs.len(),
        inputs: vec_to_ptr(inputs),
        outputs_count: outputs.len(),
        outputs: vec_to_ptr(outputs),
    }));
    PlatformWalletFFIResult::ok()
}

/// Releases a [`PlatformWalletDecodedTransactionFFI`] returned by
/// [`platform_wallet_transaction_decode`]. Safe to call with null.
///
/// # Safety
/// `decoded` must be null or a pointer returned by
/// [`platform_wallet_transaction_decode`] that has not been freed yet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_decoded_transaction_free(
    decoded: *mut PlatformWalletDecodedTransactionFFI,
) {
    if decoded.is_null() {
        return;
    }
    let decoded = Box::from_raw(decoded);

    if !decoded.inputs.is_null() {
        let inputs =
            Vec::from_raw_parts(decoded.inputs, decoded.inputs_count, decoded.inputs_count);
        for input in &inputs {
            if !input.address.is_null() {
                drop(CString::from_raw(input.address));
            }
        }
    }

    if !decoded.outputs.is_null() {
        let outputs = Vec::from_raw_parts(
            decoded.outputs,
            decoded.outputs_count,
            decoded.outputs_count,
        );
        for output in &outputs {
            if !output.address.is_null() {
                drop(CString::from_raw(output.address));
            }
            if !output.script_pubkey.is_null() {
                drop(Vec::from_raw_parts(
                    output.script_pubkey,
                    output.script_pubkey_len,
                    output.script_pubkey_len,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! The decode rules are tested in `platform_wallet`. These pin the C
    //! layout, ownership and error codes.
    use super::*;
    use dashcore::consensus::serialize;
    use platform_wallet::test_support::p2pkh_spend_tx;
    use std::ffi::CStr;
    use std::ptr::NonNull;

    /// Runs a decode that must fail and returns its result code. The out
    /// parameter starts dangling to pin that a failure nulls it.
    fn decode_err(tx_bytes: *const u8, tx_bytes_len: usize) -> PlatformWalletFFIResultCode {
        unsafe {
            let mut out: *mut PlatformWalletDecodedTransactionFFI = NonNull::dangling().as_ptr();
            let result = platform_wallet_transaction_decode(
                tx_bytes,
                tx_bytes_len,
                FFINetwork::Testnet,
                &mut out,
            );
            assert!(out.is_null(), "out param is nulled on failure");
            result.code
        }
    }

    #[test]
    fn should_marshal_a_decoded_transaction_across_the_boundary() {
        let (tx, _) = p2pkh_spend_tx();
        let bytes = serialize(&tx);
        unsafe {
            let mut out: *mut PlatformWalletDecodedTransactionFFI = ptr::null_mut();
            let result = platform_wallet_transaction_decode(
                bytes.as_ptr(),
                bytes.len(),
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!out.is_null());

            assert_eq!((*out).txid, tx.txid().to_byte_array());
            assert_eq!((*out).inputs_count, 1);
            assert_eq!((*out).outputs_count, 2);
            let inputs = slice::from_raw_parts((*out).inputs, (*out).inputs_count);
            let outputs = slice::from_raw_parts((*out).outputs, (*out).outputs_count);
            let spent = tx.input[0].previous_output;
            assert_eq!(inputs[0].prev_txid, spent.txid.to_byte_array());
            assert_eq!(inputs[0].prev_vout, spent.vout);
            assert!(!inputs[0].address.is_null());
            assert_eq!(outputs[0].value_duffs, 151_072);
            assert!(CStr::from_ptr(outputs[0].address)
                .to_str()
                .unwrap()
                .starts_with('y'));
            assert_eq!(
                slice::from_raw_parts(outputs[0].script_pubkey, outputs[0].script_pubkey_len),
                tx.output[0].script_pubkey.as_bytes()
            );
            assert!(outputs[1].address.is_null(), "OP_RETURN has no address");
            assert!(outputs[1].script_pubkey_len > 0);

            platform_wallet_decoded_transaction_free(out);
        }
    }

    #[test]
    fn should_reject_invalid_input() {
        let mut trailing = serialize(&p2pkh_spend_tx().0);
        trailing.extend_from_slice(&[0xDE, 0xAD]);
        assert_eq!(
            decode_err(trailing.as_ptr(), trailing.len()),
            PlatformWalletFFIResultCode::ErrorDeserialization
        );

        let garbage = [0xFFu8; 16];
        assert_eq!(
            decode_err(garbage.as_ptr(), garbage.len()),
            PlatformWalletFFIResultCode::ErrorDeserialization
        );

        assert_eq!(
            decode_err(ptr::null(), 4),
            PlatformWalletFFIResultCode::ErrorNullPointer
        );
        assert_eq!(
            decode_err(garbage.as_ptr(), 0),
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );

        let result = unsafe {
            platform_wallet_transaction_decode(
                garbage.as_ptr(),
                garbage.len(),
                FFINetwork::Testnet,
                ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);

        unsafe { platform_wallet_decoded_transaction_free(ptr::null_mut()) }
    }
}
