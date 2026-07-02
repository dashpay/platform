//! C-ABI marshalling for [`platform_wallet::decode_transaction`] — **thin FFI
//! surface only**. All decoding logic (consensus decode, address derivation,
//! the P2PKH sender-address heuristic) lives in the pure `platform-wallet`
//! crate and is unit-tested there; these entry points only cross the C
//! boundary: marshal bytes in, marshal the decoded structure out, and pair
//! every allocation with a free. Mirrors the delegation shape of
//! `mnemonic_words.rs` (which forwards to `key_wallet::Mnemonic`).
//!
//! `txid` / `prev_txid` are in consensus (internal) byte order — the same
//! convention as `OutPointFFI` — reverse for explorer-style display.

use std::ffi::CString;
use std::os::raw::c_char;

use dashcore::hashes::Hash;
use dashcore::Address;
use platform_wallet::decode_transaction;

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};

/// One decoded transaction input.
#[repr(C)]
pub struct DecodedTxInputFFI {
    /// Previous output's txid in consensus (internal) byte order.
    pub prev_txid: [u8; 32],
    /// Previous output's index.
    pub prev_vout: u32,
    /// Sender address recovered from a P2PKH scriptSig, or null when the
    /// input is coinbase / non-P2PKH / unparseable. Owned by the parent
    /// structure — freed by `platform_wallet_decoded_transaction_free`.
    pub address: *mut c_char,
}

/// One decoded transaction output.
#[repr(C)]
pub struct DecodedTxOutputFFI {
    /// Destination address for standard scripts (P2PKH / P2SH), or null for
    /// non-standard scripts. Owned by the parent structure.
    pub address: *mut c_char,
    /// Output value in duffs.
    pub value_duffs: u64,
    /// Raw scriptPubKey bytes (owned by the parent structure).
    pub script_pubkey: *mut u8,
    pub script_pubkey_len: usize,
}

/// A consensus-decoded transaction. Allocated by
/// `platform_wallet_decode_transaction`; release with
/// `platform_wallet_decoded_transaction_free`.
#[repr(C)]
pub struct DecodedTransactionFFI {
    /// Transaction id in consensus (internal) byte order.
    pub txid: [u8; 32],
    pub inputs: *mut DecodedTxInputFFI,
    pub inputs_count: usize,
    pub outputs: *mut DecodedTxOutputFFI,
    pub outputs_count: usize,
}

/// Boxed-slice allocator matching the crate's array-return convention.
fn vec_to_ptr<T>(v: Vec<T>) -> *mut T {
    if v.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(v.into_boxed_slice()) as *mut T
    }
}

/// Render an optional address to an owned C string, or null.
fn addr_to_cstr(address: Option<Address>) -> *mut c_char {
    address
        .and_then(|a| CString::new(a.to_string()).ok())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// Consensus-decode `tx_bytes` into a `DecodedTransactionFFI`.
///
/// # Safety
/// - `tx_bytes` must point to `tx_bytes_len` readable bytes.
/// - `out_decoded` must be a valid pointer; on success it receives an owned
///   `*mut DecodedTransactionFFI` that must be released with
///   `platform_wallet_decoded_transaction_free`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_decode_transaction(
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    network: FFINetwork,
    out_decoded: *mut *mut DecodedTransactionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(tx_bytes);
    check_ptr!(out_decoded);
    if tx_bytes_len == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "tx_bytes_len is zero",
        );
    }

    let network: Network = network.into();
    let slice = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let decoded = unwrap_result_or_return!(decode_transaction(slice, network));

    let inputs: Vec<DecodedTxInputFFI> = decoded
        .inputs
        .into_iter()
        .map(|input| DecodedTxInputFFI {
            prev_txid: input.previous_output.txid.to_byte_array(),
            prev_vout: input.previous_output.vout,
            address: addr_to_cstr(input.address),
        })
        .collect();

    let outputs: Vec<DecodedTxOutputFFI> = decoded
        .outputs
        .into_iter()
        .map(|output| {
            let script = output.script_pubkey.as_bytes().to_vec();
            let script_len = script.len();
            DecodedTxOutputFFI {
                address: addr_to_cstr(output.address),
                value_duffs: output.value_duffs,
                script_pubkey: vec_to_ptr(script),
                script_pubkey_len: script_len,
            }
        })
        .collect();

    let ffi = DecodedTransactionFFI {
        txid: decoded.txid.to_byte_array(),
        inputs_count: inputs.len(),
        inputs: vec_to_ptr(inputs),
        outputs_count: outputs.len(),
        outputs: vec_to_ptr(outputs),
    };

    *out_decoded = Box::into_raw(Box::new(ffi));
    PlatformWalletFFIResult::ok()
}

