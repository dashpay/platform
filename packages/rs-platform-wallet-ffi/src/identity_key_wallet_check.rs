//! Check an identity key against the key the wallet derives at its slot.
//!
//! The FFI mirror of
//! [`platform_wallet::wallet::identity::network::check_identity_key_from_master`],
//! driven by the resolver-backed seed like
//! [`crate::dash_sdk_derive_identity_key_at_slot_with_resolver`]: the
//! mnemonic is pulled through the Swift/Kotlin-owned `MnemonicResolver`
//! for the duration of the call only.

use std::ffi::CString;
use std::os::raw::c_char;

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use platform_wallet::wallet::identity::network::{
    check_identity_key_from_master, IdentityKeyWalletCheck,
};
use rs_sdk_ffi::MnemonicResolverHandle;
use zeroize::Zeroize;

use crate::error::*;
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::types::{FFINetwork, Network};
use crate::{check_ptr, unwrap_result_or_return};

/// The outcome of [`platform_wallet_check_identity_key_with_resolver`]
/// (platform-wallet's `IdentityKeyWalletCheck`).
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum IdentityKeyWalletCheckOutcomeFFI {
    /// The wallet does not derive keys of this type at identity key
    /// slots, so there is nothing to compare the key with.
    #[default]
    NotDerived = 0,
    /// The wallet's key at the slot is this key.
    Matches = 1,
    /// The wallet derives a different key at the slot.
    Mismatch = 2,
}

/// The result of [`platform_wallet_check_identity_key_with_resolver`].
/// Read it only after the call returns Success, and release it with
/// [`platform_wallet_check_identity_key_free`] whatever the call
/// returned; the free wipes the private key.
#[repr(C)]
pub struct IdentityKeyWalletCheckFFI {
    pub outcome: IdentityKeyWalletCheckOutcomeFFI,
    /// The slot's DIP-9 path as a NUL-terminated string; null for
    /// `NotDerived`.
    pub derivation_path: *mut c_char,
    /// `Mismatch` only: the wallet's key at the slot in the key type's
    /// on-chain form. Null with a zero length otherwise.
    pub derived_public_key_data: *mut u8,
    pub derived_public_key_data_len: usize,
    /// `Matches` only: the key's 32-byte private key; zero otherwise.
    pub private_key_bytes: [u8; 32],
    /// `Matches` only: the key's public key hash; zero otherwise.
    pub public_key_hash: [u8; 20],
}

impl IdentityKeyWalletCheckFFI {
    pub const fn empty() -> Self {
        Self {
            outcome: IdentityKeyWalletCheckOutcomeFFI::NotDerived,
            derivation_path: std::ptr::null_mut(),
            derived_public_key_data: std::ptr::null_mut(),
            derived_public_key_data_len: 0,
            private_key_bytes: [0u8; 32],
            public_key_hash: [0u8; 20],
        }
    }
}

impl Default for IdentityKeyWalletCheckFFI {
    fn default() -> Self {
        Self::empty()
    }
}

/// Check the identity key `(key_id, key_type, purpose, security_level,
/// public_key_data)` of the identity at `identity_index` against the key
/// the wallet derives at the key's slot, from the mnemonic the resolver
/// returns for `wallet_id_bytes`. platform-wallet's
/// `check_identity_key_from_master` decides; this marshals its outcome
/// into `out_check`.
///
/// `key_type`, `purpose` and `security_level` are DPP discriminants; an
/// unknown one is refused with `ErrorInvalidParameter` before the seed is
/// resolved. `network` picks the coin type (`5'` mainnet, `1'`
/// otherwise).
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must be a live handle from
///   `dash_sdk_mnemonic_resolver_create`.
/// - `public_key_data` must point to `public_key_data_len` readable bytes.
/// - `out_check` must be a valid, writable pointer; release it with
///   [`platform_wallet_check_identity_key_free`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_check_identity_key_with_resolver(
    network: FFINetwork,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_index: u32,
    key_id: u32,
    key_type: u8,
    purpose: u8,
    security_level: u8,
    public_key_data: *const u8,
    public_key_data_len: usize,
    out_check: *mut IdentityKeyWalletCheckFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_check);
    *out_check = IdentityKeyWalletCheckFFI::empty();

    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(public_key_data);

    let identity_public_key = unwrap_result_or_return!(identity_public_key_from_parts(
        key_id,
        key_type,
        purpose,
        security_level,
        std::slice::from_raw_parts(public_key_data, public_key_data_len),
    ));

    let kw_network: Network = network.into();
    let wallet_id: [u8; 32] = std::ptr::read(wallet_id_bytes.cast::<[u8; 32]>());
    let mut master = unwrap_result_or_return!(resolve_master_from_resolver(
        mnemonic_resolver_handle,
        &wallet_id,
        kw_network,
    ));

    let check =
        check_identity_key_from_master(&master, kw_network, identity_index, &identity_public_key);
    // Erase the master now so its scalar does not outlive the check.
    master.private_key.zeroize();
    let check = unwrap_result_or_return!(check);

    write_check(check, &mut *out_check)
}

