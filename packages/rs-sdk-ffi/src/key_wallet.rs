//! FFI bindings for key-wallet functionality
//!
//! This module exposes HD wallet functionality from rust-dashcore's key-wallet crate
//! through C-compatible FFI bindings for use in iOS/Swift applications.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::slice;
use std::str::FromStr;

use dashcore::{Address, Network, Script, Transaction, TxIn, TxOut};
use key_wallet::{
    DerivationPath, ExtendedPrivKey, ExtendedPubKey, Mnemonic as KeyWalletMnemonic,
    Network as KeyWalletNetwork,
};
use secp256k1::Secp256k1;
use secp256k1::SecretKey;

use crate::error::FFIError;
use dash_spv_ffi::set_last_error;

// MARK: - Network Type

/// FFI-compatible network enum for key wallet operations
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum FFIKeyNetwork {
    KeyMainnet = 0,
    KeyTestnet = 1,
    KeyRegtest = 2,
    KeyDevnet = 3,
}

impl From<FFIKeyNetwork> for KeyWalletNetwork {
    fn from(network: FFIKeyNetwork) -> Self {
        match network {
            FFIKeyNetwork::KeyMainnet => KeyWalletNetwork::Dash,
            FFIKeyNetwork::KeyTestnet => KeyWalletNetwork::Testnet,
            FFIKeyNetwork::KeyRegtest => KeyWalletNetwork::Regtest,
            FFIKeyNetwork::KeyDevnet => KeyWalletNetwork::Devnet,
        }
    }
}

// MARK: - Mnemonic

/// Opaque handle for a BIP39 mnemonic
pub struct FFIMnemonic {
    inner: KeyWalletMnemonic,
}

/// Generate a new BIP39 mnemonic
///
/// # Parameters
/// - `word_count`: Number of words (12, 15, 18, 21, or 24)
///
/// # Returns
/// - Pointer to FFIMnemonic on success
/// - NULL on error (check dash_get_last_error)
#[no_mangle]
pub extern "C" fn dash_key_mnemonic_generate(word_count: u8) -> *mut FFIMnemonic {
    match KeyWalletMnemonic::generate(word_count as usize, key_wallet::mnemonic::Language::English)
    {
        Ok(mnemonic) => Box::into_raw(Box::new(FFIMnemonic { inner: mnemonic })),
        Err(e) => {
            set_last_error(&format!("Failed to generate mnemonic: {}", e));
            ptr::null_mut()
        }
    }
}

/// Create a mnemonic from a phrase
///
/// # Parameters
/// - `phrase`: The mnemonic phrase as a C string
///
/// # Returns
/// - Pointer to FFIMnemonic on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_mnemonic_from_phrase(phrase: *const c_char) -> *mut FFIMnemonic {
    let phrase_str = match unsafe { CStr::from_ptr(phrase).to_str() } {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid UTF-8 in phrase: {}", e));
            return ptr::null_mut();
        }
    };

    match KeyWalletMnemonic::from_phrase(phrase_str, key_wallet::mnemonic::Language::English) {
        Ok(mnemonic) => Box::into_raw(Box::new(FFIMnemonic { inner: mnemonic })),
        Err(e) => {
            set_last_error(&format!("Invalid mnemonic: {}", e));
            ptr::null_mut()
        }
    }
}

/// Get the phrase from a mnemonic
///
/// # Parameters
/// - `mnemonic`: The mnemonic handle
///
/// # Returns
/// - C string containing the phrase (caller must free with dash_string_free)
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_mnemonic_phrase(mnemonic: *const FFIMnemonic) -> *mut c_char {
    if mnemonic.is_null() {
        set_last_error("mnemonic");
        return ptr::null_mut();
    }

    let mnemonic = unsafe { &*mnemonic };
    match CString::new(mnemonic.inner.phrase()) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            set_last_error(&format!("Failed to convert phrase: {}", e));
            ptr::null_mut()
        }
    }
}

/// Convert mnemonic to seed
///
/// # Parameters
/// - `mnemonic`: The mnemonic handle
/// - `passphrase`: Optional passphrase (can be NULL)
/// - `seed_out`: Buffer to write seed (must be 64 bytes)
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_key_mnemonic_to_seed(
    mnemonic: *const FFIMnemonic,
    passphrase: *const c_char,
    seed_out: *mut u8,
) -> i32 {
    if mnemonic.is_null() || seed_out.is_null() {
        set_last_error("mnemonic or seed_out");
        return -1;
    }

    let mnemonic = unsafe { &*mnemonic };
    let passphrase_str = if passphrase.is_null() {
        ""
    } else {
        match unsafe { CStr::from_ptr(passphrase).to_str() } {
            Ok(s) => s,
            Err(e) => {
                set_last_error(&format!("Invalid passphrase: {}", e));
                return -1;
            }
        }
    };

    let seed = mnemonic.inner.to_seed(passphrase_str);
    unsafe {
        ptr::copy_nonoverlapping(seed.as_ptr(), seed_out, 64);
    }
    0
}

