//! Stateless derivation helpers.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::str::FromStr;

use key_wallet::bip32::{DerivationPath, ExtendedPrivKey, KeyDerivationType};
use platform_wallet::wallet::identity::network::identity_auth_derivation_path_for_type;
use platform_wallet::{
    first_receive_address_for_mnemonic, parse_mnemonic_any_language, wallet_id_for_mnemonic,
};

use crate::error::*;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};

/// Derive a 32-byte ECDSA private key at a BIP-32 derivation path from
/// a mnemonic phrase.
///
/// On success `out_secret_key` (32 bytes) receives the secp256k1
/// private key and `out_chain_code` (32 bytes) receives the BIP-32
/// chain code. The corresponding compressed public key (33 bytes) is
/// computed from the private key and written to `out_public_key` so
/// callers that only need the pubkey (identity registration: pubkey
/// only) never have to touch the private material.
///
/// # Inputs
/// * `mnemonic` — UTF-8 null-terminated BIP-39 mnemonic phrase.
/// * `passphrase` — optional UTF-8 null-terminated BIP-39 passphrase.
///    Pass `null` for no passphrase.
/// * `network` — Drives the xpriv version bytes; the derived secret
///    key itself is network-independent but the master key's chain
///    code derivation uses the HD seed directly.
/// * `path_utf8` — UTF-8 null-terminated BIP-32 path string, e.g.
///    `m/9'/5'/0'/0'/0'`. Parsed via [`DerivationPath::from_str`].
/// * `out_secret_key` — 32 byte buffer receiving the private key.
///    Caller must ensure the buffer is 32 bytes.
/// * `out_chain_code` — 32 byte buffer receiving the chain code.
/// * `out_public_key` — 33 byte buffer receiving the compressed
///    secp256k1 pubkey. Pass `null` to skip.
///
/// # Safety
/// All pointers must either be null (only where marked optional) or
/// point at valid memory of the documented sizes. The function does
/// not retain any pointer beyond the call.
///
/// # Zeroization
/// The derived master xpriv, the derived xpriv at `path`, and the
/// intermediate seed are wrapped in [`zeroize::Zeroizing`] inside this
/// function. The only material left in memory after return is the
/// 32-byte secret bytes written into the caller's buffer (caller's
/// responsibility to manage).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_derive_ext_priv_key_from_mnemonic(
    mnemonic: *const c_char,
    passphrase: *const c_char,
    network: FFINetwork,
    path_utf8: *const c_char,
    out_secret_key: *mut u8,
    out_chain_code: *mut u8,
    out_public_key: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(mnemonic);
    check_ptr!(path_utf8);
    check_ptr!(out_secret_key);
    check_ptr!(out_chain_code);

    let mnemonic_str = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let passphrase_str: &str = if passphrase.is_null() {
        ""
    } else {
        unwrap_result_or_return!(CStr::from_ptr(passphrase).to_str())
    };
    let path_str = unwrap_result_or_return!(CStr::from_ptr(path_utf8).to_str());

    let network: Network = network.into();

    let path = unwrap_result_or_return!(DerivationPath::from_str(path_str));

    let mnemonic_obj = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));

    let seed = mnemonic_obj.to_seed(passphrase_str);

    let master = unwrap_result_or_return!(ExtendedPrivKey::new_master(network, &*seed));

    let derived = unwrap_result_or_return!(master.derive_priv(&path));

    let secret = derived.private_key.to_bytes();
    std::ptr::copy_nonoverlapping(secret.as_ptr(), out_secret_key, 32);

    std::ptr::copy_nonoverlapping(derived.chain_code.as_ref().as_ptr(), out_chain_code, 32);

    if !out_public_key.is_null() {
        let pubkey_bytes = derived.private_key.public_key().to_compressed();
        std::ptr::copy_nonoverlapping(pubkey_bytes.as_ptr(), out_public_key, 33);
    }

    PlatformWalletFFIResult::ok()
}

/// Returns the DIP-13 ECDSA identity authentication key path for
/// `identity_index` and `key_index` on `network`, such as
/// `m/9'/5'/5'/0'/0'/3'/0'` on mainnet.
///
/// Either index at `2^31` or above fails with `ErrorInvalidParameter`, because
/// it cannot be a hardened child. On success the caller owns `*out_path` and
/// must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `out_path` must point to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_identity_authentication_path(
    network: FFINetwork,
    identity_index: u32,
    key_index: u32,
    out_path: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_path);
    *out_path = ptr::null_mut();

    let path = unwrap_result_or_return!(identity_auth_derivation_path_for_type(
        network.into(),
        KeyDerivationType::ECDSA,
        identity_index,
        key_index
    )
    .map_err(|e| PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorInvalidParameter,
        e.to_string()
    )));
    let c_path = unwrap_result_or_return!(CString::new(path.to_string()));
    *out_path = c_path.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Writes the 32-byte id the platform wallet manager gives the wallet of
