//! JNI bridge for consensus transaction decoding — a thin marshaler over
//! key-wallet-ffi's `transaction_decode` (single Rust FFI entry point, per
//! the `packages/kotlin-sdk/CLAUDE.md` boundary rule).
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.TxDecodeNative`,
//! driven by `org.dashfoundation.dashsdk.keywallet.TransactionDecoder` —
//! the Android analog of `SwiftDashSDK/KeyWallet/TransactionDecoder.swift`
//! (platform PR #3981).
//!
//! ## Result convention
//!
//! `DecodedTransactionFFI` is a pointer graph (per-input/per-output C
//! strings and script buffers); rather than exposing a native handle plus
//! N accessors across JNI, the whole decode result is copied ONCE into a
//! packed byte blob (the same big-endian BLOB convention as
//! `wallet_manager::walletAddressesWithBalances`) and every Rust
//! allocation is freed via `decoded_transaction_free` before the export
//! returns. Errors throw `DashSDKException` carrying the raw
//! key-wallet-ffi `FFIErrorCode` (same namespace `MnemonicNative
//! .generateMnemonic` already throws — `InvalidInput = 1` for malformed /
//! empty / trailing-garbage bytes).
//!
//! ## BLOB layout (big-endian; keep in sync with `TransactionDecoder.parseBlob`)
//!
//! ```text
//! u8[32] txid                    (consensus/internal byte order)
//! u32    input_count
//! repeat input_count times:
//!   u8[32] prev_txid             (consensus/internal byte order)
//!   u32    prev_vout
//!   u16    address_len           (0 = no recovered sender address)
//!   u8[address_len] address      (UTF-8 base58; unauthenticated P2PKH hint)
//! u32    output_count
//! repeat output_count times:
//!   u64    value_duffs
//!   u16    address_len           (0 = non-standard script, no address)
//!   u8[address_len] address      (UTF-8 base58)
//!   u32    script_len
//!   u8[script_len] script_pubkey
//! ```

use crate::support::{guard, throw_sdk_exception};
use dash_network::ffi::FFINetwork;
use jni::objects::{JByteArray, JClass};
use jni::sys::{jbyteArray, jint};
use jni::JNIEnv;
use key_wallet_ffi::tx_decode::{
    decoded_transaction_free, transaction_decode, DecodedTransactionFFI,
};
use std::ffi::CStr;

/// FFINetwork ordinal → enum (0=Mainnet, 2=Devnet, 3=Regtest, else
/// Testnet). Matches `transactions::net_from_ord` and Kotlin's
/// `Network.ffiValue`.
fn net_from_ord(ord: i32) -> FFINetwork {
    match ord {
        0 => FFINetwork::Mainnet,
        2 => FFINetwork::Devnet,
        3 => FFINetwork::Regtest,
        _ => FFINetwork::Testnet,
    }
}

/// Append `u16 len + bytes` for an optional C string (null → len 0).
/// An `Address` rendering is never empty, so 0 unambiguously means "none".
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string.
unsafe fn push_cstr_opt(blob: &mut Vec<u8>, ptr: *const std::os::raw::c_char) {
    if ptr.is_null() {
        blob.extend_from_slice(&0u16.to_be_bytes());
        return;
    }
    let bytes = CStr::from_ptr(ptr).to_bytes();
    // Addresses are ~34 chars; anything that would overflow u16 is not an
    // address — encode as absent rather than corrupting the layout.
    let Ok(len) = u16::try_from(bytes.len()) else {
        blob.extend_from_slice(&0u16.to_be_bytes());
        return;
    };
    blob.extend_from_slice(&len.to_be_bytes());
    blob.extend_from_slice(bytes);
}

/// Copy a `DecodedTransactionFFI` pointer graph into the packed blob.
///
/// # Safety
/// `decoded` must be a valid result of `transaction_decode` that has not
/// been freed.
unsafe fn encode_decoded_transaction(decoded: &DecodedTransactionFFI) -> Vec<u8> {
    let mut blob = Vec::with_capacity(256);
    blob.extend_from_slice(&decoded.txid);

    blob.extend_from_slice(&(decoded.inputs_count as u32).to_be_bytes());
    if !decoded.inputs.is_null() {
        let inputs = std::slice::from_raw_parts(decoded.inputs, decoded.inputs_count);
        for input in inputs {
            blob.extend_from_slice(&input.prev_txid);
            blob.extend_from_slice(&input.prev_vout.to_be_bytes());
            push_cstr_opt(&mut blob, input.address);
        }
    }

    blob.extend_from_slice(&(decoded.outputs_count as u32).to_be_bytes());
    if !decoded.outputs.is_null() {
        let outputs = std::slice::from_raw_parts(decoded.outputs, decoded.outputs_count);
        for output in outputs {
            blob.extend_from_slice(&output.value_duffs.to_be_bytes());
            push_cstr_opt(&mut blob, output.address);
            blob.extend_from_slice(&(output.script_pubkey_len as u32).to_be_bytes());
            if !output.script_pubkey.is_null() && output.script_pubkey_len > 0 {
                blob.extend_from_slice(std::slice::from_raw_parts(
                    output.script_pubkey,
                    output.script_pubkey_len,
                ));
            }
        }
    }
    blob
}