/// The identity public key a caller describes by its fields. Fields the
/// check does not read take their defaults.
fn identity_public_key_from_parts(
    key_id: u32,
    key_type: u8,
    purpose: u8,
    security_level: u8,
    public_key_data: &[u8],
) -> Result<IdentityPublicKey, PlatformWalletFFIResult> {
    let invalid = |field: &str, value: u8| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("unknown {field} {value}"),
        )
    };
    let key_type = KeyType::try_from(key_type).map_err(|_| invalid("key type", key_type))?;
    let purpose = Purpose::try_from(purpose).map_err(|_| invalid("purpose", purpose))?;
    let security_level = SecurityLevel::try_from(security_level)
        .map_err(|_| invalid("security level", security_level))?;
    Ok(IdentityPublicKeyV0 {
        id: key_id,
        purpose,
        security_level,
        contract_bounds: None,
        key_type,
        read_only: false,
        data: BinaryData::new(public_key_data.to_vec()),
        disabled_at: None,
    }
    .into())
}

/// Marshal `check` into `out` (already reset to empty).
fn write_check(
    check: IdentityKeyWalletCheck,
    out: &mut IdentityKeyWalletCheckFFI,
) -> PlatformWalletFFIResult {
    match check {
        IdentityKeyWalletCheck::NotDerived => {}
        IdentityKeyWalletCheck::Matches {
            derivation_path,
            private_key,
            public_key_hash,
        } => {
            let path = unwrap_result_or_return!(CString::new(derivation_path.to_string()));
            out.outcome = IdentityKeyWalletCheckOutcomeFFI::Matches;
            out.derivation_path = path.into_raw();
            out.private_key_bytes = *private_key;
            out.public_key_hash = public_key_hash;
        }
        IdentityKeyWalletCheck::Mismatch {
            derivation_path,
            derived_public_key_data,
        } => {
            let path = unwrap_result_or_return!(CString::new(derivation_path.to_string()));
            out.outcome = IdentityKeyWalletCheckOutcomeFFI::Mismatch;
            out.derivation_path = path.into_raw();
            if !derived_public_key_data.is_empty() {
                let data: Box<[u8]> = derived_public_key_data.into_boxed_slice();
                out.derived_public_key_data_len = data.len();
                out.derived_public_key_data = Box::into_raw(data).cast::<u8>();
            }
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Release a result of [`platform_wallet_check_identity_key_with_resolver`]:
/// wipe the private key (a volatile write the optimizer cannot elide),
/// free the path and the derived key data, and reset the struct to
/// empty. Safe on a null pointer and on an already-freed result.
///
/// # Safety
/// `check` must be null or point to a result the check wrote (or an
/// empty one), not copied from another.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_check_identity_key_free(
    check: *mut IdentityKeyWalletCheckFFI,
) {
    if check.is_null() {
        return;
    }
    let check = &mut *check;
    if !check.derivation_path.is_null() {
        drop(CString::from_raw(check.derivation_path));
    }
    if !check.derived_public_key_data.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            check.derived_public_key_data,
            check.derived_public_key_data_len,
        )));
    }
    check.private_key_bytes.zeroize();
    *check = IdentityKeyWalletCheckFFI::empty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_key_preview::IdentityKeyPreviewFFI;
    use crate::{
        dash_sdk_derive_identity_key_at_slot_free,
        dash_sdk_derive_identity_key_at_slot_with_resolver,
    };
    use rs_sdk_ffi::{
        dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy,
        mnemonic_resolver_result,
    };
    use std::ffi::{c_void, CStr};

    /// English BIP-39 test vector (all-zero entropy); the fixture the
    /// platform-wallet derivation tests use.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const WALLET_ID: [u8; 32] = [0x07; 32];

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

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    /// A resolver over [`TEST_MNEMONIC`]; destroy it with
    /// `dash_sdk_mnemonic_resolver_destroy`.
    fn test_resolver() -> *mut MnemonicResolverHandle {
        unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), test_resolve, noop_destroy)
        }
    }

    /// The key the slot derivation gives at `(identity_index, key_index)`:
    /// `(derivation path, compressed public key, private key)`.
    fn slot_key(
        resolver: *mut MnemonicResolverHandle,
        identity_index: u32,
        key_index: u32,
    ) -> (String, Vec<u8>, [u8; 32]) {
        let mut row = IdentityKeyPreviewFFI::empty();
        let result = unsafe {
            dash_sdk_derive_identity_key_at_slot_with_resolver(
                FFINetwork::Testnet,
                WALLET_ID.as_ptr(),
                resolver,
                identity_index,
                key_index,
                &mut row,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        let slot = unsafe {
            (
                CStr::from_ptr(row.derivation_path)
                    .to_str()
                    .expect("utf-8 path")
                    .to_owned(),
                std::slice::from_raw_parts(row.public_key, row.public_key_len).to_vec(),
                row.private_key_bytes,
            )
        };
        unsafe { dash_sdk_derive_identity_key_at_slot_free(&mut row) };
        slot
    }

    fn check(
        resolver: *mut MnemonicResolverHandle,
        identity_index: u32,
        key_id: u32,
        key_type: u8,
        data: &[u8],
    ) -> (PlatformWalletFFIResult, IdentityKeyWalletCheckFFI) {
        let mut out = IdentityKeyWalletCheckFFI::empty();
        let result = unsafe {
            platform_wallet_check_identity_key_with_resolver(
                FFINetwork::Testnet,
                WALLET_ID.as_ptr(),
                resolver,
                identity_index,
                key_id,
                key_type,
                Purpose::AUTHENTICATION as u8,
                SecurityLevel::HIGH as u8,
                data.as_ptr(),
                data.len(),
                &mut out,
            )
        };
        (result, out)
    }

    fn path_of(check: &IdentityKeyWalletCheckFFI) -> String {
        assert!(!check.derivation_path.is_null());
        unsafe { CStr::from_ptr(check.derivation_path) }
            .to_str()
            .expect("utf-8 path")
            .to_owned()
    }

    #[test]
    fn should_report_a_hecate_signer_key_as_not_derived() {
        let resolver = test_resolver();
        let (result, mut out) = check(resolver, 0, 3, KeyType::REDPALLAS_HECATE as u8, &[1; 64]);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.outcome, IdentityKeyWalletCheckOutcomeFFI::NotDerived);
        assert!(out.derivation_path.is_null());
        assert!(out.derived_public_key_data.is_null());
        assert_eq!(out.private_key_bytes, [0u8; 32]);

        unsafe {
            platform_wallet_check_identity_key_free(&mut out);
            dash_sdk_mnemonic_resolver_destroy(resolver);
        }
    }

    #[test]
    fn should_match_an_ecdsa_key_at_its_slot() {
        let resolver = test_resolver();
        let (path, public_key, private_key) = slot_key(resolver, 1, 2);

        let (result, mut out) = check(resolver, 1, 2, KeyType::ECDSA_SECP256K1 as u8, &public_key);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.outcome, IdentityKeyWalletCheckOutcomeFFI::Matches);
        assert_eq!(path_of(&out), path);
        assert_eq!(out.private_key_bytes, private_key);
        assert_eq!(
            out.public_key_hash.as_slice(),
            dpp::util::hash::ripemd160_sha256(&public_key).as_slice()
        );
        assert!(out.derived_public_key_data.is_null());

        unsafe { platform_wallet_check_identity_key_free(&mut out) };
        assert_eq!(
            out.private_key_bytes, [0u8; 32],
            "free wipes the private key"
        );
        assert!(out.derivation_path.is_null());
        // A second free is a no-op.
        unsafe {
            platform_wallet_check_identity_key_free(&mut out);
            dash_sdk_mnemonic_resolver_destroy(resolver);
        }
    }

    #[test]
    fn should_report_a_mismatch_for_a_key_from_another_slot() {
        let resolver = test_resolver();
        let (_, other_public_key, _) = slot_key(resolver, 1, 2);
        let (own_path, own_public_key, _) = slot_key(resolver, 1, 3);

        let (result, mut out) = check(
            resolver,
            1,
            3,
            KeyType::ECDSA_SECP256K1 as u8,
            &other_public_key,
        );

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.outcome, IdentityKeyWalletCheckOutcomeFFI::Mismatch);
        assert_eq!(path_of(&out), own_path);
        assert_eq!(
            unsafe {
                std::slice::from_raw_parts(
                    out.derived_public_key_data,
                    out.derived_public_key_data_len,
                )
            },
            own_public_key.as_slice()
        );
        assert_eq!(out.private_key_bytes, [0u8; 32]);
        assert_eq!(out.public_key_hash, [0u8; 20]);

        unsafe { platform_wallet_check_identity_key_free(&mut out) };
        assert!(out.derived_public_key_data.is_null());
        assert_eq!(out.derived_public_key_data_len, 0);
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn should_refuse_an_unknown_key_type_purpose_or_security_level() {
        let resolver = test_resolver();
        let data = [2u8; 33];
        for (key_type, purpose, security_level) in
            [(u8::MAX, 0, 0), (0, u8::MAX, 0), (0, 0, u8::MAX)]
        {
            let mut out = IdentityKeyWalletCheckFFI::empty();
            let result = unsafe {
                platform_wallet_check_identity_key_with_resolver(
                    FFINetwork::Testnet,
                    WALLET_ID.as_ptr(),
                    resolver,
                    0,
                    0,
                    key_type,
                    purpose,
                    security_level,
                    data.as_ptr(),
                    data.len(),
                    &mut out,
                )
            };
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "({key_type}, {purpose}, {security_level})"
            );
            unsafe { platform_wallet_check_identity_key_free(&mut out) };
        }
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn should_refuse_null_pointers() {
        let resolver = test_resolver();
        let data = [2u8; 33];
        let call = |wallet_id: *const u8,
                    resolver: *mut MnemonicResolverHandle,
                    data: *const u8,
                    out: *mut IdentityKeyWalletCheckFFI| unsafe {
            platform_wallet_check_identity_key_with_resolver(
                FFINetwork::Testnet,
                wallet_id,
                resolver,
                0,
                0,
                KeyType::ECDSA_SECP256K1 as u8,
                0,
                0,
                data,
                33,
                out,
            )
            .code
        };
        let mut out = IdentityKeyWalletCheckFFI::empty();
        for code in [
            call(std::ptr::null(), resolver, data.as_ptr(), &mut out),
            call(
                WALLET_ID.as_ptr(),
                std::ptr::null_mut(),
                data.as_ptr(),
                &mut out,
            ),
            call(WALLET_ID.as_ptr(), resolver, std::ptr::null(), &mut out),
            call(
                WALLET_ID.as_ptr(),
                resolver,
                data.as_ptr(),
                std::ptr::null_mut(),
            ),
        ] {
            assert_eq!(code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn should_free_a_null_check() {
        unsafe { platform_wallet_check_identity_key_free(std::ptr::null_mut()) };
    }
}
