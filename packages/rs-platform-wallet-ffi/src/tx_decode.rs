//! C-ABI transaction decoding — consensus-decode raw Dash transaction bytes
//! into per-input/per-output structures with derived addresses, in one call.
//!
//! Wallet persistence stores every transaction's raw bytes but only
//! materializes TXO rows for the wallet's *own* outputs, so callers that need
//! to match arbitrary transactions against external addresses (e.g. the
//! CrowdNode on-chain API: "did a tx pay amount X to address Y?") need
//! per-output `(address, amount)` reconstructed from the stored bytes. This
//! entry point decodes straight into [`dashcore::Transaction`] and marshals
//! that into the C structures below — no intermediate representation.
//!
//! - Output `address` comes from [`Address::from_script`]; null for
//!   non-standard scripts (OP_RETURN, bare multisig, …).
//! - Input `address` is recovered only for P2PKH-shaped scriptSigs and is
//!   **unauthenticated** — see [`DecodedTxInputFFI::address`].
//! - `txid` / `prev_txid` are in consensus (internal) byte order — the same
//!   convention as `OutPointFFI` — reverse for explorer-style display.

use std::ffi::CString;
use std::os::raw::c_char;

use dashcore::blockdata::script::Instruction;
use dashcore::consensus::deserialize;
use dashcore::hashes::Hash;
use dashcore::{Address, PublicKey, ScriptBuf, Transaction};

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
    /// Sender address recovered from a P2PKH-shaped scriptSig (exactly two
    /// pushes: DER signature, then public key), or null when the input is
    /// coinbase / non-P2PKH / unparseable. **Unauthenticated**: the value is
    /// derived from a script push the spender fully controls, with no
    /// signature verification against the spent UTXO — use it for display or
    /// as a matching hint only, never for authentication or authorization.
    /// The attacker-resistant matching primitive is the per-output
    /// `(address, value_duffs)` pair. Owned by the parent structure — freed
    /// by `platform_wallet_decoded_transaction_free`.
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

/// Best-effort P2PKH sender address from a scriptSig. Requires the exact
/// P2PKH spend shape — two data pushes and nothing else: a DER-shaped ECDSA
/// signature (`0x30` tag, ≤ 73 bytes including the sighash byte) followed by
/// a 33/65-byte public key — so a P2SH redeem script or any other
/// last-push-happens-to-parse collision returns `None` rather than an
/// unrelated address. Nothing here verifies the signature against the spent
/// UTXO, so even a `Some` result is only the *claimed* sender — see
/// [`DecodedTxInputFFI::address`].
fn input_address_from_script_sig(script_sig: &ScriptBuf, network: Network) -> Option<Address> {
    let mut sig: Option<&[u8]> = None;
    let mut pubkey_bytes: Option<&[u8]> = None;
    for instruction in script_sig.instructions() {
        match instruction {
            Ok(Instruction::PushBytes(bytes)) => match (sig, pubkey_bytes) {
                (None, None) => sig = Some(bytes.as_bytes()),
                (Some(_), None) => pubkey_bytes = Some(bytes.as_bytes()),
                _ => return None, // more than two pushes
            },
            _ => return None, // non-push opcode or unparseable script
        }
    }
    let (sig, pubkey_bytes) = (sig?, pubkey_bytes?);
    if sig.is_empty() || sig[0] != 0x30 || sig.len() > 73 {
        return None;
    }
    if pubkey_bytes.len() != 33 && pubkey_bytes.len() != 65 {
        return None;
    }
    let pubkey = PublicKey::from_slice(pubkey_bytes).ok()?;
    Some(Address::p2pkh(&pubkey, network))
}

