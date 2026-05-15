//! Single-slot mnemonic-driven identity-authentication key derivation.

use std::ffi::{c_void, CString};
use std::os::raw::c_char;

use crate::types::{FFINetwork, Network};
use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
use zeroize::Zeroizing;

use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};
use crate::error::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use crate::{check_ptr, unwrap_result_or_return};

/// Derive a single ECDSA identity-authentication keypair at
/// `(identity_index, key_index)` from a BIP-39 mnemonic.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_key_at_slot(
    mnemonic_cstr: *const std::os::raw::c_char,
    passphrase_cstr: *const std::os::raw::c_char,
    network: FFINetwork,
    identity_index: u32,
    key_index: u32,
    out_row: *mut IdentityKeyPreviewFFI,
) -> PlatformWalletFFIResult {
    use std::ffi::CStr;

    check_ptr!(out_row);
    *out_row = IdentityKeyPreviewFFI::empty();

    check_ptr!(mnemonic_cstr);

    let mnemonic_str = unwrap_result_or_return!(CStr::from_ptr(mnemonic_cstr).to_str());
    let passphrase_str: &str = if passphrase_cstr.is_null() {
        ""
    } else {
        unwrap_result_or_return!(CStr::from_ptr(passphrase_cstr).to_str())
    };

    derive_at_slot_inner(
        mnemonic_str,
        passphrase_str,
        network,
        identity_index,
        key_index,
        out_row,
    )
}

unsafe fn derive_at_slot_inner(
    mnemonic_str: &str,
    passphrase_str: &str,
    network: FFINetwork,
    identity_index: u32,
    key_index: u32,
    out_row: *mut IdentityKeyPreviewFFI,
) -> PlatformWalletFFIResult {
    let mnemonic = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(passphrase_str));

    let kw_network: Network = network.into();
    let master = unwrap_result_or_return!(ExtendedPrivKey::new_master(kw_network, seed.as_ref()));

    let derived = unwrap_result_or_return!(derive_ecdsa_identity_auth_keypair_from_master(
        &master,
        kw_network,
        identity_index,
        key_index,
    ));

    let path_cstring = unwrap_result_or_return!(CString::new(derived.derivation_path.to_string()));

    let pub_bytes_vec = derived.public_key.to_vec();
    let mut pub_box: Box<[u8]> = pub_bytes_vec.into_boxed_slice();
    let pub_ptr = pub_box.as_mut_ptr();
    let pub_len = pub_box.len();
    std::mem::forget(pub_box);

    let secret_key = match dashcore::secp256k1::SecretKey::from_slice(derived.private_key.as_ref())
    {
        Ok(k) => k,
        Err(e) => {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                pub_ptr, pub_len,
            )));
            drop(path_cstring);
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("SecretKey::from_slice failed: {e}"),
            );
        }
    };
    let dash_private = DashPrivateKey {
        compressed: true,
        network: kw_network,
        inner: secret_key,
    };
    let wif_cstring = match CString::new(dash_private.to_wif()) {
        Ok(s) => s,
        Err(e) => {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                pub_ptr, pub_len,
            )));
            drop(path_cstring);
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                format!("WIF string contained NUL byte: {e}"),
            );
        }
    };

    let mut private_key_bytes_buf: Zeroizing<[u8; 32]> = Zeroizing::new([0u8; 32]);
    private_key_bytes_buf.copy_from_slice(derived.private_key.as_ref());

    *out_row = IdentityKeyPreviewFFI {
        identity_index,
        derivation_path: path_cstring.into_raw(),
        public_key: pub_ptr,
        public_key_len: pub_len,
        private_key_wif: wif_cstring.into_raw(),
        private_key_bytes: *private_key_bytes_buf,
    };

    PlatformWalletFFIResult::ok()
}

/// Resolver-based variant of [`dash_sdk_derive_identity_key_at_slot`].
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_key_at_slot_with_resolver(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_index: u32,
    key_index: u32,
    out_row: *mut IdentityKeyPreviewFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_row);
    *out_row = IdentityKeyPreviewFFI::empty();

    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);

    let mut mnemonic_buf: Zeroizing<[u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]> =
        Zeroizing::new([0u8; MNEMONIC_RESOLVER_BUFFER_CAPACITY]);
    let mut mnemonic_len: usize = 0;

    let resolver = &*mnemonic_resolver_handle;
    let resolver_vtable = &*resolver.vtable;
    let rc = (resolver_vtable.resolve)(
        resolver.ctx as *const c_void,
        wallet_id_bytes,
        mnemonic_buf.as_mut_ptr() as *mut c_char,
        MNEMONIC_RESOLVER_BUFFER_CAPACITY,
        &mut mnemonic_len,
    );
    match rc {
        x if x == mnemonic_resolver_result::SUCCESS => {}
        x if x == mnemonic_resolver_result::NOT_FOUND => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver: no mnemonic stored for the supplied wallet_id",
            );
        }
        x if x == mnemonic_resolver_result::BUFFER_TOO_SMALL => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver: mnemonic exceeded the FFI buffer capacity",
            );
        }
        _ => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                "mnemonic resolver: failed (other / Keychain access error)",
            );
        }
    }

    let mnemonic_str = unwrap_result_or_return!(std::str::from_utf8(&mnemonic_buf[..mnemonic_len]));

    derive_at_slot_inner(
        mnemonic_str,
        "",
        network,
        identity_index,
        key_index,
        out_row,
    )
}

/// Free a row populated by [`dash_sdk_derive_identity_key_at_slot`].
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_key_at_slot_free(
    out_row: *mut IdentityKeyPreviewFFI,
) {
    if out_row.is_null() {
        return;
    }
    let row = &mut *out_row;

    if !row.derivation_path.is_null() {
        let _ = CString::from_raw(row.derivation_path);
        row.derivation_path = std::ptr::null_mut();
    }
    if !row.public_key.is_null() && row.public_key_len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            row.public_key,
            row.public_key_len,
        ));
        row.public_key = std::ptr::null_mut();
        row.public_key_len = 0;
    }
    if !row.private_key_wif.is_null() {
        let cstring = CString::from_raw(row.private_key_wif);
        let bytes_len = cstring.as_bytes().len();
        std::ptr::write_bytes(cstring.as_ptr() as *mut u8, 0, bytes_len);
        drop(cstring);
        row.private_key_wif = std::ptr::null_mut();
    }
    for byte in row.private_key_bytes.iter_mut() {
        *byte = 0;
    }
    row.identity_index = 0;
}
