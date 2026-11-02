//! Stateless derivation helpers.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::str::FromStr;

use dashcore::{Address, PublicKey};
use key_wallet::bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey, KeyDerivationType};
use key_wallet::mnemonic::Mnemonic;
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use key_wallet::wallet::Wallet;
use platform_wallet::CoreAddressPrivateKey;
use zeroize::{Zeroize, Zeroizing};

use crate::address_private_key::{write_private_key, AddressPrivateKeyFFI};
use crate::error::*;
use crate::manager_diagnostics::account_type_from_spec_ref;
use crate::types::{FFINetwork, Network};
use crate::wallet_restore_types::AccountSpecFFI;
use crate::{check_ptr, unwrap_result_or_return};

fn parse_mnemonic_any_language(phrase: &str) -> Result<Mnemonic, &'static str> {
    // Upstream's `from_phrase` IS the auto-detecting parse since
    // rust-dashcore#981 — one path, English diagnostics preserved when
    // nothing matches. This wrapper survives only to narrow the error to
    // the `&'static str` its callers report.
    Mnemonic::from_phrase(phrase).map_err(|_| "phrase does not match any supported BIP-39 wordlist")
}

/// The master key of `phrase` and `passphrase` on `network`. The seed is wiped
/// before it returns.
fn master_from_phrase(
    phrase: &str,
    passphrase: &str,
    network: Network,
) -> Result<ExtendedPrivKey, PlatformWalletFFIResult> {
    let mnemonic = parse_mnemonic_any_language(phrase)?;
    let seed = Zeroizing::new(mnemonic.to_seed(passphrase));
    Ok(ExtendedPrivKey::new_master(network, &*seed)?)
}

fn parse_path(path: &str) -> Result<DerivationPath, String> {
    DerivationPath::from_str(path).map_err(|e| format!("invalid derivation path '{path}': {e}"))
}

/// The P2PKH addresses of `phrase` at `paths` on `network`, joined with `\n` in
/// path order. The seed is computed once for all paths.
fn joined_addresses(
    phrase: &str,
    network: Network,
    paths: &[DerivationPath],
) -> Result<CString, PlatformWalletFFIResult> {
    let mut master = master_from_phrase(phrase, "", network)?;
    let addresses: Result<Vec<String>, PlatformWalletFFIResult> = paths
        .iter()
        .map(|path| {
            let mut derived = master.derive_priv(path)?;
            let public_key = PublicKey::new(ExtendedPubKey::from_priv(&derived).public_key);
            derived.private_key.zeroize();
            Ok(Address::p2pkh(&public_key, network).to_string())
        })
        .collect();
    master.private_key.zeroize();
    Ok(CString::new(addresses?.join("\n"))?)
}

/// The base58 extended public key of `phrase` at `path` on `network`. The seed
/// and the derived private keys are wiped before it returns.
fn extended_pub_key(
    phrase: &str,
    network: Network,
    path: &DerivationPath,
) -> Result<CString, PlatformWalletFFIResult> {
    let mut master = master_from_phrase(phrase, "", network)?;
    let derived = master.derive_priv(path);
    master.private_key.zeroize();
    let mut derived = derived?;
    let xpub = ExtendedPubKey::from_priv(&derived);
    derived.private_key.zeroize();
    Ok(CString::new(xpub.to_string())?)
}

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
/// intermediate seed are wrapped in [`Zeroizing`] inside this
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

    let master =
        unwrap_result_or_return!(master_from_phrase(mnemonic_str, passphrase_str, network));

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
/// On success the caller owns `*out_path` and must free it via
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

    let path = DerivationPath::identity_authentication_path(
        network.into(),
        KeyDerivationType::ECDSA,
        identity_index,
        key_index,
    );
    let c_path = unwrap_result_or_return!(CString::new(path.to_string()));
    *out_path = c_path.into_raw();
    PlatformWalletFFIResult::ok()
}

/// Returns the derivation path of the account `spec` names on `network`, such
/// as `m/9'/5'/3'/1'` for the provider voting keys on mainnet.
///
/// The path comes from key-wallet's account definitions. Only the account
/// fields of `spec` are read, and its xpub is ignored. On success the caller
/// owns `*out_path` and must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `spec` must point to a valid [`AccountSpecFFI`] and `out_path` to writable
/// memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_account_path(
    network: FFINetwork,
    spec: *const AccountSpecFFI,
    out_path: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_path);
    *out_path = ptr::null_mut();
    check_ptr!(spec);

    let account_type = unwrap_result_or_return!(account_type_from_spec_ref(&*spec));
    let path = unwrap_result_or_return!(account_type.derivation_path(network.into()));
    *out_path = unwrap_result_or_return!(CString::new(path.to_string())).into_raw();
    PlatformWalletFFIResult::ok()
}