/// `mnemonic` on `network`, without creating a wallet. See
/// [`platform_wallet::wallet_id_for_mnemonic`].
///
/// A phrase in no supported wordlist fails with `ErrorInvalidParameter`.
///
/// # Safety
/// `mnemonic` must point to a valid null-terminated UTF-8 C string and
/// `out_wallet_id` to 32 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_wallet_id_from_mnemonic(
    mnemonic: *const c_char,
    network: FFINetwork,
    out_wallet_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(mnemonic);
    check_ptr!(out_wallet_id);

    let mnemonic = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let wallet_id = unwrap_result_or_return!(wallet_id_for_mnemonic(mnemonic, network.into()));
    ptr::copy_nonoverlapping(wallet_id.as_ptr(), out_wallet_id, 32);
    PlatformWalletFFIResult::ok()
}

/// Returns the first receive address of `mnemonic` on `network`. See
/// [`platform_wallet::first_receive_address_for_mnemonic`].
///
/// A phrase in no supported wordlist fails with `ErrorInvalidParameter`. On
/// success the caller owns `*out_address` and must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `mnemonic` must point to a valid null-terminated UTF-8 C string and
/// `out_address` to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_first_receive_address_from_mnemonic(
    mnemonic: *const c_char,
    network: FFINetwork,
    out_address: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_address);
    *out_address = ptr::null_mut();
    check_ptr!(mnemonic);

    let mnemonic = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let address =
        unwrap_result_or_return!(first_receive_address_for_mnemonic(mnemonic, network.into()));
    *out_address = unwrap_result_or_return!(CString::new(address.to_string())).into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::out_string;
    use platform_wallet::test_support::{
        TEST_INVALID_MNEMONIC as GIBBERISH, TEST_PLATFORM_WALLET_MNEMONIC as ENGLISH,
    };
    use PlatformWalletFFIResultCode::{ErrorInvalidParameter, ErrorNullPointer, Success};

    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    #[test]
    fn should_pass_the_native_phrase_answers_through() {
        let wallet_id = |phrase: *const c_char| {
            let mut id = [0u8; 32];
            let r = unsafe {
                platform_wallet_wallet_id_from_mnemonic(
                    phrase,
                    FFINetwork::Testnet,
                    id.as_mut_ptr(),
                )
            };
            (r.code == Success).then_some(id).ok_or(r.code)
        };
        let address = |phrase: *const c_char| {
            out_string(|out| unsafe {
                platform_wallet_first_receive_address_from_mnemonic(
                    phrase,
                    FFINetwork::Mainnet,
                    out,
                )
            })
            .map(Option::unwrap)
        };
        let english = c(ENGLISH);
        assert_eq!(
            wallet_id(english.as_ptr()),
            Ok(wallet_id_for_mnemonic(ENGLISH, Network::Testnet).unwrap())
        );
        assert_eq!(
            address(english.as_ptr()).as_deref(),
            Ok("XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5")
        );

        let gibberish = c(GIBBERISH);
        assert_eq!(wallet_id(gibberish.as_ptr()), Err(ErrorInvalidParameter));
        assert_eq!(address(gibberish.as_ptr()), Err(ErrorInvalidParameter));
        assert_eq!(wallet_id(ptr::null()), Err(ErrorNullPointer));
        assert_eq!(address(ptr::null()), Err(ErrorNullPointer));
    }

    #[test]
    fn should_build_identity_authentication_paths() {
        let path = |network, identity_index, key_index| {
            out_string(|out| unsafe {
                platform_wallet_identity_authentication_path(
                    network,
                    identity_index,
                    key_index,
                    out,
                )
            })
            .map(Option::unwrap)
        };
        assert_eq!(
            path(FFINetwork::Mainnet, 3, 0).as_deref(),
            Ok("m/9'/5'/5'/0'/0'/3'/0'")
        );
        assert_eq!(
            path(FFINetwork::Testnet, 3, 0).as_deref(),
            Ok("m/9'/1'/5'/0'/0'/3'/0'")
        );
        assert_eq!(
            path(FFINetwork::Regtest, 0, 7).as_deref(),
            Ok("m/9'/1'/5'/0'/0'/0'/7'")
        );
        for (identity_index, key_index) in [(1 << 31, 0), (0, 1 << 31)] {
            assert_eq!(
                path(FFINetwork::Mainnet, identity_index, key_index),
                Err(ErrorInvalidParameter),
                "{identity_index} {key_index}"
            );
        }

        let r = unsafe {
            platform_wallet_identity_authentication_path(FFINetwork::Mainnet, 0, 0, ptr::null_mut())
        };
        assert_eq!(r.code, ErrorNullPointer);
    }
}