/// Destroy a mnemonic
#[no_mangle]
pub extern "C" fn dash_key_mnemonic_destroy(mnemonic: *mut FFIMnemonic) {
    if !mnemonic.is_null() {
        unsafe {
            let _ = Box::from_raw(mnemonic);
        }
    }
}

// MARK: - Extended Keys

/// Opaque handle for an extended private key
pub struct FFIExtendedPrivKey {
    inner: ExtendedPrivKey,
    network: KeyWalletNetwork,
}

/// Opaque handle for an extended public key
pub struct FFIExtendedPubKey {
    inner: ExtendedPubKey,
    network: KeyWalletNetwork,
}

/// Create an extended private key from seed
///
/// # Parameters
/// - `seed`: The seed bytes (must be 64 bytes)
/// - `network`: The network type
///
/// # Returns
/// - Pointer to FFIExtendedPrivKey on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_xprv_from_seed(
    seed: *const u8,
    network: FFIKeyNetwork,
) -> *mut FFIExtendedPrivKey {
    if seed.is_null() {
        set_last_error("seed");
        return ptr::null_mut();
    }

    let seed_slice = unsafe { slice::from_raw_parts(seed, 64) };
    let network = network.into();

    match ExtendedPrivKey::new_master(network, seed_slice) {
        Ok(xprv) => Box::into_raw(Box::new(FFIExtendedPrivKey {
            inner: xprv,
            network,
        })),
        Err(e) => {
            set_last_error(&format!("Failed to create master key: {}", e));
            ptr::null_mut()
        }
    }
}

/// Derive a child key from extended private key
///
/// # Parameters
/// - `xprv`: The parent extended private key
/// - `index`: The child index
/// - `hardened`: Whether to use hardened derivation
///
/// # Returns
/// - Pointer to derived FFIExtendedPrivKey on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_xprv_derive_child(
    xprv: *const FFIExtendedPrivKey,
    index: u32,
    hardened: bool,
) -> *mut FFIExtendedPrivKey {
    if xprv.is_null() {
        set_last_error("xprv");
        return ptr::null_mut();
    }

    let xprv = unsafe { &*xprv };
    let child_number = if hardened {
        key_wallet::bip32::ChildNumber::from_hardened_idx(index)
    } else {
        key_wallet::bip32::ChildNumber::from_normal_idx(index)
    };

    match child_number.and_then(|cn| xprv.inner.ckd_priv(&Secp256k1::new(), cn)) {
        Ok(child) => Box::into_raw(Box::new(FFIExtendedPrivKey {
            inner: child,
            network: xprv.network,
        })),
        Err(e) => {
            set_last_error(&format!("Failed to derive child: {}", e));
            ptr::null_mut()
        }
    }
}

/// Derive key at BIP32 path
///
/// # Parameters
/// - `xprv`: The root extended private key
/// - `path`: The derivation path (e.g., "m/44'/5'/0'/0/0")
///
/// # Returns
/// - Pointer to derived FFIExtendedPrivKey on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_xprv_derive_path(
    xprv: *const FFIExtendedPrivKey,
    path: *const c_char,
) -> *mut FFIExtendedPrivKey {
    if xprv.is_null() || path.is_null() {
        set_last_error("xprv or path");
        return ptr::null_mut();
    }

    let xprv = unsafe { &*xprv };
    let path_str = match unsafe { CStr::from_ptr(path).to_str() } {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("Invalid path: {}", e));
            return ptr::null_mut();
        }
    };

    match DerivationPath::from_str(path_str) {
        Ok(derivation_path) => match xprv.inner.derive_priv(&Secp256k1::new(), &derivation_path) {
            Ok(derived) => Box::into_raw(Box::new(FFIExtendedPrivKey {
                inner: derived,
                network: xprv.network,
            })),
            Err(e) => {
                set_last_error(&format!("Failed to derive: {}", e));
                ptr::null_mut()
            }
        },
        Err(e) => {
            set_last_error(&format!("Invalid derivation path: {}", e));
            ptr::null_mut()
        }
    }
}

