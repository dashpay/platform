//! DashPay Connect key derivation at the DIP-13 sub-feature paths
//! (dashpay/dips#191), driven by the resolver-backed seed.
//!
//! Both DashPay Connect v2 keys live under new DIP-13 sub-features with
//! DIP-14 256-bit hardened leaves, so no wallet-local counter is an input
//! and two devices restored from one seed derive the same key:
//!
//! ```text
//! session auth:   m / 9' / coin' / 5' / 6' / 0' / identityId' / requestId'
//! app encryption: m / 9' / coin' / 5' / 7' / 0' / identityId' / contractId' / purpose' / rotation'
//! ```
//!
//! [`dash_sdk_derive_legacy_app_encryption_key_with_resolver`] derives the
//! app encryption key at the pre-rotation layout (no `rotation'` level), so
//! wallets can recognise keys registered on testnet/devnet before DIP-13
//! gained that level.
//!
//! The wallet derives every key; the app never sees the seed. This module
//! is the FFI mirror of
//! [`platform_wallet::wallet::identity::network::derive_connect_keypair_from_master`]
//! and follows the resolver contract of
//! [`crate::dash_sdk_derive_identity_key_at_slot_with_resolver`]: the
//! mnemonic is pulled through the Swift/Kotlin-owned `MnemonicResolver`
//! for the duration of the call only, in a zeroized buffer.

use key_wallet::bip32::ApplicationKeyPurpose;
use key_wallet::dip9::{
    FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_ENCRYPTION,
    FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_SESSION_AUTHENTICATION,
};
use platform_wallet::wallet::identity::network::{derive_connect_keypair_from_master, ConnectKey};

use crate::error::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::types::{read_identifier, FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};
use rs_sdk_ffi::MnemonicResolverHandle;
use zeroize::Zeroize;

/// `sub_feature` for the DashPay Connect session authentication key. The
/// leaf is the connect request id (`hash256` of the app's ephemeral public
/// key); `purpose` and `rotation` are `0`.
pub const CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION: u32 = 6;

/// `sub_feature` for the DashPay Connect app encryption key pair. The leaf
/// is the id of the data contract the key is bound to; `purpose` is the
/// DPP purpose discriminant, `1` ENCRYPTION or `2` DECRYPTION, and
/// `rotation` the trailing `rotation'` level.
pub const CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION: u32 = 7;

/// `purpose` for the ENCRYPTION half of an app encryption pair (the DPP
/// `Purpose::ENCRYPTION` discriminant).
pub const CONNECT_KEY_PURPOSE_ENCRYPTION: u32 = 1;

/// `purpose` for the DECRYPTION half of an app encryption pair (the DPP
/// `Purpose::DECRYPTION` discriminant).
pub const CONNECT_KEY_PURPOSE_DECRYPTION: u32 = 2;

// cbindgen only exports literal constants; these pin the literals above to
// key-wallet's.
const _: () = assert!(
    CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION
        == FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_SESSION_AUTHENTICATION
);
const _: () = assert!(
    CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION
        == FEATURE_PURPOSE_IDENTITIES_SUBFEATURE_APPLICATION_ENCRYPTION
);
const _: () = assert!(CONNECT_KEY_PURPOSE_ENCRYPTION == ApplicationKeyPurpose::Encryption as u32);
const _: () = assert!(CONNECT_KEY_PURPOSE_DECRYPTION == ApplicationKeyPurpose::Decryption as u32);

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