/// Writes the 32-byte id the platform wallet manager gives the wallet of
/// `mnemonic` on `network`, without creating or registering a wallet.
///
/// The id is network-scoped, so one phrase has a distinct id per network. The
/// phrase may be in any supported BIP-39 language and has no passphrase, like
/// every wallet the manager creates.
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

    let phrase = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let mnemonic = unwrap_result_or_return!(parse_mnemonic_any_language(phrase));
    let seed = Zeroizing::new(mnemonic.to_seed(""));
    let root = unwrap_result_or_return!(RootExtendedPrivKey::new_master(&*seed));
    let wallet_id = Wallet::compute_wallet_id_from_root_extended_pub_key(
        &root.to_root_extended_pub_key(),
        Some(network.into()),
    );
    ptr::copy_nonoverlapping(wallet_id.as_ptr(), out_wallet_id, 32);
    PlatformWalletFFIResult::ok()
}

/// Returns the BIP-44 P2PKH address of `mnemonic` at
/// `account_index` / `is_change` / `address_index` on `network`.
///
/// Only the address leaves this function. The seed and the derived private key
/// are wiped before it returns. On success the caller owns `*out_address` and
/// must free it via [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `mnemonic` must point to a valid null-terminated UTF-8 C string and
/// `out_address` to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_bip44_address_from_mnemonic(
    mnemonic: *const c_char,
    network: FFINetwork,
    account_index: u32,
    is_change: bool,
    address_index: u32,
    out_address: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_address);
    *out_address = ptr::null_mut();
    check_ptr!(mnemonic);

    let phrase = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let network: Network = network.into();
    let path =
        DerivationPath::bip_44_payment_path(network, account_index, is_change, address_index);
    *out_address = unwrap_result_or_return!(joined_addresses(phrase, network, &[path])).into_raw();
    PlatformWalletFFIResult::ok()
}

/// Returns the P2PKH addresses of `mnemonic` on `network` at the
/// newline-separated derivation `paths`, as one newline-separated string in
/// the same order.
///
/// The seed is computed once for the whole list, and only the addresses leave
/// this function. Any path that does not parse fails the call with
/// `ErrorInvalidParameter`. On success the caller owns `*out_addresses` and
/// must free it via [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `mnemonic` and `paths` must point to valid null-terminated UTF-8 C strings
/// and `out_addresses` to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_addresses_from_mnemonic(
    mnemonic: *const c_char,
    network: FFINetwork,
    paths: *const c_char,
    out_addresses: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_addresses);
    *out_addresses = ptr::null_mut();
    check_ptr!(mnemonic);
    check_ptr!(paths);

    let phrase = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let paths = unwrap_result_or_return!(CStr::from_ptr(paths).to_str());
    let paths = unwrap_result_or_return!(paths
        .split('\n')
        .map(parse_path)
        .collect::<Result<Vec<_>, _>>());
    *out_addresses =
        unwrap_result_or_return!(joined_addresses(phrase, network.into(), &paths)).into_raw();
    PlatformWalletFFIResult::ok()
}

/// Derives the private key of `mnemonic` at the derivation `path` on
/// `network`, as hex and WIF.
///
/// A path that does not parse fails the call with `ErrorInvalidParameter`.
/// On error `out` is left empty. Release it with
/// [`crate::address_private_key::platform_wallet_address_private_key_free`],
/// which zeroizes both strings.
///
/// # Safety
/// `mnemonic` and `path` must point to valid null-terminated UTF-8 C strings
/// and `out` must be a valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_private_key_from_mnemonic(
    mnemonic: *const c_char,
    network: FFINetwork,
    path: *const c_char,
    out: *mut AddressPrivateKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out);
    *out = AddressPrivateKeyFFI::empty();
    check_ptr!(mnemonic);
    check_ptr!(path);

    let phrase = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let path = unwrap_result_or_return!(parse_path(unwrap_result_or_return!(
        CStr::from_ptr(path).to_str()
    )));
    let network: Network = network.into();
    let mut master = unwrap_result_or_return!(master_from_phrase(phrase, "", network));
    let key = CoreAddressPrivateKey::derive(&master, path, network);
    master.private_key.zeroize();
    write_private_key(out, unwrap_result_or_return!(key))
}

