//! DashPay Connect key derivation at the DIP-13 sub-feature paths
//! (dashpay/dips#191), driven by the resolver-backed seed.
//!
//! Both DashPay Connect v2 keys live under new DIP-13 sub-features with
//! DIP-14 256-bit hardened leaves, so no wallet-local counter is an input
//! and two devices restored from one seed derive the same key:
//!
//! ```text
//! session auth:   m / 9' / coin' / 5' / 6' / 0' / identityId' / requestId'
//! app encryption: m / 9' / coin' / 5' / 7' / 0' / identityId' / contractId' / purpose'
//! ```
//!
//! The wallet derives every key; the app never sees the seed. This module
//! is the FFI mirror of
//! [`platform_wallet::wallet::identity::network::derive_connect_keypair_from_master`]
//! and follows the resolver contract of
//! [`crate::dash_sdk_derive_identity_key_at_slot_with_resolver`]: the
//! mnemonic is pulled through the Swift/Kotlin-owned `MnemonicResolver`
//! for the duration of the call only, in a zeroized buffer.

use std::ffi::c_void;
use std::os::raw::c_char;

use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_connect_keypair_from_master;
use zeroize::Zeroizing;

use crate::error::*;
use crate::identity_keys_from_mnemonic::parse_mnemonic_any_language;
use crate::types::{read_identifier, FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};
use rs_sdk_ffi::{
    mnemonic_resolver_result, MnemonicResolverHandle, MNEMONIC_RESOLVER_BUFFER_CAPACITY,
};

/// `sub_feature` for the DashPay Connect session authentication key. The
/// leaf is the connect request id (`hash256` of the app's ephemeral public
/// key); `purpose` is `0`.
pub const CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION: u32 =
    platform_wallet::wallet::identity::network::CONNECT_SUB_FEATURE_SESSION_AUTHENTICATION;

/// `sub_feature` for the DashPay Connect app encryption key pair. The leaf
/// is the id of the data contract the key is bound to; `purpose` is the
/// DPP purpose discriminant, `1` ENCRYPTION or `2` DECRYPTION.
pub const CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION: u32 =
    platform_wallet::wallet::identity::network::CONNECT_SUB_FEATURE_APP_ENCRYPTION;

/// `purpose` for the ENCRYPTION half of an app encryption pair (the DPP
/// `Purpose::ENCRYPTION` discriminant).
pub const CONNECT_KEY_PURPOSE_ENCRYPTION: u32 = 1;

/// `purpose` for the DECRYPTION half of an app encryption pair (the DPP
/// `Purpose::DECRYPTION` discriminant).
pub const CONNECT_KEY_PURPOSE_DECRYPTION: u32 = 2;

/// A derived DashPay Connect keypair. Plain old data: the caller copies
/// what it needs and calls [`dash_sdk_derive_connect_key_free`] to wipe
/// the private scalar.
#[repr(C)]
pub struct ConnectDerivedKeyFFI {
    /// Raw 32-byte secp256k1 private scalar. Inline so the free path can
    /// zeroize it without chasing a pointer.
    pub private_key_bytes: [u8; 32],
    /// Compressed secp256k1 public key, always 33 bytes.
    pub public_key_bytes: [u8; 33],
}

impl ConnectDerivedKeyFFI {
    pub const fn empty() -> Self {
        Self {
            private_key_bytes: [0u8; 32],
            public_key_bytes: [0u8; 33],
        }
    }
}

impl Default for ConnectDerivedKeyFFI {
    fn default() -> Self {
        Self::empty()
    }
}

