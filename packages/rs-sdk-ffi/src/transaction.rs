//! FFI bindings for transaction functionality
//!
//! This module exposes transaction creation and manipulation functionality
//! from rust-dashcore through C-compatible FFI bindings.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;

use dashcore::{
    Transaction, TxIn, TxOut, OutPoint, Script, ScriptBuf, 
    Txid, consensus, Network, Amount, EcdsaSighashType,
    sighash::SighashCache, hashes::{Hash, sha256d},
    Address, PrivateKey, PublicKey,
};
use secp256k1::{Secp256k1, SecretKey, Message};

use dash_spv_ffi::set_last_error;
use crate::error::FFIError;
use crate::key_wallet::FFIKeyNetwork;

// MARK: - Transaction Types

/// Opaque handle for a transaction
pub struct FFITransaction {
    inner: Transaction,
}

/// FFI-compatible transaction input
#[repr(C)]
pub struct FFITxIn {
    /// Transaction ID (32 bytes)
    pub txid: [u8; 32],
    /// Output index
    pub vout: u32,
    /// Script signature length
    pub script_sig_len: u32,
    /// Script signature data pointer
    pub script_sig: *const u8,
    /// Sequence number
    pub sequence: u32,
}

/// FFI-compatible transaction output
#[repr(C)]
pub struct FFITxOut {
    /// Amount in satoshis
    pub amount: u64,
    /// Script pubkey length
    pub script_pubkey_len: u32,
    /// Script pubkey data pointer
    pub script_pubkey: *const u8,
}

// MARK: - Transaction Creation

/// Create a new empty transaction
///
/// # Returns
/// - Pointer to FFITransaction on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_tx_create() -> *mut FFITransaction {
    let tx = Transaction {
        version: 2,
        lock_time: 0,
        input: vec![],
        output: vec![],
        special_transaction_payload: None,
    };
    
    Box::into_raw(Box::new(FFITransaction { inner: tx }))
}

/// Add an input to a transaction
///
/// # Parameters
/// - `tx`: The transaction
/// - `input`: The input to add
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_tx_add_input(
    tx: *mut FFITransaction,
    input: *const FFITxIn,
) -> i32 {
    if tx.is_null() || input.is_null() {
        set_last_error("tx or input");
        return -1;
    }

    let tx = unsafe { &mut *tx };
    let input = unsafe { &*input };
    
    // Convert txid
    // Convert 32-byte array to Txid
    let txid = match Txid::from_slice(&input.txid) {
        Ok(txid) => txid,
        Err(e) => {
            set_last_error(&format!("Invalid txid: {}", e));
            return -1;
        }
    };
    
    // Convert script
    let script_sig = if input.script_sig.is_null() || input.script_sig_len == 0 {
        ScriptBuf::new()
    } else {
        let script_slice = unsafe { 
            slice::from_raw_parts(input.script_sig, input.script_sig_len as usize) 
        };
        ScriptBuf::from(script_slice.to_vec())
    };
    
    let tx_in = TxIn {
        previous_output: OutPoint {
            txid,
            vout: input.vout,
        },
        script_sig,
        sequence: input.sequence,
        witness: Default::default(),
    };
    
    tx.inner.input.push(tx_in);
    0
}

/// Add an output to a transaction
///
/// # Parameters
/// - `tx`: The transaction
/// - `output`: The output to add
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_tx_add_output(
    tx: *mut FFITransaction,
    output: *const FFITxOut,
) -> i32 {
    if tx.is_null() || output.is_null() {
        set_last_error("tx or output");
        return -1;
    }

    let tx = unsafe { &mut *tx };
    let output = unsafe { &*output };
    
    // Convert script
    let script_pubkey = if output.script_pubkey.is_null() || output.script_pubkey_len == 0 {
        set_last_error("Output script cannot be empty");
        return -1;
    } else {
        let script_slice = unsafe { 
            slice::from_raw_parts(output.script_pubkey, output.script_pubkey_len as usize) 
        };
        ScriptBuf::from(script_slice.to_vec())
    };
    
    let tx_out = TxOut {
        value: output.amount,
        script_pubkey,
    };
    
    tx.inner.output.push(tx_out);
    0
}

/// Get the transaction ID
///
/// # Parameters
/// - `tx`: The transaction
/// - `txid_out`: Buffer to write txid (must be 32 bytes)
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_tx_get_txid(
    tx: *const FFITransaction,
    txid_out: *mut u8,
) -> i32 {
    if tx.is_null() || txid_out.is_null() {
        set_last_error("tx or txid_out");
        return -1;
    }

    let tx = unsafe { &*tx };
    let txid = tx.inner.txid();
    
    unsafe {
        let txid_bytes = txid.as_byte_array();
        ptr::copy_nonoverlapping(txid_bytes.as_ptr(), txid_out, 32);
    }
    0
}

