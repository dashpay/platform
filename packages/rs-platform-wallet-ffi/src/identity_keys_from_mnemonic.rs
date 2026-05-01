//! Mnemonic-driven identity-registration key derivation.

use std::ffi::CString;

use dashcore::secp256k1::Secp256k1;
use dashcore::PrivateKey as DashPrivateKey;
use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dip9::{
    IDENTITY_AUTHENTICATION_PATH_MAINNET, IDENTITY_AUTHENTICATION_PATH_TESTNET,
};
use key_wallet::mnemonic::{Language, Mnemonic};
use zeroize::Zeroizing;

use crate::error::*;
use crate::identity_key_preview::IdentityKeyPreviewFFI;
use crate::identity_registration_with_signer::IdentityRegistrationKeyDerivationsFFI;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};

/// Parse a BIP-39 mnemonic against every supported wordlist.
pub(crate) fn parse_mnemonic_any_language(phrase: &str) -> Result<Mnemonic, &'static str> {
    const LANGUAGES: [Language; 10] = [
        Language::English,
        Language::Spanish,
        Language::French,
        Language::Italian,
        Language::Japanese,
        Language::Korean,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
        Language::Czech,
        Language::Portuguese,
    ];
    for lang in LANGUAGES {
        if let Ok(m) = Mnemonic::from_phrase(phrase, lang) {
            return Ok(m);
        }
    }
    Err("phrase does not match any supported BIP-39 wordlist")
}

/// Build the DIP-9 identity-authentication derivation path
/// `m/9'/coin'/5'/0'/0'/identity_index'/key_index'`.
pub(crate) fn identity_auth_derivation_path(
    network: Network,
    identity_index: u32,
    key_index: u32,
) -> Result<DerivationPath, String> {
    let base_path: DerivationPath = match network {
        Network::Mainnet => IDENTITY_AUTHENTICATION_PATH_MAINNET,
        _ => IDENTITY_AUTHENTICATION_PATH_TESTNET,
    }
    .into();
    let key_type_index: u32 = 0;
    Ok(base_path.extend([
        ChildNumber::from_hardened_idx(key_type_index)
            .map_err(|e| format!("invalid key_type_index: {e}"))?,
        ChildNumber::from_hardened_idx(identity_index)
            .map_err(|e| format!("invalid identity_index {identity_index}: {e}"))?,
        ChildNumber::from_hardened_idx(key_index)
            .map_err(|e| format!("invalid key_index {key_index}: {e}"))?,
    ]))
}

/// Derive `key_count` identity-registration keys from
/// `(mnemonic, passphrase, network)`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_derive_identity_keys_from_mnemonic(
    mnemonic_cstr: *const std::os::raw::c_char,
    passphrase_cstr: *const std::os::raw::c_char,
    network: FFINetwork,
    identity_index: u32,
    key_count: u32,
    out_rows: *mut IdentityRegistrationKeyDerivationsFFI,
) -> PlatformWalletFFIResult {
    use std::ffi::CStr;

    check_ptr!(out_rows);
    *out_rows = IdentityRegistrationKeyDerivationsFFI {
        items: std::ptr::null_mut(),
        count: 0,
    };

    check_ptr!(mnemonic_cstr);

    if key_count == 0 {
        return PlatformWalletFFIResult::ok();
    }

    let mnemonic_str = unwrap_result_or_return!(CStr::from_ptr(mnemonic_cstr).to_str());
    let passphrase_str: &str = if passphrase_cstr.is_null() {
        ""
    } else {
        unwrap_result_or_return!(CStr::from_ptr(passphrase_cstr).to_str())
    };

    let mnemonic = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(passphrase_str));

    let kw_network: Network = network.into();
    let master = unwrap_result_or_return!(ExtendedPrivKey::new_master(kw_network, seed.as_ref()));
    let secp = Secp256k1::new();

    let mut rows: Vec<IdentityKeyPreviewFFI> = Vec::with_capacity(key_count as usize);

    let cleanup = |rows: Vec<IdentityKeyPreviewFFI>| {
        for row in rows {
            if !row.derivation_path.is_null() {
                let _ = CString::from_raw(row.derivation_path);
            }
            if !row.public_key.is_null() && row.public_key_len > 0 {
                let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
            }
            if !row.private_key_wif.is_null() {
                let _ = CString::from_raw(row.private_key_wif);
            }
        }
    };

    for key_index in 0..key_count {
        let path = match identity_auth_derivation_path(kw_network, identity_index, key_index) {
            Ok(p) => p,
            Err(detail) => {
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!(
                        "derive_identity_keys_from_mnemonic: path build failed at \
                         (identity={identity_index}, key={key_index}): {detail}"
                    ),
                );
            }
        };

        let derived = match master.derive_priv(&secp, &path) {
            Ok(d) => d,
            Err(e) => {
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorWalletOperation,
                    format!(
                        "derive_identity_keys_from_mnemonic: derive_priv failed at \
                         (identity={identity_index}, key={key_index}): {e}"
                    ),
                );
            }
        };
        let extended_pub = ExtendedPubKey::from_priv(&secp, &derived);
        let public_key = extended_pub.public_key;

        let path_cstring = match CString::new(path.to_string()) {
            Ok(s) => s,
            Err(e) => {
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                    format!("derivation path contained NUL byte: {e}"),
                );
            }
        };

        let pub_bytes: [u8; 33] = public_key.serialize();
        let mut pub_box: Box<[u8]> = pub_bytes.to_vec().into_boxed_slice();
        let pub_ptr = pub_box.as_mut_ptr();
        let pub_len = pub_box.len();
        std::mem::forget(pub_box);

        let dash_private = DashPrivateKey {
            compressed: true,
            network: kw_network,
            inner: derived.private_key,
        };
        let wif_cstring = match CString::new(dash_private.to_wif()) {
            Ok(s) => s,
            Err(e) => {
                drop(Vec::from_raw_parts(pub_ptr, pub_len, pub_len));
                drop(path_cstring);
                cleanup(rows);
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUtf8Conversion,
                    format!("WIF string contained NUL byte: {e}"),
                );
            }
        };

        rows.push(IdentityKeyPreviewFFI {
            identity_index,
            derivation_path: path_cstring.into_raw(),
            public_key: pub_ptr,
            public_key_len: pub_len,
            private_key_wif: wif_cstring.into_raw(),
            private_key_bytes: derived.private_key.secret_bytes(),
        });
    }

    let mut boxed = rows.into_boxed_slice();
    let items_ptr = boxed.as_mut_ptr();
    let items_count = boxed.len();
    std::mem::forget(boxed);

    *out_rows = IdentityRegistrationKeyDerivationsFFI {
        items: items_ptr,
        count: items_count,
    };
    PlatformWalletFFIResult::ok()
}