/// Release a `DecodedTransactionFFI` returned by
/// `platform_wallet_decode_transaction`. Safe to call with null.
///
/// # Safety
/// `decoded` must be null or a pointer previously returned by
/// `platform_wallet_decode_transaction` that has not been freed yet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_decoded_transaction_free(
    decoded: *mut DecodedTransactionFFI,
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
        drop(inputs);
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
        drop(outputs);
    }
}

#[cfg(test)]
mod tests {
    //! Boundary-only tests: the decode *logic* is covered in
    //! `platform_wallet::transaction_decode`. Here we only assert the C
    //! marshalling round-trips and that memory handling is sound.
    use super::*;
    use dashcore::consensus::serialize;
    use dashcore::secp256k1::{Secp256k1, SecretKey};
    use dashcore::{OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
    use std::ffi::CStr;

    fn synthetic_tx_bytes() -> Vec<u8> {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0x42u8; 32]).unwrap();
        let pubkey = dashcore::PublicKey::new(sk.public_key(&secp));
        let addr = Address::p2pkh(&pubkey, Network::Testnet);
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice(&[0x30u8; 71])
            .push_key(&pubkey)
            .into_script();
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([0x11u8; 32]),
                    vout: 3,
                },
                script_sig,
                sequence: 0xffffffff,
                witness: Witness::default(),
            }],
            output: vec![TxOut {
                value: 151_072,
                script_pubkey: addr.script_pubkey(),
            }],
            special_transaction_payload: None,
        };
        serialize(&tx)
    }

    #[test]
    fn marshals_decoded_transaction_across_the_boundary() {
        let bytes = synthetic_tx_bytes();
        unsafe {
            let mut out: *mut DecodedTransactionFFI = std::ptr::null_mut();
            let res = platform_wallet_decode_transaction(
                bytes.as_ptr(),
                bytes.len(),
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::Success);
            assert!(!out.is_null());

            let inputs = std::slice::from_raw_parts((*out).inputs, (*out).inputs_count);
            let outputs = std::slice::from_raw_parts((*out).outputs, (*out).outputs_count);
            assert_eq!((*out).inputs_count, 1);
            assert_eq!((*out).outputs_count, 1);
            assert_eq!(inputs[0].prev_vout, 3);
            assert!(!inputs[0].address.is_null());
            assert_eq!(outputs[0].value_duffs, 151_072);
            let addr = CStr::from_ptr(outputs[0].address).to_str().unwrap();
            assert!(addr.starts_with('y'));
            assert!(outputs[0].script_pubkey_len > 0);

            platform_wallet_decoded_transaction_free(out);
        }
    }

    #[test]
    fn rejects_null_and_empty_input() {
        unsafe {
            let mut out: *mut DecodedTransactionFFI = std::ptr::null_mut();
            let res = platform_wallet_decode_transaction(
                std::ptr::null(),
                4,
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);

            let bytes = [0u8; 4];
            let res = platform_wallet_decode_transaction(
                bytes.as_ptr(),
                0,
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
        }
    }

    #[test]
    fn free_null_is_safe() {
        unsafe { platform_wallet_decoded_transaction_free(std::ptr::null_mut()) }
    }
}