/// Serialize a transaction
///
/// # Parameters
/// - `tx`: The transaction
/// - `out_buf`: Buffer to write serialized data (can be NULL to get size)
/// - `out_len`: In/out parameter for buffer size
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_tx_serialize(
    tx: *const FFITransaction,
    out_buf: *mut u8,
    out_len: *mut u32,
) -> i32 {
    if tx.is_null() || out_len.is_null() {
        set_last_error("tx or out_len");
        return -1;
    }

    let tx = unsafe { &*tx };
    let serialized = consensus::serialize(&tx.inner);
    let size = serialized.len() as u32;
    
    unsafe {
        if out_buf.is_null() {
            // Just return size
            *out_len = size;
            return 0;
        }
        
        let provided_size = *out_len;
        if provided_size < size {
            set_last_error(&
                format!("Buffer too small: {} < {}", provided_size, size)
            );
            *out_len = size;
            return -1;
        }
        
        ptr::copy_nonoverlapping(serialized.as_ptr(), out_buf, serialized.len());
        *out_len = size;
    }
    
    0
}

/// Deserialize a transaction
///
/// # Parameters
/// - `data`: The serialized transaction data
/// - `len`: Length of the data
///
/// # Returns
/// - Pointer to FFITransaction on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_tx_deserialize(
    data: *const u8,
    len: u32,
) -> *mut FFITransaction {
    if data.is_null() {
        set_last_error("data");
        return ptr::null_mut();
    }

    let slice = unsafe { slice::from_raw_parts(data, len as usize) };
    
    match consensus::deserialize::<Transaction>(slice) {
        Ok(tx) => Box::into_raw(Box::new(FFITransaction { inner: tx })),
        Err(e) => {
            set_last_error(&format!("Failed to deserialize: {}", e));
            ptr::null_mut()
        }
    }
}

/// Destroy a transaction
#[no_mangle]
pub extern "C" fn dash_tx_destroy(tx: *mut FFITransaction) {
    if !tx.is_null() {
        unsafe {
            let _ = Box::from_raw(tx);
        }
    }
}

// MARK: - Transaction Signing

/// Calculate signature hash for an input
///
/// # Parameters
/// - `tx`: The transaction
/// - `input_index`: Which input to sign
/// - `script_pubkey`: The script pubkey of the output being spent
/// - `script_pubkey_len`: Length of script pubkey
/// - `sighash_type`: Signature hash type (usually 0x01 for SIGHASH_ALL)
/// - `hash_out`: Buffer to write hash (must be 32 bytes)
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_tx_sighash(
    tx: *const FFITransaction,
    input_index: u32,
    script_pubkey: *const u8,
    script_pubkey_len: u32,
    sighash_type: u32,
    hash_out: *mut u8,
) -> i32 {
    if tx.is_null() || script_pubkey.is_null() || hash_out.is_null() {
        set_last_error("tx, script_pubkey, or hash_out");
        return -1;
    }

    let tx = unsafe { &*tx };
    let script_slice = unsafe { 
        slice::from_raw_parts(script_pubkey, script_pubkey_len as usize) 
    };
    let script = Script::from_bytes(script_slice);
    
    let sighash_type = EcdsaSighashType::from_consensus(sighash_type);
    let cache = SighashCache::new(&tx.inner);
    
    match cache.legacy_signature_hash(input_index as usize, script, sighash_type.to_u32()) {
        Ok(hash) => {
            unsafe {
                let hash_bytes: &[u8] = hash.as_ref();
                ptr::copy_nonoverlapping(hash_bytes.as_ptr(), hash_out, 32);
            }
            0
        }
        Err(e) => {
            set_last_error(&format!("Failed to calculate sighash: {}", e));
            -1
        }
    }
}