/// Derive a DashPay Connect key from the mnemonic the resolver returns for
/// `wallet_id_bytes`, at the DIP-13 application paths key-wallet builds.
///
/// - `sub_feature` `6` with `purpose` `0` and `rotation` `0`: the session
///   authentication key; `leaf_bytes` is the connect request id. The path
///   has no purpose or rotation level.
/// - `sub_feature` `7` with `purpose` `1` (ENCRYPTION) or `2` (DECRYPTION):
///   one half of the app encryption pair; `leaf_bytes` is the bound contract
///   id and `rotation` is the trailing `rotation'` level (`0` for the first
///   pair, at most `2^31 - 1`).
///
/// Anything else is refused with `ErrorInvalidParameter`. `network` picks
/// the coin type (`5'` mainnet, `1'` otherwise). Keys at the pre-rotation
/// app encryption layout come from
/// [`dash_sdk_derive_legacy_app_encryption_key_with_resolver`] instead.
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
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn dash_sdk_derive_connect_key_with_resolver(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    sub_feature: u32,
    identity_id_bytes: *const u8,
    leaf_bytes: *const u8,
    purpose: u32,
    rotation: u32,
    out_key: *mut ConnectDerivedKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_key);
    *out_key = ConnectDerivedKeyFFI::empty();

    check_ptr!(leaf_bytes);
    let identity_id = unwrap_result_or_return!(read_identifier(identity_id_bytes));
    let leaf = read_leaf(leaf_bytes);
    let key = match (sub_feature, purpose, rotation) {
        (CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION, 0, 0) => {
            ConnectKey::SessionAuthentication {
                identity_id,
                request_id: leaf,
            }
        }
        (CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, purpose, rotation) => ConnectKey::AppEncryption {
            identity_id,
            contract_id: leaf,
            purpose: unwrap_result_or_return!(app_key_purpose(purpose)),
            rotation,
        },
        (sub_feature, purpose, rotation) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "sub_feature {sub_feature} with purpose {purpose} and rotation {rotation} is \
                     not a DashPay Connect key (6 with purpose 0 and rotation 0, or 7 with \
                     purpose 1 or 2)"
                ),
            );
        }
    };

    derive_into(
        network,
        wallet_id_bytes,
        mnemonic_resolver_handle,
        key,
        out_key,
    )
}

/// Derive one half of a DashPay Connect app encryption pair at the
/// pre-rotation DIP-13 layout
/// `m/9'/coin'/5'/7'/0'/identityId'/contractId'/purpose'`, which has no
/// trailing `rotation'` level. Only for recognising keys registered on
/// testnet/devnet before DIP-13 gained that level; derive new keys with
/// [`dash_sdk_derive_connect_key_with_resolver`].
///
/// `purpose` is `1` (ENCRYPTION) or `2` (DECRYPTION); anything else is
/// refused with `ErrorInvalidParameter`. `contract_id_bytes` is the bound
/// contract id. `network` picks the coin type (`5'` mainnet, `1'`
/// otherwise).
///
/// The derived key is written to `out_key`; call
/// [`dash_sdk_derive_connect_key_free`] when done with it.
///
/// # Safety
/// - `wallet_id_bytes`, `identity_id_bytes` and `contract_id_bytes` must
///   point to 32 readable bytes each.
/// - `mnemonic_resolver_handle` must be a live handle from
///   `dash_sdk_mnemonic_resolver_create`.
/// - `out_key` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn dash_sdk_derive_legacy_app_encryption_key_with_resolver(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_id_bytes: *const u8,
    contract_id_bytes: *const u8,
    purpose: u32,
    out_key: *mut ConnectDerivedKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_key);
    *out_key = ConnectDerivedKeyFFI::empty();

    check_ptr!(contract_id_bytes);
    let identity_id = unwrap_result_or_return!(read_identifier(identity_id_bytes));
    let key = ConnectKey::AppEncryptionLegacy {
        identity_id,
        contract_id: read_leaf(contract_id_bytes),
        purpose: unwrap_result_or_return!(app_key_purpose(purpose)),
    };

    derive_into(
        network,
        wallet_id_bytes,
        mnemonic_resolver_handle,
        key,
        out_key,
    )
}

/// Copy the 32-byte leaf (request or contract id) behind a checked pointer.
unsafe fn read_leaf(leaf_bytes: *const u8) -> [u8; 32] {
    std::slice::from_raw_parts(leaf_bytes, 32)
        .try_into()
        .expect("from_raw_parts(_, 32) always yields exactly 32 bytes")
}

/// The app encryption half a C `purpose` names.
fn app_key_purpose(purpose: u32) -> Result<ApplicationKeyPurpose, PlatformWalletFFIResult> {
    match purpose {
        CONNECT_KEY_PURPOSE_ENCRYPTION => Ok(ApplicationKeyPurpose::Encryption),
        CONNECT_KEY_PURPOSE_DECRYPTION => Ok(ApplicationKeyPurpose::Decryption),
        purpose => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "purpose {purpose} is not an app encryption purpose (1 ENCRYPTION or 2 DECRYPTION)"
            ),
        )),
    }
}