/// Get extended public key from extended private key
///
/// # Parameters
/// - `xprv`: The extended private key
///
/// # Returns
/// - Pointer to FFIExtendedPubKey on success
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_xprv_to_xpub(xprv: *const FFIExtendedPrivKey) -> *mut FFIExtendedPubKey {
    if xprv.is_null() {
        set_last_error("xprv");
        return ptr::null_mut();
    }

    let xprv = unsafe { &*xprv };
    let xpub = ExtendedPubKey::from_priv(&Secp256k1::new(), &xprv.inner);

    Box::into_raw(Box::new(FFIExtendedPubKey {
        inner: xpub,
        network: xprv.network,
    }))
}

/// Get private key bytes
///
/// # Parameters
/// - `xprv`: The extended private key
/// - `key_out`: Buffer to write key (must be 32 bytes)
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_key_xprv_private_key(
    xprv: *const FFIExtendedPrivKey,
    key_out: *mut u8,
) -> i32 {
    if xprv.is_null() || key_out.is_null() {
        set_last_error("xprv or key_out");
        return -1;
    }

    let xprv = unsafe { &*xprv };
    let key_bytes = xprv.inner.private_key.secret_bytes();

    unsafe {
        ptr::copy_nonoverlapping(key_bytes.as_ptr(), key_out, 32);
    }
    0
}

/// Destroy an extended private key
#[no_mangle]
pub extern "C" fn dash_key_xprv_destroy(xprv: *mut FFIExtendedPrivKey) {
    if !xprv.is_null() {
        unsafe {
            let _ = Box::from_raw(xprv);
        }
    }
}

/// Get public key bytes from extended public key
///
/// # Parameters
/// - `xpub`: The extended public key
/// - `key_out`: Buffer to write key (must be 33 bytes for compressed)
///
/// # Returns
/// - 0 on success
/// - -1 on error
#[no_mangle]
pub extern "C" fn dash_key_xpub_public_key(
    xpub: *const FFIExtendedPubKey,
    key_out: *mut u8,
) -> i32 {
    if xpub.is_null() || key_out.is_null() {
        set_last_error("xpub or key_out");
        return -1;
    }

    let xpub = unsafe { &*xpub };
    let key_bytes = xpub.inner.public_key.serialize();

    unsafe {
        ptr::copy_nonoverlapping(key_bytes.as_ptr(), key_out, 33);
    }
    0
}

/// Destroy an extended public key
#[no_mangle]
pub extern "C" fn dash_key_xpub_destroy(xpub: *mut FFIExtendedPubKey) {
    if !xpub.is_null() {
        unsafe {
            let _ = Box::from_raw(xpub);
        }
    }
}

// MARK: - Address Generation

/// Generate a P2PKH address from public key
///
/// # Parameters
/// - `pubkey`: The public key bytes (33 bytes compressed)
/// - `network`: The network type
///
/// # Returns
/// - C string containing the address (caller must free)
/// - NULL on error
#[no_mangle]
pub extern "C" fn dash_key_address_from_pubkey(
    pubkey: *const u8,
    network: FFIKeyNetwork,
) -> *mut c_char {
    if pubkey.is_null() {
        set_last_error("pubkey");
        return ptr::null_mut();
    }

    let pubkey_slice = unsafe { slice::from_raw_parts(pubkey, 33) };
    let network: Network = network.into();

    match secp256k1::PublicKey::from_slice(pubkey_slice) {
        Ok(secp_pk) => {
            let pk = dashcore::PublicKey::new(secp_pk);
            let address = Address::p2pkh(&pk, network);
            match CString::new(address.to_string()) {
                Ok(s) => s.into_raw(),
                Err(e) => {
                    set_last_error(&format!("Failed to convert address: {}", e));
                    ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_last_error(&format!("Invalid public key: {}", e));
            ptr::null_mut()
        }
    }
}

/// Validate an address string
///
/// # Parameters
/// - `address`: The address string
/// - `network`: The expected network
///
/// # Returns
/// - 1 if valid
/// - 0 if invalid
#[no_mangle]
pub extern "C" fn dash_key_address_validate(address: *const c_char, network: FFIKeyNetwork) -> i32 {
    if address.is_null() {
        return 0;
    }

    let address_str = match unsafe { CStr::from_ptr(address).to_str() } {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let expected_network: Network = network.into();

    match address_str.parse::<Address<_>>() {
        Ok(addr) => {
            if *addr.network() == expected_network {
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}