/// Derive the DashPay Connect key at
/// `m/9'/coin'/5'/<sub_feature>'/0'/<identity_id>'/<leaf>'[/<purpose>']`
/// from the mnemonic the resolver returns for `wallet_id_bytes`.
///
/// - `network` picks the coin type (`5'` mainnet, `1'` otherwise), as the
///   existing identity derivations do.
/// - `sub_feature` is `6` (session authentication) or `7` (app
///   encryption); any 31-bit value derives, the protocol decides which
///   are meaningful.
/// - `identity_id_bytes` and `leaf_bytes` are 32-byte DIP-14 hardened
///   children: the identity's own id, and the request id or bound
///   contract id.
/// - `purpose` is `0` for no purpose level (the session authentication
///   path), or the DPP purpose discriminant of the half being derived:
///   `1` ENCRYPTION or `2` DECRYPTION, appended as one further hardened
///   child. The DIP-13 amendment defines exactly those two, so any other
///   value is refused with `ErrorInvalidParameter` rather than derived
///   into a key nothing will ever look for.
///
/// The derived key is written to `out_key`; call
/// [`dash_sdk_derive_connect_key_free`] when done with it.
///
/// # Safety
/// - `wallet_id_bytes`, `identity_id_bytes` and `leaf_bytes` must point to
///   32 readable bytes each.
/// - `mnemonic_resolver_handle` must be a live handle from
///   `dash_sdk_mnemonic_resolver_create`.
/// - `out_key` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_connect_key_with_resolver(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    sub_feature: u32,
    identity_id_bytes: *const u8,
    leaf_bytes: *const u8,
    purpose: u32,
    out_key: *mut ConnectDerivedKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_key);
    *out_key = ConnectDerivedKeyFFI::empty();

    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(leaf_bytes);

    let purpose = match purpose {
        0 => None,
        CONNECT_KEY_PURPOSE_ENCRYPTION | CONNECT_KEY_PURPOSE_DECRYPTION => Some(purpose),
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "purpose {other} is not a DashPay Connect key purpose (0 = none, \
                     1 = ENCRYPTION, 2 = DECRYPTION)"
                ),
            );
        }
    };

    let identity_id = unwrap_result_or_return!(read_identifier(identity_id_bytes));
    let leaf: [u8; 32] = std::slice::from_raw_parts(leaf_bytes, 32)
        .try_into()
        .expect("from_raw_parts(_, 32) always yields exactly 32 bytes");

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
    if mnemonic_len > MNEMONIC_RESOLVER_BUFFER_CAPACITY {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            "mnemonic resolver: reported length exceeds the FFI buffer capacity",
        );
    }

    let mnemonic_str = unwrap_result_or_return!(std::str::from_utf8(&mnemonic_buf[..mnemonic_len]));
    let mnemonic = unwrap_result_or_return!(parse_mnemonic_any_language(mnemonic_str));
    let seed: Zeroizing<[u8; 64]> = Zeroizing::new(mnemonic.to_seed(""));

    let kw_network: Network = network.into();
    let master = unwrap_result_or_return!(ExtendedPrivKey::new_master(kw_network, seed.as_ref()));

    let derived = unwrap_result_or_return!(derive_connect_keypair_from_master(
        &master,
        kw_network,
        sub_feature,
        &identity_id,
        leaf,
        purpose,
    ));

    (*out_key).private_key_bytes = *derived.private_key;
    (*out_key).public_key_bytes = derived.public_key;

    PlatformWalletFFIResult::ok()
}