/// Consensus-decode `tx_bytes` into a `DecodedTransactionFFI`. Rejects
/// trailing bytes after a valid transaction (uses [`deserialize`], not a
/// streaming decode), so a valid prefix followed by garbage is an error
/// rather than a silent trim.
///
/// # Safety
/// - `tx_bytes` must point to `tx_bytes_len` readable bytes.
/// - `out_decoded` must be a valid pointer; it is nulled on entry and, on
///   success, receives an owned `*mut DecodedTransactionFFI` that must be
///   released with `platform_wallet_decoded_transaction_free`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_decode_transaction(
    tx_bytes: *const u8,
    tx_bytes_len: usize,
    network: FFINetwork,
    out_decoded: *mut *mut DecodedTransactionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_decoded);
    *out_decoded = std::ptr::null_mut();
    check_ptr!(tx_bytes);
    if tx_bytes_len == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "tx_bytes_len is zero",
        );
    }

    let network: Network = network.into();
    let slice = std::slice::from_raw_parts(tx_bytes, tx_bytes_len);
    let tx: Transaction = unwrap_result_or_return!(deserialize(slice));
    let txid = tx.txid(); // before the input/output vectors are consumed

    let inputs: Vec<DecodedTxInputFFI> = tx
        .input
        .into_iter()
        .map(|txin| {
            let address = if txin.previous_output.is_null() {
                None // coinbase
            } else {
                input_address_from_script_sig(&txin.script_sig, network)
            };
            DecodedTxInputFFI {
                prev_txid: txin.previous_output.txid.to_byte_array(),
                prev_vout: txin.previous_output.vout,
                address: addr_to_cstr(address),
            }
        })
        .collect();

    let outputs: Vec<DecodedTxOutputFFI> = tx
        .output
        .into_iter()
        .map(|txout| {
            let address = Address::from_script(&txout.script_pubkey, network).ok();
            let script = txout.script_pubkey.into_bytes();
            let script_len = script.len();
            DecodedTxOutputFFI {
                address: addr_to_cstr(address),
                value_duffs: txout.value,
                script_pubkey: vec_to_ptr(script),
                script_pubkey_len: script_len,
            }
        })
        .collect();

    let ffi = DecodedTransactionFFI {
        txid: txid.to_byte_array(),
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
    //! Everything funnels through the public FFI entry point: fixture →
    //! serialize → `platform_wallet_decode_transaction` → plain-Rust copy →
    //! assert. `decode_ok` owns the pointer walking and frees exactly once.
    //! Fixtures use `dashcore::test_utils` (`Address::dummy`,
    //! `Transaction::dummy`, `Transaction::dummy_coinbase`) where they fit;
    //! test_utils has no builder for a P2PKH-shaped scriptSig
    //! (signature + pubkey pushes), so that fixture stays hand-built.
    use super::*;
    use dashcore::consensus::serialize;
    use dashcore::secp256k1::{Secp256k1, SecretKey};
    use dashcore::{OutPoint, TxIn, TxOut, Txid, Witness};
    use std::ffi::CStr;

    /// FFI result copied into plain Rust so assertions read naturally.
    struct Decoded {
        txid: [u8; 32],
        /// `(prev_txid, prev_vout, sender address)`
        inputs: Vec<([u8; 32], u32, Option<String>)>,
        /// `(address, value_duffs, script_pubkey)`
        outputs: Vec<(Option<String>, u64, Vec<u8>)>,
    }

    /// Copy an optional C string into owned Rust.
    unsafe fn cstr_opt(ptr: *mut c_char) -> Option<String> {
        if ptr.is_null() {
            None
        } else {
            Some(CStr::from_ptr(ptr).to_str().unwrap().to_owned())
        }
    }

    /// Decode `tx` through the FFI boundary, copy the result out, and free
    /// the FFI allocation exactly once.
    fn decode_ok(tx: &Transaction, network: FFINetwork) -> Decoded {
        let bytes = serialize(tx);
        unsafe {
            let mut out: *mut DecodedTransactionFFI = std::ptr::null_mut();
            let res =
                platform_wallet_decode_transaction(bytes.as_ptr(), bytes.len(), network, &mut out);
            assert_eq!(res.code, PlatformWalletFFIResultCode::Success);
            assert!(!out.is_null());

            let inputs = if (*out).inputs.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts((*out).inputs, (*out).inputs_count)
                    .iter()
                    .map(|i| (i.prev_txid, i.prev_vout, cstr_opt(i.address)))
                    .collect()
            };
            let outputs = if (*out).outputs.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts((*out).outputs, (*out).outputs_count)
                    .iter()
                    .map(|o| {
                        let script = if o.script_pubkey.is_null() {
                            Vec::new()
                        } else {
                            std::slice::from_raw_parts(o.script_pubkey, o.script_pubkey_len)
                                .to_vec()
                        };
                        (cstr_opt(o.address), o.value_duffs, script)
                    })
                    .collect()
            };
            let copied = Decoded {
                txid: (*out).txid,
                inputs,
                outputs,
            };
            platform_wallet_decoded_transaction_free(out);
            copied
        }
    }

    fn test_pubkey() -> PublicKey {
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[0x42u8; 32]).expect("valid secret key");
        PublicKey::new(sk.public_key(&secp))
    }

    /// One P2PKH-shaped input (spending 11..11:3), one P2PKH output of
    /// 151072 duffs, one OP_RETURN output. The 151072 amount mirrors a
    /// CrowdNode signUp signal so the fixture doubles as documentation.
    fn p2pkh_spend_tx(network: Network) -> (Transaction, Address) {
        let pubkey = test_pubkey();
        let addr = Address::p2pkh(&pubkey, network);
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0x30u8; 71]) // DER-shaped: 0x30 tag, ≤ 73 bytes
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

    /// A one-input `Transaction::dummy` with its scriptSig replaced.
    fn tx_with_script_sig(script_sig: ScriptBuf) -> Transaction {
        let addr = Address::dummy(Network::Testnet, 1);
        let mut tx = Transaction::dummy(&addr, 1..2, &[9_000]);
        tx.input[0].script_sig = script_sig;
        tx
    }

    #[test]
    fn decodes_outputs_with_addresses_and_values() {
        let addr = Address::dummy(Network::Testnet, 7);
        let tx = Transaction::dummy(&addr, 1..2, &[151_072, 20_002]);
        let decoded = decode_ok(&tx, FFINetwork::Testnet);

        assert_eq!(decoded.txid, tx.txid().to_byte_array());
        assert_eq!(decoded.outputs.len(), 2);
        assert_eq!(decoded.outputs[0].0, Some(addr.to_string()));
        assert_eq!(decoded.outputs[0].1, 151_072);
        assert_eq!(decoded.outputs[0].2, addr.script_pubkey().into_bytes());
        assert_eq!(decoded.outputs[1].0, Some(addr.to_string()));
        assert_eq!(decoded.outputs[1].1, 20_002);
        assert!(
            addr.to_string().starts_with('y'),
            "testnet P2PKH starts with 'y'"
        );
    }

    #[test]
    fn op_return_output_has_no_address() {
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let decoded = decode_ok(&tx, FFINetwork::Testnet);
        assert!(decoded.outputs[1].0.is_none());
        assert!(
            !decoded.outputs[1].2.is_empty(),
            "script bytes still present"
        );
    }

    #[test]
    fn recovers_p2pkh_input_address_from_script_sig() {
        let (tx, addr) = p2pkh_spend_tx(Network::Testnet);
        let decoded = decode_ok(&tx, FFINetwork::Testnet);

        assert_eq!(decoded.inputs.len(), 1);
        assert_eq!(decoded.inputs[0].0, [0x11u8; 32]);
        assert_eq!(decoded.inputs[0].1, 3);
        assert_eq!(decoded.inputs[0].2, Some(addr.to_string()));
    }

    #[test]
    fn network_changes_rendered_addresses() {
        let addr = Address::dummy(Network::Testnet, 7);
        let tx = Transaction::dummy(&addr, 1..2, &[151_072]);
        let decoded = decode_ok(&tx, FFINetwork::Mainnet);
        let rendered = decoded.outputs[0].0.clone().unwrap();
        assert_ne!(rendered, addr.to_string());
        assert!(rendered.starts_with('X'), "mainnet P2PKH starts with 'X'");
    }

    #[test]
    fn coinbase_input_has_no_address() {
        let addr = Address::dummy(Network::Testnet, 3);
        let tx = Transaction::dummy_coinbase(&addr, 50_000);
        let decoded = decode_ok(&tx, FFINetwork::Testnet);
        assert!(decoded.inputs[0].2.is_none());
    }

    #[test]
    fn non_p2pkh_script_sig_yields_no_input_address() {
        // Transaction::dummy fills script_sig with a *lock* script
        // (OP_DUP OP_HASH160 <20 B> OP_EQUALVERIFY OP_CHECKSIG): opcodes
        // present, last push 20 bytes — must not produce an address.
        let addr = Address::dummy(Network::Testnet, 9);
        let tx = Transaction::dummy(&addr, 1..2, &[1_000]);
        let decoded = decode_ok(&tx, FFINetwork::Testnet);
        assert!(decoded.inputs[0].2.is_none());
    }

    #[test]
    fn redeem_script_collision_yields_no_input_address() {
        // Three pushes ending in a valid 33-byte pubkey — the P2SH
        // redeem-script shape the exactly-two-pushes rule exists to reject.
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0x30u8; 71])
            .push_slice([0x01u8; 20])
            .push_key(&test_pubkey())
            .into_script();
        let decoded = decode_ok(&tx_with_script_sig(script_sig), FFINetwork::Testnet);
        assert!(decoded.inputs[0].2.is_none());
    }

    #[test]
    fn non_signature_first_push_yields_no_input_address() {
        // Two pushes, but the first is not DER-shaped (no 0x30 tag).
        let script_sig = dashcore::blockdata::script::Builder::new()
            .push_slice([0xAAu8; 10])
            .push_key(&test_pubkey())
            .into_script();
        let decoded = decode_ok(&tx_with_script_sig(script_sig), FFINetwork::Testnet);
        assert!(decoded.inputs[0].2.is_none());
    }

    #[test]
    fn trailing_garbage_is_a_deserialization_error() {
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let mut bytes = serialize(&tx);
        bytes.extend_from_slice(&[0xDE, 0xAD]);
        unsafe {
            // Pre-poison the out param to pin that failures null it.
            let mut out: *mut DecodedTransactionFFI = std::ptr::NonNull::dangling().as_ptr();
            let res = platform_wallet_decode_transaction(
                bytes.as_ptr(),
                bytes.len(),
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorDeserialization);
            assert!(out.is_null(), "out param is nulled on failure");
        }
    }

    #[test]
    fn garbage_bytes_are_a_deserialization_error() {
        let bytes = [0xFFu8; 16];
        unsafe {
            let mut out: *mut DecodedTransactionFFI = std::ptr::NonNull::dangling().as_ptr();
            let res = platform_wallet_decode_transaction(
                bytes.as_ptr(),
                bytes.len(),
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorDeserialization);
            assert!(out.is_null(), "out param is nulled on failure");
        }
    }

    #[test]
    fn marshals_decoded_transaction_across_the_boundary() {
        // Raw pointer walk (no helper) — pins the C-side memory layout.
        let (tx, _) = p2pkh_spend_tx(Network::Testnet);
        let bytes = serialize(&tx);
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
            assert_eq!((*out).outputs_count, 2);
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
            // Pre-poison the out param to pin that failures null it.
            let mut out: *mut DecodedTransactionFFI = std::ptr::NonNull::dangling().as_ptr();
            let res = platform_wallet_decode_transaction(
                std::ptr::null(),
                4,
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
            assert!(out.is_null(), "out param is nulled on failure");

            let bytes = [0u8; 4];
            out = std::ptr::NonNull::dangling().as_ptr();
            let res = platform_wallet_decode_transaction(
                bytes.as_ptr(),
                0,
                FFINetwork::Testnet,
                &mut out,
            );
            assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
            assert!(out.is_null(), "out param is nulled on failure");
        }
    }

    #[test]
    fn free_null_is_safe() {
        unsafe { platform_wallet_decoded_transaction_free(std::ptr::null_mut()) }
    }
}
