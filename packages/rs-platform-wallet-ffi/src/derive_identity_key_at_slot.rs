//! Single-slot mnemonic-driven identity-authentication key derivation.

use std::ffi::{c_void, CString};
use std::os::raw::c_char;

use crate::types::{FFINetwork, Network};
use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;
use zeroize::Zeroizing;

use crate::error::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use crate::sign_gate::verify_seed_matches_wallet_id;
use crate::{check_ptr, unwrap_result_or_return};
use dashcore::secp256k1::Secp256k1;
use key_wallet::bip32::ExtendedPubKey;
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

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

    // Fail-closed wrong-seed gate before any signing-relevant material
    // is materialized for the caller. Derive the root xpub from the
    // resolver-supplied seed in its own scope so the master xpriv +
    // its SecretKey are non-secure-erased before the inner derivation
    // path runs (which re-derives from the seed via the shared helper).
    {
        let mnemonic = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));
        let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
        drop(mnemonic);
        let kw_network: Network = network.into();
        let mut master =
            unwrap_result_or_return!(ExtendedPrivKey::new_master(kw_network, seed.as_ref()));
        let secp = Secp256k1::new();
        let root_xpub = ExtendedPubKey::from_priv(&secp, &master);
        let mut wallet_id_expected = [0u8; 32];
        std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id_expected.as_mut_ptr(), 32);
        let gate_ok = verify_seed_matches_wallet_id(&root_xpub, &wallet_id_expected);
        master.private_key.non_secure_erase();
        if !gate_ok {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWrongSeedForWallet,
                "wrong seed for wallet (derive_identity_key_at_slot_with_resolver gate)",
            );
        }
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use key_wallet::wallet::root_extended_keys::RootExtendedPubKey;
    use key_wallet::wallet::Wallet;
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};

    /// English BIP-39 test vector (all-zero entropy). Matches the
    /// fixture in the sibling resolver-fed entrypoints.
    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    /// Compute the `wallet_id` the gate will recompute from the
    /// resolver-supplied seed. Tests must pass this id for happy-path
    /// or short-circuit with `ErrorWrongSeedForWallet` on mismatch.
    fn wallet_id_for_english_phrase() -> [u8; 32] {
        let mnemonic = parse_mnemonic_any_language(ENGLISH_PHRASE).unwrap();
        let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));
        let mut master =
            ExtendedPrivKey::new_master(key_wallet::Network::Testnet, seed.as_ref()).unwrap();
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &master);
        let root = RootExtendedPubKey::from_extended_pub_key(&xpub);
        let id = Wallet::compute_wallet_id_from_root_extended_pub_key(&root);
        master.private_key.non_secure_erase();
        id
    }

    unsafe extern "C" fn english_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = ENGLISH_PHRASE.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    fn make_resolver() -> *mut MnemonicResolverHandle {
        unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), english_resolve, noop_destroy)
        }
    }

    /// Happy path: resolver yields a mnemonic whose derived wallet_id
    /// matches the caller-supplied one; the gate passes and a non-empty
    /// derived keypair is returned in `out_row`.
    #[test]
    fn matching_seed_returns_derived_key() {
        let resolver = make_resolver();
        let wallet_id = wallet_id_for_english_phrase();
        let mut out_row = IdentityKeyPreviewFFI::empty();
        let rc = unsafe {
            dash_sdk_derive_identity_key_at_slot_with_resolver(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                resolver,
                5, // identity_index
                0, // key_index
                &mut out_row,
            )
        };
        assert_eq!(rc.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out_row.identity_index, 5);
        assert!(!out_row.derivation_path.is_null());
        assert!(!out_row.public_key.is_null());
        assert_eq!(out_row.public_key_len, 33, "compressed secp256k1 pubkey");
        assert!(!out_row.private_key_wif.is_null());
        // private_key_bytes must be populated (non-all-zero with
        // overwhelming probability for a real derivation).
        assert!(
            out_row.private_key_bytes.iter().any(|b| *b != 0),
            "derived private scalar should not be all zeros"
        );

        // Path shape: testnet coin = 1, identity_index = 5, key_index = 0.
        let path = unsafe { std::ffi::CStr::from_ptr(out_row.derivation_path) }.to_string_lossy();
        assert_eq!(path, "m/9'/1'/5'/0'/0'/5'/0'");

        unsafe {
            dash_sdk_derive_identity_key_at_slot_free(&mut out_row);
            dash_sdk_mnemonic_resolver_destroy(resolver);
        }
    }

    /// Wrong-seed gate fires: resolver yields a valid mnemonic but the
    /// caller-supplied `wallet_id` doesn't match its derived id.
    /// `out_row` must be left at its zeroed-empty state and no derived
    /// key material may leak through it.
    #[test]
    fn wrong_wallet_id_fails_closed_with_wrong_seed_tag() {
        let resolver = make_resolver();
        // A wallet_id that cannot match the abandon-x12 derived id.
        let wrong_wallet_id = [0xAAu8; 32];
        let mut out_row = IdentityKeyPreviewFFI::empty();
        let rc = unsafe {
            dash_sdk_derive_identity_key_at_slot_with_resolver(
                FFINetwork::Testnet,
                wrong_wallet_id.as_ptr(),
                resolver,
                0,
                0,
                &mut out_row,
            )
        };
        assert_eq!(
            rc.code,
            PlatformWalletFFIResultCode::ErrorWrongSeedForWallet,
            "wrong-seed gate must fire with the dedicated structural tag"
        );
        // Caller-owned output struct must be the zero/empty state — no
        // derivation_path, no pubkey, no WIF, no private scalar bytes.
        assert!(out_row.derivation_path.is_null());
        assert!(out_row.public_key.is_null());
        assert_eq!(out_row.public_key_len, 0);
        assert!(out_row.private_key_wif.is_null());
        for b in out_row.private_key_bytes {
            assert_eq!(b, 0, "private_key_bytes must be fully zeroed");
        }
        assert_eq!(out_row.identity_index, 0);

        unsafe {
            // Defensive: free is null-tolerant on the empty row.
            dash_sdk_derive_identity_key_at_slot_free(&mut out_row);
            dash_sdk_mnemonic_resolver_destroy(resolver);
        }
    }
}