/// Sign a transaction input
///
/// # Parameters
/// - `tx`: The transaction
/// - `input_index`: Which input to sign
/// - `private_key`: The private key (32 bytes)
/// - `script_pubkey`: The script pubkey of the output being spent
/// - `script_pubkey_len`: Length of script pubkey
/// - `sighash_type`: Signature hash type
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_tx_sign_input(
    tx: *mut FFITransaction,
    input_index: u32,
    private_key: *const u8,
    script_pubkey: *const u8,
    script_pubkey_len: u32,
    sighash_type: u32,
) -> i32 {
    if tx.is_null() || private_key.is_null() || script_pubkey.is_null() {
        set_last_error("tx, private_key, or script_pubkey");
        return -1;
    }

    let tx = unsafe { &mut *tx };
    let input_index = input_index as usize;
    
    if input_index >= tx.inner.input.len() {
        set_last_error("Input index out of range");
        return -1;
    }

    // Calculate sighash
    let mut sighash = [0u8; 32];
    if dash_tx_sighash(
        tx as *const FFITransaction,
        input_index as u32,
        script_pubkey,
        script_pubkey_len,
        sighash_type,
        sighash.as_mut_ptr(),
    ) != 0 {
        return -1;
    }

    // Parse private key
    let privkey_slice = unsafe { slice::from_raw_parts(private_key, 32) };
    let privkey = match SecretKey::from_slice(privkey_slice) {
        Ok(k) => k,
        Err(e) => {
            set_last_error(&format!("Invalid private key: {}", e));
            return -1;
        }
    };

    // Sign
    let secp = Secp256k1::new();
    let message = Message::from_digest(sighash);
    let sig = secp.sign_ecdsa(&message, &privkey);
    
    // Build signature script (simplified P2PKH)
    let mut sig_bytes = sig.serialize_der().to_vec();
    sig_bytes.push(sighash_type as u8);
    
    let pubkey = secp256k1::PublicKey::from_secret_key(&secp, &privkey);
    let pubkey_bytes = pubkey.serialize();
    
    let mut script_sig = vec![];
    script_sig.push(sig_bytes.len() as u8);
    script_sig.extend_from_slice(&sig_bytes);
    script_sig.push(pubkey_bytes.len() as u8);
    script_sig.extend_from_slice(&pubkey_bytes);
    
    tx.inner.input[input_index].script_sig = ScriptBuf::from(script_sig);
    0
}

// MARK: - Script Utilities

/// Create a P2PKH script pubkey
///
/// # Parameters
/// - `pubkey_hash`: The public key hash (20 bytes)
/// - `out_buf`: Buffer to write script (can be NULL to get size)
/// - `out_len`: In/out parameter for buffer size
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_script_p2pkh(
    pubkey_hash: *const u8,
    out_buf: *mut u8,
    out_len: *mut u32,
) -> i32 {
    if pubkey_hash.is_null() || out_len.is_null() {
        set_last_error("pubkey_hash or out_len");
        return -1;
    }

    let hash_slice = unsafe { slice::from_raw_parts(pubkey_hash, 20) };
    
    // Build P2PKH script: OP_DUP OP_HASH160 <hash> OP_EQUALVERIFY OP_CHECKSIG
    let mut script = vec![0x76, 0xa9, 0x14]; // OP_DUP OP_HASH160 PUSH(20)
    script.extend_from_slice(hash_slice);
    script.extend_from_slice(&[0x88, 0xac]); // OP_EQUALVERIFY OP_CHECKSIG
    
    let size = script.len() as u32;
    
    unsafe {
        if out_buf.is_null() {
            *out_len = size;
            return 0;
        }
        
        let provided_size = *out_len;
        if provided_size < size {
            set_last_error(&
                format!("Buffer too small: {} < {}", provided_size, size)
            );
            *out_len = size;
            return -1;
        }
        
        ptr::copy_nonoverlapping(script.as_ptr(), out_buf, script.len());
        *out_len = size;
    }
    
    0
}

/// Extract public key hash from P2PKH address
///
/// # Parameters
/// - `address`: The address string
/// - `network`: The expected network
/// - `hash_out`: Buffer to write hash (must be 20 bytes)
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_address_to_pubkey_hash(
    address: *const c_char,
    network: FFIKeyNetwork,
    hash_out: *mut u8,
) -> i32 {
    if address.is_null() || hash_out.is_null() {
        set_last_error("address or hash_out");
        return -1;
    }

    let address_str = match unsafe { CStr::from_ptr(address).to_str() } {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8: {}", e));
            return -1;
        }
    };

    let expected_network: Network = network.into();
    
    match address_str.parse::<dashcore::Address<_>>() {
        Ok(addr) => {
            if *addr.network() != expected_network {
                set_last_error("Address network mismatch");
                return -1;
            }
            
            match addr.payload() {
                dashcore::address::Payload::PubkeyHash(hash) => {
                    unsafe {
                        let hash_bytes = hash.as_byte_array();
                        ptr::copy_nonoverlapping(hash_bytes.as_ptr(), hash_out, 20);
                    }
                    0
                }
                _ => {
                    set_last_error("Not a P2PKH address");
                    -1
                }
            }
        }
        Err(e) => {
            set_last_error(&format!("Invalid address: {}", e));
            -1
        }
    }
}