/// Resolve the wallet's master key, derive `key` and write it to `out_key`
/// (already reset to empty by the caller).
unsafe fn derive_into(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    key: ConnectKey,
    out_key: *mut ConnectDerivedKeyFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);

    let kw_network: Network = network.into();
    // Refuse an unrepresentable key (a rotation of 2^31 or more) before
    // the resolver is asked for the seed.
    unwrap_result_or_return!(key.derivation_path(kw_network));
    let wallet_id: [u8; 32] = std::slice::from_raw_parts(wallet_id_bytes, 32)
        .try_into()
        .expect("from_raw_parts(_, 32) always yields exactly 32 bytes");
    let mut master = unwrap_result_or_return!(resolve_master_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
        kw_network,
    ));

    let derived = derive_connect_keypair_from_master(&master, kw_network, key);
    // Dropping `master` zeroizes it too; erase now so the scalar does not
    // outlive the derive.
    master.private_key.zeroize();
    let derived = unwrap_result_or_return!(derived);

    (*out_key).private_key_bytes = *derived.private_key;
    (*out_key).public_key_bytes = derived.public_key;

    PlatformWalletFFIResult::ok()
}

/// Wipe a key populated by [`dash_sdk_derive_connect_key_with_resolver`] or
/// [`dash_sdk_derive_legacy_app_encryption_key_with_resolver`].
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
    use key_wallet::bip32::ExtendedPrivKey;
    use key_wallet::mnemonic::Mnemonic;
    use rs_sdk_ffi::mnemonic_resolver_result;
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};
    use std::ffi::c_void;
    use std::ffi::CStr;
    use std::os::raw::c_char;

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
        rotation: u32,
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
                rotation,
                &mut out,
            )
        };
        (result, out)
    }

    fn derive_legacy(
        resolver: *mut MnemonicResolverHandle,
        network: FFINetwork,
        contract_id: [u8; 32],
        purpose: u32,
    ) -> (PlatformWalletFFIResult, ConnectDerivedKeyFFI) {
        let wallet_id = [0x07u8; 32];
        let mut out = ConnectDerivedKeyFFI::empty();
        let result = unsafe {
            dash_sdk_derive_legacy_app_encryption_key_with_resolver(
                network,
                wallet_id.as_ptr(),
                resolver,
                IDENTITY.as_ptr(),
                contract_id.as_ptr(),
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
            0,
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let (result, second) = derive(
            resolver,
            FFINetwork::Testnet,
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            LEAF,
            0,
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
            ConnectKey::SessionAuthentication {
                identity_id: Identifier::from(IDENTITY),
                request_id: LEAF,
            },
        )
        .unwrap();
        assert_eq!(first.private_key_bytes, *expected.private_key);
        assert_eq!(first.public_key_bytes, expected.public_key);

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// Both app encryption entry points derive the DIP-13 pinned vectors
    /// (mainnet, identity 0x35.., contract 0x6b..): the rotated layout at
    /// rotations 0 and 1, and the pre-rotation layout.
    #[test]
    fn app_encryption_entry_points_derive_the_pinned_vectors() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), test_resolve, noop_destroy)
        };
        let check = |(result, out): (PlatformWalletFFIResult, ConnectDerivedKeyFFI), expected| {
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(hex::encode(out.public_key_bytes), expected);
        };

        for (rotation, expected) in [
            (
                0,
                "031a882c119e2b1f8e0349b444d877afa2599efeef49a26930fd25518cb6e9d57a",
            ),
            (
                1,
                "03d0f6f0f48d968b9554d8e7fe467d7135bacf29d5c4994ca593bd10ebeed696f6",
            ),
        ] {
            check(
                derive(
                    resolver,
                    FFINetwork::Mainnet,
                    CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION,
                    LEAF,
                    CONNECT_KEY_PURPOSE_ENCRYPTION,
                    rotation,
                ),
                expected,
            );
        }
        check(
            derive_legacy(
                resolver,
                FFINetwork::Mainnet,
                LEAF,
                CONNECT_KEY_PURPOSE_ENCRYPTION,
            ),
            "03e989de1b62f137231cc06659c810c17100becd1f5e23d5710faa7602769fe434",
        );

        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    /// Leaf, purpose and sub-feature each change the key.
    #[test]
    fn different_leaves_purposes_and_sub_features_give_different_keys() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), test_resolve, noop_destroy)
        };
        let key = |sub_feature, leaf, purpose, rotation| {
            let (result, out) = derive(
                resolver,
                FFINetwork::Testnet,
                sub_feature,
                leaf,
                purpose,
                rotation,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            out.public_key_bytes
        };

        let auth = key(CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION, LEAF, 0, 0);
        let auth_other_leaf = key(
            CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
            [0x6C; 32],
            0,
            0,
        );
        let enc = key(CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, LEAF, 1, 0);
        let dec = key(CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, LEAF, 2, 0);
        let enc_next_rotation = key(CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, LEAF, 1, 1);
        let (result, legacy) = derive_legacy(resolver, FFINetwork::Testnet, LEAF, 1);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);

        let all = [
            auth,
            auth_other_leaf,
            enc,
            dec,
            enc_next_rotation,
            legacy.public_key_bytes,
        ];
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
            0,
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
                0,
                &mut out,
            )
        };
        assert_ne!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.private_key_bytes, [0u8; 32]);
    }

    /// Only session authentication with no purpose or rotation, and app
    /// encryption with ENCRYPTION or DECRYPTION at a 31-bit rotation, derive;
    /// anything else is refused before the seed is resolved (the dangling
    /// resolver handle is never touched).
    #[test]
    fn other_sub_feature_purpose_and_rotation_combinations_are_invalid_parameters() {
        for (sub_feature, purpose, rotation, expected) in [
            (CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, 3, 0, "purpose 3"),
            (CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION, 0, 0, "purpose 0"),
            (
                CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION,
                1,
                0x8000_0000,
                "rotation 2147483648",
            ),
            (
                CONNECT_KEY_SUB_FEATURE_APP_ENCRYPTION,
                2,
                u32::MAX,
                "rotation 4294967295",
            ),
            (
                CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
                1,
                0,
                "sub_feature 6 with purpose 1 and rotation 0",
            ),
            (
                CONNECT_KEY_SUB_FEATURE_SESSION_AUTHENTICATION,
                0,
                1,
                "sub_feature 6 with purpose 0 and rotation 1",
            ),
            (5, 0, 0, "sub_feature 5 with purpose 0 and rotation 0"),
        ] {
            let mut out = ConnectDerivedKeyFFI::empty();
            let wallet_id = [0x07u8; 32];
            let mut result = unsafe {
                dash_sdk_derive_connect_key_with_resolver(
                    FFINetwork::Testnet,
                    wallet_id.as_ptr(),
                    std::ptr::dangling_mut(),
                    sub_feature,
                    IDENTITY.as_ptr(),
                    LEAF.as_ptr(),
                    purpose,
                    rotation,
                    &mut out,
                )
            };
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "sub_feature {sub_feature} purpose {purpose} rotation {rotation}"
            );
            let message = unsafe { CStr::from_ptr(result.message) }.to_str().unwrap();
            assert!(message.contains(expected), "{message}");
            unsafe { platform_wallet_ffi_result_free(&mut result) };
        }
    }

    /// The legacy entry point takes only ENCRYPTION or DECRYPTION, refused
    /// before the seed is resolved.
    #[test]
    fn legacy_app_encryption_refuses_other_purposes() {
        for purpose in [0, 3] {
            let (mut result, out) =
                derive_legacy(std::ptr::dangling_mut(), FFINetwork::Testnet, LEAF, purpose);
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            );
            let message = unsafe { CStr::from_ptr(result.message) }.to_str().unwrap();
            assert!(message.contains(&format!("purpose {purpose}")), "{message}");
            assert_eq!(out.private_key_bytes, [0u8; 32]);
            unsafe { platform_wallet_ffi_result_free(&mut result) };
        }
    }
}