/// JNI-free core: consensus-decode `tx_bytes` and marshal the result into
/// the packed blob, freeing every Rust-side allocation before returning.
/// `Err((code, message))` carries the key-wallet-ffi `FFIErrorCode`.
fn decode_to_blob(tx_bytes: &[u8], network: FFINetwork) -> Result<Vec<u8>, (i32, String)> {
    // Out-param error slot; the FFI writes code+message into it.
    let mut error = key_wallet_ffi::FFIError {
        code: key_wallet_ffi::FFIErrorCode::Success,
        message: std::ptr::null_mut(),
    };
    let mut out: *mut DecodedTransactionFFI = std::ptr::null_mut();
    let ok = unsafe {
        transaction_decode(tx_bytes.as_ptr(), tx_bytes.len(), network, &mut out, &mut error)
    };
    if !ok || out.is_null() {
        let message = if error.message.is_null() {
            String::from("transaction decode failed")
        } else {
            unsafe { CStr::from_ptr(error.message) }
                .to_string_lossy()
                .into_owned()
        };
        // Capture the code BEFORE clean() — clean() resets it to Success
        // while freeing the message (same hazard as MnemonicNative).
        let code = error.code as i32;
        unsafe { error.clean() };
        return Err((code, message));
    }
    let blob = unsafe { encode_decoded_transaction(&*out) };
    unsafe { decoded_transaction_free(out) };
    Ok(blob)
}

/// Consensus-decode raw transaction bytes into the packed BLOB documented
/// in the module header. `networkOrd` is `Network.ffiValue`. Throws
/// `DashSDKException` (key-wallet-ffi code namespace; `InvalidInput = 1`)
/// on malformed bytes — including a valid transaction followed by trailing
/// garbage.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TxDecodeNative_decodeTransaction(
    mut env: JNIEnv,
    _class: JClass,
    tx_bytes: JByteArray,
    network_ord: jint,
) -> jbyteArray {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let bytes = match env.convert_byte_array(&tx_bytes) {
            Ok(b) => b,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "txBytes byte[] was null/invalid");
                return std::ptr::null_mut();
            }
        };
        match decode_to_blob(&bytes, net_from_ord(network_ord)) {
            Ok(blob) => env
                .byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(std::ptr::null_mut()),
            Err((code, message)) => {
                throw_sdk_exception(env, code, &message);
                std::ptr::null_mut()
            }
        }
    })
}

#[cfg(test)]
mod tests {
    //! Everything funnels through [`decode_to_blob`] — the exact code the
    //! JNI export runs minus the `JNIEnv` marshaling — so the blob bytes
    //! these tests pin are the bytes Kotlin's `TransactionDecoder
    //! .parseBlob` receives. `TransactionDecoderTest.kt` asserts the SAME
    //! fixture blob hex, cross-pinning the layout from both sides.

    use super::*;
    use dashcore::consensus::serialize;
    use dashcore::hashes::Hash;
    use dashcore::secp256k1::{Secp256k1, SecretKey};
    use dashcore::{
        Address, Network, OutPoint, PublicKey, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness,
    };