/// Release a [`IdentityRegistrationKeyDerivationsFFI`] previously
/// populated by [`dash_sdk_derive_identity_keys_from_mnemonic`].
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_identity_keys_from_mnemonic_free(
    rows: *mut IdentityRegistrationKeyDerivationsFFI,
) {
    if rows.is_null() {
        return;
    }
    let owned = std::mem::replace(
        &mut *rows,
        IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        },
    );
    if owned.items.is_null() || owned.count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(owned.items, owned.count);
    for row in slice.iter_mut() {
        if !row.derivation_path.is_null() {
            let _ = CString::from_raw(row.derivation_path);
        }
        if !row.public_key.is_null() && row.public_key_len > 0 {
            let _ = Vec::from_raw_parts(row.public_key, row.public_key_len, row.public_key_len);
        }
        if !row.private_key_wif.is_null() {
            let mut wif = CString::from_raw(row.private_key_wif).into_bytes_with_nul();
            zeroize::Zeroize::zeroize(&mut wif);
            row.private_key_wif = std::ptr::null_mut();
        }
        zeroize::Zeroize::zeroize(&mut row.private_key_bytes);
    }
    let _ = Box::from_raw(slice as *mut [IdentityKeyPreviewFFI]);
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGLISH_PHRASE: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn derives_three_keys_with_correct_shape() {
        let mnemonic = std::ffi::CString::new(ENGLISH_PHRASE).unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                FFINetwork::Testnet,
                7,
                3,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.count, 3);
        assert!(!out.items.is_null());

        for i in 0..out.count {
            let row = unsafe { &*out.items.add(i) };
            assert_eq!(row.identity_index, 7);
            assert_eq!(row.public_key_len, 33);
            assert!(!row.public_key.is_null());
            assert!(!row.derivation_path.is_null());

            let path = unsafe { std::ffi::CStr::from_ptr(row.derivation_path) }
                .to_str()
                .unwrap();
            assert!(
                path.starts_with("m/9'/1'/5'/0'/0'/7'/"),
                "unexpected path prefix: {path}"
            );
            assert!(
                path.ends_with(&format!("/{i}'")),
                "unexpected path tail: {path}"
            );

            assert!(
                row.private_key_bytes.iter().any(|b| *b != 0),
                "secret bytes are all zero at row {i}"
            );
        }

        unsafe { dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out) };
        assert!(out.items.is_null());
        assert_eq!(out.count, 0);
    }

    #[test]
    fn derives_mainnet_path_uses_coin_type_5() {
        let mnemonic = std::ffi::CString::new(ENGLISH_PHRASE).unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                FFINetwork::Mainnet,
                0,
                1,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.count, 1);

        let row = unsafe { &*out.items };
        let path = unsafe { std::ffi::CStr::from_ptr(row.derivation_path) }
            .to_str()
            .unwrap();
        assert_eq!(path, "m/9'/5'/5'/0'/0'/0'/0'");

        unsafe { dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out) };
    }

    #[test]
    fn key_count_zero_is_noop_success() {
        let mnemonic = std::ffi::CString::new(ENGLISH_PHRASE).unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                FFINetwork::Testnet,
                0,
                0,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.count, 0);
        assert!(out.items.is_null());
        unsafe { dash_sdk_derive_identity_keys_from_mnemonic_free(&mut out) };
    }

    #[test]
    fn rejects_invalid_mnemonic() {
        let mnemonic = std::ffi::CString::new("not a real bip39 phrase at all here").unwrap();
        let mut out = IdentityRegistrationKeyDerivationsFFI {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let result = unsafe {
            dash_sdk_derive_identity_keys_from_mnemonic(
                mnemonic.as_ptr(),
                std::ptr::null(),
                FFINetwork::Testnet,
                0,
                3,
                &mut out,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert!(out.items.is_null());
        assert_eq!(out.count, 0);
    }
}