/// Returns the base58 extended public key (`xpub` on mainnet, `tpub`
/// elsewhere) of `mnemonic` at the derivation `path` on `network`, such as a
/// BIP-44 account path `m/44'/5'/0'`.
///
/// Only the public key leaves this function. A path that does not parse fails
/// the call with `ErrorInvalidParameter`. On success the caller owns
/// `*out_xpub` and must free it via
/// [`crate::xpub_render::platform_wallet_free_string`].
///
/// # Safety
/// `mnemonic` and `path` must point to valid null-terminated UTF-8 C strings
/// and `out_xpub` to writable memory for one `*mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_xpub_from_mnemonic(
    mnemonic: *const c_char,
    network: FFINetwork,
    path: *const c_char,
    out_xpub: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_xpub);
    *out_xpub = ptr::null_mut();
    check_ptr!(mnemonic);
    check_ptr!(path);

    let phrase = unwrap_result_or_return!(CStr::from_ptr(mnemonic).to_str());
    let path = unwrap_result_or_return!(parse_path(unwrap_result_or_return!(
        CStr::from_ptr(path).to_str()
    )));
    *out_xpub =
        unwrap_result_or_return!(extended_pub_key(phrase, network.into(), &path)).into_raw();
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_private_key::platform_wallet_address_private_key_free;
    use crate::persistence::build_account_spec_ffi;
    use crate::runtime::runtime;
    use crate::test_fixtures::mock_manager;
    use crate::xpub_render::{platform_wallet_account_xpub_to_string, platform_wallet_free_string};
    use bincode::config;
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use std::ptr::NonNull;
    use PlatformWalletFFIResultCode::{ErrorInvalidParameter, ErrorNullPointer, Success};

    const ENGLISH: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const FRENCH: &str =
        "abaisser agréable inductif agréable éligible achat bolide boucle amateur exister dérober bloquer";
    const GIBBERISH: &str = "definitely not a bip39 phrase at all zz";

    /// Runs `call` with a dangling out pointer and returns the C string it
    /// wrote, or the failure code. A failure must leave the pointer null.
    fn out_string(
        call: impl FnOnce(*mut *mut c_char) -> PlatformWalletFFIResult,
    ) -> Result<String, PlatformWalletFFIResultCode> {
        let mut out: *mut c_char = NonNull::dangling().as_ptr();
        let r = call(&mut out);
        if r.code != Success {
            assert!(out.is_null());
            return Err(r.code);
        }
        let string = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
        unsafe { platform_wallet_free_string(out) };
        Ok(string)
    }

    fn c(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    fn wallet_id(
        phrase: *const c_char,
        network: FFINetwork,
    ) -> Result<[u8; 32], PlatformWalletFFIResultCode> {
        let mut id = [0u8; 32];
        let r =
            unsafe { platform_wallet_wallet_id_from_mnemonic(phrase, network, id.as_mut_ptr()) };
        if r.code != Success {
            return Err(r.code);
        }
        Ok(id)
    }

    fn bip44_address(
        phrase: *const c_char,
        network: FFINetwork,
        index: u32,
    ) -> Result<String, PlatformWalletFFIResultCode> {
        out_string(|out| unsafe {
            platform_wallet_bip44_address_from_mnemonic(phrase, network, 0, false, index, out)
        })
    }

    fn addresses(
        phrase: *const c_char,
        network: FFINetwork,
        paths: &str,
    ) -> Result<Vec<String>, PlatformWalletFFIResultCode> {
        let paths = c(paths);
        let joined = out_string(|out| unsafe {
            platform_wallet_addresses_from_mnemonic(phrase, network, paths.as_ptr(), out)
        })?;
        Ok(joined.split('\n').map(str::to_owned).collect())
    }

    fn xpub(
        phrase: *const c_char,
        network: FFINetwork,
        path: &str,
    ) -> Result<String, PlatformWalletFFIResultCode> {
        let path = c(path);
        out_string(|out| unsafe {
            platform_wallet_xpub_from_mnemonic(phrase, network, path.as_ptr(), out)
        })
    }

    /// The (hex, WIF) private key at `path`.
    fn private_key(
        phrase: *const c_char,
        network: FFINetwork,
        path: &str,
    ) -> Result<(String, String), PlatformWalletFFIResultCode> {
        let path = c(path);
        let mut out = AddressPrivateKeyFFI {
            private_key_hex: NonNull::dangling().as_ptr(),
            private_key_wif: NonNull::dangling().as_ptr(),
        };
        unsafe {
            let r =
                platform_wallet_private_key_from_mnemonic(phrase, network, path.as_ptr(), &mut out);
            if r.code != Success {
                assert!(out.private_key_hex.is_null() && out.private_key_wif.is_null());
                return Err(r.code);
            }
            let read = |p| CStr::from_ptr(p).to_str().unwrap().to_owned();
            let key = (read(out.private_key_hex), read(out.private_key_wif));
            platform_wallet_address_private_key_free(&mut out);
            assert!(out.private_key_hex.is_null() && out.private_key_wif.is_null());
            Ok(key)
        }
    }

    #[test]
    fn wallet_id_matches_the_id_of_a_created_wallet() {
        for phrase in [ENGLISH, FRENCH] {
            for network in [FFINetwork::Mainnet, FFINetwork::Testnet, FFINetwork::Devnet] {
                let wallet = Wallet::from_mnemonic(
                    Mnemonic::from_phrase(phrase).unwrap(),
                    network.into(),
                    WalletAccountCreationOptions::Default,
                )
                .unwrap();
                assert_eq!(
                    wallet_id(c(phrase).as_ptr(), network),
                    Ok(wallet.wallet_id),
                    "{network:?}"
                );
            }
        }
        let english = c(ENGLISH);
        assert_ne!(
            wallet_id(english.as_ptr(), FFINetwork::Mainnet),
            wallet_id(english.as_ptr(), FFINetwork::Testnet)
        );
        assert_eq!(
            wallet_id(c(GIBBERISH).as_ptr(), FFINetwork::Mainnet),
            Err(ErrorInvalidParameter)
        );
        assert_eq!(
            wallet_id(ptr::null(), FFINetwork::Mainnet),
            Err(ErrorNullPointer)
        );
    }

    #[test]
    fn address_helpers_match_pinned_vectors() {
        let (english, french) = (c(ENGLISH), c(FRENCH));
        for (phrase, network, expected) in [
            (
                &english,
                FFINetwork::Mainnet,
                "XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5",
            ),
            (
                &french,
                FFINetwork::Mainnet,
                "XvdJKc1Qf9QWHfHTp8rpiNZ3ZWLGcFnoqV",
            ),
            (
                &french,
                FFINetwork::Testnet,
                "yiz7bBUGQUJ4cTfUWqwwPq7g8XTc7ynEEw",
            ),
        ] {
            assert_eq!(
                bip44_address(phrase.as_ptr(), network, 0).as_deref(),
                Ok(expected)
            );
        }
        assert_eq!(
            addresses(english.as_ptr(), FFINetwork::Mainnet, "m/44'/5'/0'/0/0"),
            Ok(vec!["XoJA8qE3N2Y3jMLEtZ3vcN42qseZ8LvFf5".to_owned()])
        );
        let bip44 = |index| bip44_address(french.as_ptr(), FFINetwork::Testnet, index).unwrap();
        assert_eq!(
            addresses(
                french.as_ptr(),
                FFINetwork::Testnet,
                "m/44'/1'/0'/0/0\nm/44'/1'/0'/0/7\nm/44'/1'/0'/0/0"
            ),
            Ok(vec![bip44(0), bip44(7), bip44(0)])
        );

        for bad_paths in [
            "",
            "m/44'/1'/0'/0/0\n",
            "m/44'/1'/0'/0/x",
            "m/44'/1'/0'/0/0\n44'",
        ] {
            assert_eq!(
                addresses(english.as_ptr(), FFINetwork::Testnet, bad_paths),
                Err(ErrorInvalidParameter),
                "{bad_paths:?}"
            );
        }
        assert_eq!(
            addresses(c(GIBBERISH).as_ptr(), FFINetwork::Testnet, "m/0"),
            Err(ErrorInvalidParameter)
        );
        assert_eq!(
            addresses(ptr::null(), FFINetwork::Mainnet, "m/0"),
            Err(ErrorNullPointer)
        );
        assert_eq!(
            bip44_address(ptr::null(), FFINetwork::Mainnet, 0),
            Err(ErrorNullPointer)
        );
    }

    #[test]
    fn path_helpers_match_the_keys_of_a_created_wallet() {
        let manager = mock_manager();
        let wallet = runtime()
            .block_on(manager.create_wallet_from_mnemonic(
                ENGLISH,
                Network::Testnet,
                WalletAccountCreationOptions::Default,
                Some(0),
            ))
            .unwrap();
        let english = c(ENGLISH);
        // BIP44 receive and change, then the DIP-3 provider voting and owner pools.
        let paths = [
            "m/44'/1'/0'/0/0",
            "m/44'/1'/0'/1/3",
            "m/9'/1'/3'/1'/0",
            "m/9'/1'/3'/2'/4",
        ];
        // The app's wallets are external-signable, so `coreAddressPrivateKey`
        // derives from the master its mnemonic resolver builds.
        let seed = Mnemonic::from_phrase(ENGLISH).unwrap().to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed).unwrap();
        let derived = addresses(english.as_ptr(), FFINetwork::Testnet, &paths.join("\n")).unwrap();
        for (path, address) in paths.iter().zip(&derived) {
            let expected = wallet
                .derive_core_address_private_key(address, Some(&master))
                .unwrap();
            assert_eq!(expected.derivation_path.to_string(), *path);
            assert_eq!(
                private_key(english.as_ptr(), FFINetwork::Testnet, path),
                Ok((hex::encode(&expected.private_key[..]), expected.wif.clone())),
                "{path}"
            );
        }

        let account_xpub = wallet
            .state_blocking()
            .wallet()
            .get_bip44_account(0)
            .unwrap()
            .account_xpub;
        let persisted = bincode::encode_to_vec(account_xpub, config::standard()).unwrap();
        let rendered = out_string(|out| unsafe {
            platform_wallet_account_xpub_to_string(persisted.as_ptr(), persisted.len(), out)
        })
        .unwrap();
        assert!(rendered.starts_with("tpub"), "{rendered}");
        assert_eq!(
            xpub(english.as_ptr(), FFINetwork::Testnet, "m/44'/1'/0'"),
            Ok(rendered)
        );
        assert!(xpub(english.as_ptr(), FFINetwork::Mainnet, "m/44'/5'/0'")
            .unwrap()
            .starts_with("xpub"));
        let (_, mainnet_wif) =
            private_key(english.as_ptr(), FFINetwork::Mainnet, "m/44'/5'/0'/0/0").unwrap();
        assert!(mainnet_wif.starts_with('X'));

        assert_eq!(
            xpub(english.as_ptr(), FFINetwork::Testnet, "m/44'/1'/x'"),
            Err(ErrorInvalidParameter)
        );
        assert_eq!(
            private_key(english.as_ptr(), FFINetwork::Testnet, "m/44'/1'/0'/0/x"),
            Err(ErrorInvalidParameter)
        );
        assert_eq!(
            xpub(ptr::null(), FFINetwork::Testnet, "m/0"),
            Err(ErrorNullPointer)
        );
        assert_eq!(
            private_key(ptr::null(), FFINetwork::Testnet, "m/0"),
            Err(ErrorNullPointer)
        );
    }

    #[test]
    fn account_path_matches_the_key_wallet_accounts() {
        let path = |network, spec: *const AccountSpecFFI| {
            out_string(|out| unsafe { platform_wallet_account_path(network, spec, out) })
        };
        let bip44 = |index| AccountType::Standard {
            index,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        for (network, account, expected) in [
            (FFINetwork::Mainnet, bip44(0), "m/44'/5'/0'"),
            (FFINetwork::Testnet, bip44(2), "m/44'/1'/2'"),
            (
                FFINetwork::Mainnet,
                AccountType::ProviderVotingKeys,
                "m/9'/5'/3'/1'",
            ),
            (
                FFINetwork::Testnet,
                AccountType::ProviderOwnerKeys,
                "m/9'/1'/3'/2'",
            ),
        ] {
            let spec = build_account_spec_ffi(&account, &[]);
            assert_eq!(path(network, &spec).as_deref(), Ok(expected), "{account:?}");
        }
        let mut unknown = build_account_spec_ffi(&AccountType::ProviderVotingKeys, &[]);
        unknown.type_tag = u8::MAX;
        assert_eq!(
            path(FFINetwork::Mainnet, &unknown),
            Err(ErrorInvalidParameter)
        );
        assert_eq!(
            path(FFINetwork::Mainnet, ptr::null()),
            Err(ErrorNullPointer)
        );
    }

    #[test]
    fn identity_authentication_path() {
        let path = |network, identity_index, key_index| {
            out_string(|out| unsafe {
                platform_wallet_identity_authentication_path(
                    network,
                    identity_index,
                    key_index,
                    out,
                )
            })
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

        let r = unsafe {
            platform_wallet_identity_authentication_path(FFINetwork::Mainnet, 0, 0, ptr::null_mut())
        };
        assert_eq!(r.code, ErrorNullPointer);
    }
}