    /// Minimal big-endian blob reader mirroring the Kotlin parser.
    struct Reader<'a> {
        blob: &'a [u8],
        pos: usize,
    }

    impl<'a> Reader<'a> {
        fn take(&mut self, n: usize) -> &'a [u8] {
            let s = &self.blob[self.pos..self.pos + n];
            self.pos += n;
            s
        }
        fn u16(&mut self) -> u16 {
            u16::from_be_bytes(self.take(2).try_into().unwrap())
        }
        fn u32(&mut self) -> u32 {
            u32::from_be_bytes(self.take(4).try_into().unwrap())
        }
        fn u64(&mut self) -> u64 {
            u64::from_be_bytes(self.take(8).try_into().unwrap())
        }
        fn str_opt(&mut self) -> Option<String> {
            let len = self.u16() as usize;
            if len == 0 {
                None
            } else {
                Some(String::from_utf8(self.take(len).to_vec()).unwrap())
            }
        }
    }

    fn test_pubkey() -> PublicKey {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0x42u8; 32]).expect("valid secret key");
        PublicKey::new(sk.public_key(&secp))
    }

    /// The same deterministic fixture as key-wallet-ffi's
    /// `p2pkh_spend_tx`: one P2PKH-shaped input spending 11..11:3, one
    /// P2PKH output of 151_072 duffs, one OP_RETURN output.
    fn p2pkh_spend_tx(network: Network) -> (Transaction, Address) {
        let pubkey = test_pubkey();
        let addr = Address::p2pkh(&pubkey, network);
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0x30u8; 71])
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
            output: vec![
                TxOut {
                    value: 151_072,
                    script_pubkey: addr.script_pubkey(),
                },
                TxOut {
                    value: 0,
                    script_pubkey: ScriptBuf::new_op_return(&[0xAAu8; 4]),
                },
            ],
            special_transaction_payload: None,
        };
        (tx, addr)
    }

    #[test]
    fn blob_roundtrips_the_fixture_transaction() {
        let (tx, addr) = p2pkh_spend_tx(Network::Testnet);
        let bytes = serialize(&tx);
        let blob = decode_to_blob(&bytes, FFINetwork::Testnet).expect("decode ok");

        let mut r = Reader { blob: &blob, pos: 0 };
        assert_eq!(r.take(32), tx.txid().to_byte_array());

        assert_eq!(r.u32(), 1, "one input");
        assert_eq!(r.take(32), [0x11u8; 32]);
        assert_eq!(r.u32(), 3, "prev vout");
        assert_eq!(r.str_opt().as_deref(), Some(addr.to_string().as_str()));

        assert_eq!(r.u32(), 2, "two outputs");
        assert_eq!(r.u64(), 151_072);
        assert_eq!(r.str_opt().as_deref(), Some(addr.to_string().as_str()));
        let script_len = r.u32() as usize;
        assert_eq!(r.take(script_len), addr.script_pubkey().as_bytes());

        assert_eq!(r.u64(), 0, "OP_RETURN value");
        assert_eq!(r.str_opt(), None, "OP_RETURN has no address");
        let op_return_len = r.u32() as usize;
        assert!(op_return_len > 0, "script bytes still present");
        r.take(op_return_len);

        assert_eq!(r.pos, blob.len(), "no trailing bytes");
    }

    #[test]
    fn fixture_blob_hex_is_pinned_for_kotlin() {
        // The exact bytes `TransactionDecoderTest.kt` parses — regenerate
        // BOTH sides together if the layout ever changes.
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let blob = decode_to_blob(&serialize(&tx), FFINetwork::Testnet).expect("decode ok");
        let hex: String = blob.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, FIXTURE_BLOB_HEX);
    }

    /// Shared fixture blob (see [`fixture_blob_hex_is_pinned_for_kotlin`]).
    const FIXTURE_BLOB_HEX: &str = "d5b51c39a335f82c33beee64bbcdf9c62418884c6511bf606fa75b6e217974bf000000011111111111111111111111111111111111111111111111111111111111111111000000030022794e446a323851424d6d35735936624c6a46634e6457524e656632344b4c514e7551000000020000000000024e200022794e446a323851424d6d35735936624c6a46634e6457524e656632344b4c514e75510000001976a91414db4138d56a2ecfb10881a9be394d9f321985b288ac00000000000000000000000000066a04aaaaaaaa";

    #[test]
    fn network_changes_rendered_addresses() {
        let (tx, addr) = p2pkh_spend_tx(Network::Testnet);
        let blob = decode_to_blob(&serialize(&tx), FFINetwork::Mainnet).expect("decode ok");
        let mut r = Reader { blob: &blob, pos: 0 };
        r.take(32);
        r.u32();
        r.take(36);
        let input_addr = r.str_opt().unwrap();
        assert_ne!(input_addr, addr.to_string());
        assert!(input_addr.starts_with('X'), "mainnet P2PKH starts with 'X'");
    }

    #[test]
    fn garbage_bytes_error_with_invalid_input() {
        let err = decode_to_blob(&[0xFFu8; 16], FFINetwork::Testnet).unwrap_err();
        assert_eq!(err.0, key_wallet_ffi::FFIErrorCode::InvalidInput as i32);
        assert!(!err.1.is_empty());
    }

    #[test]
    fn trailing_garbage_is_an_error() {
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let mut bytes = serialize(&tx);
        bytes.extend_from_slice(&[0xDE, 0xAD]);
        let err = decode_to_blob(&bytes, FFINetwork::Testnet).unwrap_err();
        assert_eq!(err.0, key_wallet_ffi::FFIErrorCode::InvalidInput as i32);
    }

    #[test]
    fn empty_bytes_error_with_invalid_input() {
        let err = decode_to_blob(&[], FFINetwork::Testnet).unwrap_err();
        assert_eq!(err.0, key_wallet_ffi::FFIErrorCode::InvalidInput as i32);
    }

    #[test]
    fn net_from_ord_matches_kotlin_ffi_values() {
        assert_eq!(net_from_ord(0), FFINetwork::Mainnet);
        assert_eq!(net_from_ord(1), FFINetwork::Testnet);
        assert_eq!(net_from_ord(2), FFINetwork::Devnet);
        assert_eq!(net_from_ord(3), FFINetwork::Regtest);
        assert_eq!(net_from_ord(-1), FFINetwork::Testnet, "unknown → Testnet");
    }
}