/// Wipe a key populated by [`dash_sdk_derive_connect_key_with_resolver`].
/// The struct owns no heap memory; this zeroizes the private scalar in
/// place (a volatile write the optimizer cannot elide) and clears the
/// public key. Safe on a null pointer and on an already-wiped key.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_connect_key_free(out_key: *mut ConnectDerivedKeyFFI) {
    if out_key.is_null() {
        return;
    }
    use zeroize::Zeroize;
    (*out_key).private_key_bytes.zeroize();
    (*out_key).public_key_bytes = [0u8; 33];
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use key_wallet::mnemonic::Mnemonic;
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};
    use std::ffi::CStr;

    /// English BIP-39 test vector (all-zero entropy); the same fixture the
    /// platform-wallet derivation tests pin vectors for.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const IDENTITY: [u8; 32] = [0x35; 32];
    const LEAF: [u8; 32] = [0x6B; 32];

    unsafe extern "C" fn test_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = TEST_MNEMONIC.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn not_found_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        _out_buf: *mut c_char,
        _out_capacity: usize,
        _out_len: *mut usize,
    ) -> i32 {
        mnemonic_resolver_result::NOT_FOUND
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    fn derive(
        resolver: *mut MnemonicResolverHandle,
        network: FFINetwork,
        sub_feature: u32,
        leaf: [u8; 32],
        purpose: u32,
    ) -> (PlatformWalletFFIResult, ConnectDerivedKeyFFI) {
        let wallet_id = [0x07u8; 32];
        let mut out = ConnectDerivedKeyFFI::empty();
        let result = unsafe {
            dash_sdk_derive_connect_key_with_resolver(
                network,
                wallet_id.as_ptr(),
                resolver,
                sub_feature,
                IDENTITY.as_ptr(),
                leaf.as_ptr(),
                purpose,
                &mut out,
            )
        };
        (result, out)
    }

    /// The FFI derives exactly what the platform-wallet primitive derives
    /// from the same seed, so the two cannot drift; and the same inputs
    /// always give the same key.
    #[test]
    fn derives_the_library_vector_deterministically() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), test_resolve, noop_destroy)
        };

        let (result, first) = derive(
            resolver,
            FFINetwork::Testnet,
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            LEAF,
            0,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let (result, second) = derive(
            resolver,
            FFINetwork::Testnet,
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            LEAF,
            0,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(first.private_key_bytes, second.private_key_bytes);
        assert_eq!(first.public_key_bytes, second.public_key_bytes);

        let mnemonic = Mnemonic::from_phrase(TEST_MNEMONIC).unwrap();
        let master = ExtendedPrivKey::new_master(Network::Testnet, &mnemonic.to_seed("")).unwrap();
        let expected = derive_connect_keypair_from_master(
            &master,
            Network::Testnet,
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            &Identifier::from(IDENTITY),
            LEAF,
            None,
        )
        .unwrap();
        assert_eq!(first.private_key_bytes, *expected.private_key);
        assert_eq!(first.public_key_bytes, expected.public_key);

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// Leaf, purpose and sub-feature each change the key; `purpose == 0`
    /// appends nothing, so it differs from `purpose == 1` under the
    /// encryption sub-feature.
    #[test]
    fn different_leaves_purposes_and_sub_features_give_different_keys() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), test_resolve, noop_destroy)
        };
        let key = |sub_feature, leaf, purpose| {
            let (result, out) = derive(resolver, FFINetwork::Testnet, sub_feature, leaf, purpose);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            out.public_key_bytes
        };

        let auth = key(CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION, LEAF, 0);
        let auth_other_leaf = key(
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            [0x6C; 32],
            0,
        );
        let enc_unpurposed = key(CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, LEAF, 0);
        let enc = key(CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, LEAF, 1);
        let dec = key(CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, LEAF, 2);

        let all = [auth, auth_other_leaf, enc_unpurposed, enc, dec];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b, "connect keys collided (index {i})");
            }
        }

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn free_zeroizes_and_is_idempotent() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), test_resolve, noop_destroy)
        };
        let (result, mut out) = derive(
            resolver,
            FFINetwork::Mainnet,
            CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION,
            LEAF,
            2,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_ne!(out.private_key_bytes, [0u8; 32]);

        unsafe { dash_sdk_derive_connect_key_free(&mut out) };
        assert_eq!(out.private_key_bytes, [0u8; 32]);
        assert_eq!(out.public_key_bytes, [0u8; 33]);
        unsafe { dash_sdk_derive_connect_key_free(&mut out) };
        unsafe { dash_sdk_derive_connect_key_free(std::ptr::null_mut()) };

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn resolver_miss_is_a_wallet_operation_error() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), not_found_resolve, noop_destroy)
        };
        let (mut result, out) = derive(
            resolver,
            FFINetwork::Testnet,
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            LEAF,
            0,
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        let message = unsafe { CStr::from_ptr(result.message) }.to_str().unwrap();
        assert!(message.contains("no mnemonic stored"), "{message}");
        assert_eq!(out.private_key_bytes, [0u8; 32]);
        unsafe { platform_wallet_ffi_result_free(&mut result) };
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn null_resolver_is_a_null_pointer_error() {
        let mut out = ConnectDerivedKeyFFI::empty();
        let wallet_id = [0x07u8; 32];
        let result = unsafe {
            dash_sdk_derive_connect_key_with_resolver(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                std::ptr::null_mut(),
                CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
                IDENTITY.as_ptr(),
                LEAF.as_ptr(),
                0,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// The identity id is read through `read_identifier`, whose null check
    /// runs before the resolver is touched, so a dangling resolver handle
    /// is never dereferenced here.
    #[test]
    fn null_identity_id_is_an_error_before_the_resolver_is_used() {
        let mut out = ConnectDerivedKeyFFI::empty();
        let wallet_id = [0x07u8; 32];
        let result = unsafe {
            dash_sdk_derive_connect_key_with_resolver(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                std::ptr::dangling_mut(),
                CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
                std::ptr::null(),
                LEAF.as_ptr(),
                0,
                &mut out,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.private_key_bytes, [0u8; 32]);
    }

    /// Only the two DIP-13 purposes (and 0 for none) derive; anything else
    /// is refused before the seed is resolved.
    #[test]
    fn unknown_purpose_is_an_invalid_parameter() {
        let mut out = ConnectDerivedKeyFFI::empty();
        let wallet_id = [0x07u8; 32];
        let mut result = unsafe {
            dash_sdk_derive_connect_key_with_resolver(
                FFINetwork::Testnet,
                wallet_id.as_ptr(),
                std::ptr::dangling_mut(),
                CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION,
                IDENTITY.as_ptr(),
                LEAF.as_ptr(),
                3,
                &mut out,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        let message = unsafe { CStr::from_ptr(result.message) }.to_str().unwrap();
        assert!(message.contains("purpose 3"), "{message}");
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }
}
