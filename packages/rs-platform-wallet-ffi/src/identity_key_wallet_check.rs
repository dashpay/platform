//! Check identity keys against the keys a wallet derives at their slots,
//! one key or a whole identity, with that wallet's seed only.
//!
//! The FFI mirror of platform-wallet's `PlatformWallet::check_identity_key`
//! and `PlatformWallet::check_identity_keys`, driven by the resolver-backed
//! seed like [`crate::dash_sdk_derive_identity_key_at_slot_with_resolver`]:
//! the mnemonic is pulled through the Swift/Kotlin-owned `MnemonicResolver`
//! for the duration of the call only, and the wallet refuses one that is
//! not its own.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use key_wallet::bip32::ExtendedPrivKey;
use platform_wallet::wallet::identity::network::{IdentityKeyWalletCheck, IdentityWalletOwnership};
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::MnemonicResolverHandle;
use zeroize::Zeroize;

use crate::error::*;
use crate::handle::{Handle, PLATFORM_WALLET_STORAGE};
use crate::identity_keys_from_mnemonic::resolve_master_from_resolver;
use crate::runtime::block_on_worker;
use crate::types::Network;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

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
    /// The key is in an encoding the check does not compare (an
    /// ECDSA_SECP256K1 key that is not 33 bytes), so it can't say
    /// whether the key is the wallet's.
    NotCompared = 3,
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
/// the wallet behind `wallet_handle` derives at the key's slot.
/// platform-wallet's `PlatformWallet::check_identity_key` decides: it
/// reads the mnemonic through the resolver only when the key needs
/// comparing, and refuses a mnemonic that is not this wallet's
/// (`ErrorInvalidParameter`). This marshals its outcome into `out_check`.
///
/// `key_type`, `purpose` and `security_level` are DPP discriminants; an
/// unknown one is refused with `ErrorInvalidParameter` before the seed is
/// resolved.
///
/// # Safety
/// - `mnemonic_resolver_handle` must be a live handle from
///   `dash_sdk_mnemonic_resolver_create`, pinned for the call.
/// - `public_key_data` must point to `public_key_data_len` readable bytes.
/// - `out_check` must be a valid, writable pointer; release it with
///   [`platform_wallet_check_identity_key_free`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_check_identity_key_with_resolver(
    wallet_handle: Handle,
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

    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(public_key_data);

    let identity_public_key = unwrap_result_or_return!(identity_public_key_from_parts(
        key_id,
        key_type,
        purpose,
        security_level,
        std::slice::from_raw_parts(public_key_data, public_key_data_len),
        None,
    ));

    let resolver_addr = mnemonic_resolver_handle as usize;
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let resolve_master = master_resolver(resolver_addr, wallet.wallet_id());
        let wallet = wallet.clone();
        block_on_worker(async move {
            wallet
                .check_identity_key(identity_index, &identity_public_key, resolve_master)
                .await
        })
    });
    let check = match unwrap_option_or_return!(option) {
        Ok(check) => check,
        Err(failure) => return failure.into_result(),
    };

    write_check(check, &mut *out_check)
}

/// Why a wallet's key check failed, kept `Send` so it can leave the
/// worker: what the host resolver returned, or the wallet's error.
enum KeyCheckFailure {
    Resolve {
        code: PlatformWalletFFIResultCode,
        message: String,
    },
    Wallet(PlatformWalletError),
}

impl From<PlatformWalletError> for KeyCheckFailure {
    fn from(error: PlatformWalletError) -> Self {
        KeyCheckFailure::Wallet(error)
    }
}

impl KeyCheckFailure {
    fn resolve(result: PlatformWalletFFIResult) -> Self {
        let message = if result.message.is_null() {
            String::new()
        } else {
            // SAFETY: a non-null message is a NUL-terminated string the
            // result owns until it drops below.
            unsafe { CStr::from_ptr(result.message) }
                .to_string_lossy()
                .into_owned()
        };
        KeyCheckFailure::Resolve {
            code: result.code,
            message,
        }
    }

    fn into_result(self) -> PlatformWalletFFIResult {
        match self {
            KeyCheckFailure::Resolve { code, message } => {
                PlatformWalletFFIResult::err(code, message)
            }
            // Another wallet's mnemonic is a caller mapping error, as in
            // `platform_wallet_verify_seed_binds_to_wallet`.
            KeyCheckFailure::Wallet(error @ PlatformWalletError::SeedMismatch { .. }) => {
                PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    error.to_string(),
                )
            }
            KeyCheckFailure::Wallet(error) => error.into(),
        }
    }
}

/// The master-key resolver a wallet's key check calls, at most once: the
/// mnemonic the host resolver returns for `wallet_id`, as a master key
/// on the network the wallet gives. The resolver handle is carried as an
/// address so the check can run on a worker; the caller pins the handle
/// for the call.
fn master_resolver(
    resolver_addr: usize,
    wallet_id: [u8; 32],
) -> impl FnOnce(Network) -> Result<ExtendedPrivKey, KeyCheckFailure> + Send {
    move |network| {
        // SAFETY: the FFI caller pins the resolver handle for the duration
        // of the call this resolver runs inside.
        unsafe {
            resolve_master_from_resolver(
                resolver_addr as *mut MnemonicResolverHandle,
                &wallet_id,
                network,
            )
        }
        .map_err(KeyCheckFailure::resolve)
    }
}

/// The identity public key a caller describes by its fields. Fields the
/// check does not read take their defaults.
fn identity_public_key_from_parts(
    key_id: u32,
    key_type: u8,
    purpose: u8,
    security_level: u8,
    public_key_data: &[u8],
    disabled_at: Option<u64>,
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
        disabled_at,
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
        IdentityKeyWalletCheck::NotCompared => {
            out.outcome = IdentityKeyWalletCheckOutcomeFFI::NotCompared;
        }
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

/// One identity key for [`platform_wallet_check_identity_keys_with_resolver`].
/// The caller owns the data it points to.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IdentityKeyCheckInputFFI {
    pub key_id: u32,
    /// DPP key type discriminant.
    pub key_type: u8,
    /// DPP purpose discriminant.
    pub purpose: u8,
    /// DPP security level discriminant.
    pub security_level: u8,
    /// The key in its on-chain form; must not be null.
    pub public_key_data: *const u8,
    pub public_key_data_len: usize,
    /// Whether the key is disabled; `disabled_at` is read only when it is.
    pub is_disabled: bool,
    pub disabled_at: u64,
}

/// Whether an identity is this wallet's (platform-wallet's
/// `IdentityWalletOwnership`). Only `NotOwned` is a reason to offer to
/// remove the identity.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum IdentityWalletOwnershipFFI {
    /// Neither owned nor not owned; not a reason to remove the identity.
    #[default]
    Undetermined = 0,
    /// An enabled master key is the wallet's key at its slot.
    Owned = 1,
    /// Every enabled master key is foreign and no enabled key is the
    /// wallet's.
    NotOwned = 2,
}

impl From<IdentityWalletOwnership> for IdentityWalletOwnershipFFI {
    fn from(ownership: IdentityWalletOwnership) -> Self {
        match ownership {
            IdentityWalletOwnership::Owned => IdentityWalletOwnershipFFI::Owned,
            IdentityWalletOwnership::NotOwned => IdentityWalletOwnershipFFI::NotOwned,
            IdentityWalletOwnership::Undetermined => IdentityWalletOwnershipFFI::Undetermined,
        }
    }
}

/// The result of [`platform_wallet_check_identity_keys_with_resolver`].
/// Read it only after the call returns Success, and release it with
/// [`platform_wallet_check_identity_keys_free`] whatever the call
/// returned; the free wipes every private key.
#[repr(C)]
pub struct IdentityWalletCheckFFI {
    pub ownership: IdentityWalletOwnershipFFI,
    /// One result per input key, in input order; null when there are no
    /// keys.
    pub keys: *mut IdentityKeyWalletCheckFFI,
    pub keys_len: usize,
}

impl IdentityWalletCheckFFI {
    pub const fn empty() -> Self {
        Self {
            ownership: IdentityWalletOwnershipFFI::Undetermined,
            keys: std::ptr::null_mut(),
            keys_len: 0,
        }
    }
}

impl Default for IdentityWalletCheckFFI {
    fn default() -> Self {
        Self::empty()
    }
}

/// Check every key of the identity at `identity_index` against the keys
/// the wallet behind `wallet_handle` derives at their slots, and whether
/// the identity is this wallet's. platform-wallet's
/// `PlatformWallet::check_identity_keys` decides both: it reads the
/// mnemonic through the resolver only when a key needs comparing, and
/// refuses a mnemonic that is not this wallet's (`ErrorInvalidParameter`),
/// so another wallet's seed can never make an identity look foreign. This
/// marshals the result into `out_check`.
///
/// Pass every key the identity has, disabled ones included. An identity
/// with no keys is `keys` null with `keys_len` 0. A key the caller can't
/// describe is not passed but counted in `unreadable_key_count`; any
/// such key rules out `NotOwned`. Unknown key type, purpose or security
/// level discriminants are refused with `ErrorInvalidParameter` before
/// the seed is resolved.
///
/// # Safety
/// - `mnemonic_resolver_handle` must be a live handle from
///   `dash_sdk_mnemonic_resolver_create`, pinned for the call.
/// - `keys` must point to `keys_len` readable inputs (or be null with
///   `keys_len` 0), each with `public_key_data` pointing to
///   `public_key_data_len` readable bytes.
/// - `out_check` must be a valid, writable pointer; release it with
///   [`platform_wallet_check_identity_keys_free`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_check_identity_keys_with_resolver(
    wallet_handle: Handle,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    identity_index: u32,
    keys: *const IdentityKeyCheckInputFFI,
    keys_len: usize,
    unreadable_key_count: usize,
    out_check: *mut IdentityWalletCheckFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_check);
    *out_check = IdentityWalletCheckFFI::empty();

    check_ptr!(mnemonic_resolver_handle);
    let inputs: &[IdentityKeyCheckInputFFI] = if keys_len == 0 {
        &[]
    } else {
        check_ptr!(keys);
        std::slice::from_raw_parts(keys, keys_len)
    };

    let mut identity_public_keys = Vec::with_capacity(inputs.len());
    for input in inputs {
        check_ptr!(input.public_key_data);
        identity_public_keys.push(unwrap_result_or_return!(identity_public_key_from_parts(
            input.key_id,
            input.key_type,
            input.purpose,
            input.security_level,
            std::slice::from_raw_parts(input.public_key_data, input.public_key_data_len),
            input.is_disabled.then_some(input.disabled_at),
        )));
    }

    let resolver_addr = mnemonic_resolver_handle as usize;
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let resolve_master = master_resolver(resolver_addr, wallet.wallet_id());
        let wallet = wallet.clone();
        block_on_worker(async move {
            wallet
                .check_identity_keys(
                    identity_index,
                    &identity_public_keys,
                    unreadable_key_count,
                    resolve_master,
                )
                .await
        })
    });
    let check = match unwrap_option_or_return!(option) {
        Ok(check) => check,
        Err(failure) => return failure.into_result(),
    };

    let mut results: Vec<IdentityKeyWalletCheckFFI> = Vec::with_capacity(check.keys.len());
    for key_check in check.keys {
        let mut result = IdentityKeyWalletCheckFFI::empty();
        let written = write_check(key_check, &mut result);
        if written.code != PlatformWalletFFIResultCode::Success {
            for mut written_result in results {
                platform_wallet_check_identity_key_free(&mut written_result);
            }
            return written;
        }
        results.push(result);
    }
    (*out_check).ownership = check.ownership.into();
    if !results.is_empty() {
        let results: Box<[IdentityKeyWalletCheckFFI]> = results.into_boxed_slice();
        (*out_check).keys_len = results.len();
        (*out_check).keys = Box::into_raw(results).cast::<IdentityKeyWalletCheckFFI>();
    }
    PlatformWalletFFIResult::ok()
}

/// Release a result of [`platform_wallet_check_identity_keys_with_resolver`]:
/// release every key result as [`platform_wallet_check_identity_key_free`]
/// does (wiping its private key), free the array, and reset the struct
/// to empty. Safe on a null pointer and on an already-freed result.
///
/// # Safety
/// `check` must be null or point to a result the check wrote (or an
/// empty one), not copied from another.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_check_identity_keys_free(
    check: *mut IdentityWalletCheckFFI,
) {
    if check.is_null() {
        return;
    }
    let check = &mut *check;
    if !check.keys.is_null() {
        let mut keys = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            check.keys,
            check.keys_len,
        ));
        for key in keys.iter_mut() {
            platform_wallet_check_identity_key_free(key);
        }
        drop(keys);
    }
    *check = IdentityWalletCheckFFI::empty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_key_preview::IdentityKeyPreviewFFI;
    use crate::runtime::runtime;
    use crate::types::FFINetwork;
    use crate::{
        dash_sdk_derive_identity_key_at_slot_free,
        dash_sdk_derive_identity_key_at_slot_with_resolver,
    };
    use rs_sdk_ffi::{
        dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy,
        mnemonic_resolver_result,
    };
    use std::ffi::c_void;
    use std::sync::Arc;

    /// English BIP-39 test vector (all-zero entropy): the seed of the
    /// wallet `test_platform_wallet_manager` creates.
    const TEST_MNEMONIC: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    /// Another wallet's seed.
    const OTHER_MNEMONIC: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";
    const DISABLED_AT: u64 = 1_700_000_000_000;

    unsafe fn write_phrase(
        phrase: &str,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        let phrase = phrase.as_bytes();
        if phrase.len() + 1 > out_capacity {
            return mnemonic_resolver_result::BUFFER_TOO_SMALL;
        }
        std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out_buf, phrase.len());
        *out_buf.add(phrase.len()) = 0;
        *out_len = phrase.len();
        mnemonic_resolver_result::SUCCESS
    }

    unsafe extern "C" fn test_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        write_phrase(TEST_MNEMONIC, out_buf, out_capacity, out_len)
    }

    unsafe extern "C" fn other_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        out_buf: *mut c_char,
        out_capacity: usize,
        out_len: *mut usize,
    ) -> i32 {
        write_phrase(OTHER_MNEMONIC, out_buf, out_capacity, out_len)
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

    /// A resolver handle, destroyed on drop.
    struct Resolver(*mut MnemonicResolverHandle);

    impl Resolver {
        fn new(resolve: rs_sdk_ffi::MnemonicResolveCallback) -> Self {
            Resolver(unsafe {
                dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), resolve, noop_destroy)
            })
        }
    }

    impl Drop for Resolver {
        fn drop(&mut self) {
            unsafe { dash_sdk_mnemonic_resolver_destroy(self.0) };
        }
    }

    /// The testnet wallet of [`TEST_MNEMONIC`] registered as a handle,
    /// removed on drop.
    struct TestWallet {
        _manager: Arc<
            platform_wallet::PlatformWalletManager<
                platform_wallet::test_support::NoopTestPersister,
            >,
        >,
        handle: Handle,
    }

    impl TestWallet {
        fn new() -> Self {
            let (manager, wallet) = runtime().block_on(async {
                let (manager, wallet_id) =
                    platform_wallet::test_support::test_platform_wallet_manager().await;
                let wallet = manager.get_wallet(&wallet_id).await.expect("test wallet");
                (manager, wallet)
            });
            TestWallet {
                _manager: manager,
                handle: PLATFORM_WALLET_STORAGE.insert(wallet),
            }
        }
    }

    impl Drop for TestWallet {
        fn drop(&mut self) {
            PLATFORM_WALLET_STORAGE.remove(self.handle);
        }
    }

    /// The key the slot derivation gives at `(identity_index, key_index)`:
    /// `(derivation path, compressed public key, private key)`.
    fn slot_key(identity_index: u32, key_index: u32) -> (String, Vec<u8>, [u8; 32]) {
        let resolver = Resolver::new(test_resolve);
        let mut row = IdentityKeyPreviewFFI::empty();
        let result = unsafe {
            dash_sdk_derive_identity_key_at_slot_with_resolver(
                FFINetwork::Testnet,
                [0x07; 32].as_ptr(),
                resolver.0,
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

    fn uncompressed(compressed: &[u8]) -> Vec<u8> {
        dpp::secp256k1::PublicKey::from_slice(compressed)
            .expect("compressed key")
            .serialize_uncompressed()
            .to_vec()
    }

    // ── One key ───────────────────────────────────────────────────────

    fn check(
        wallet: &TestWallet,
        resolver: &Resolver,
        identity_index: u32,
        key_id: u32,
        key_type: KeyType,
        data: &[u8],
    ) -> (PlatformWalletFFIResult, IdentityKeyWalletCheckFFI) {
        let mut out = IdentityKeyWalletCheckFFI::empty();
        let result = unsafe {
            platform_wallet_check_identity_key_with_resolver(
                wallet.handle,
                resolver.0,
                identity_index,
                key_id,
                key_type as u8,
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

    /// A key the wallet does not derive needs no seed, so a missing
    /// mnemonic (or a locked Keychain) still answers NotDerived; a key it
    /// derives still needs the seed.
    #[test]
    fn should_report_not_derived_without_reading_the_mnemonic() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(not_found_resolve);

        let (result, mut out) = check(
            &wallet,
            &resolver,
            0,
            3,
            KeyType::REDPALLAS_HECATE,
            &[1; 64],
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.outcome, IdentityKeyWalletCheckOutcomeFFI::NotDerived);
        assert!(out.derivation_path.is_null());
        unsafe { platform_wallet_check_identity_key_free(&mut out) };

        let (result, mut out) = check(&wallet, &resolver, 0, 0, KeyType::ECDSA_SECP256K1, &[2; 33]);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        assert!(out.derivation_path.is_null());
        unsafe { platform_wallet_check_identity_key_free(&mut out) };
    }

    /// The wallet decides an uncompressed key is NotCompared before it
    /// reads the seed, so the one-key check needs no mnemonic for it.
    #[test]
    fn should_report_an_uncompressed_key_as_not_compared_without_reading_the_mnemonic() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(not_found_resolve);
        let (_, public_key, _) = slot_key(0, 1);

        let (result, mut out) = check(
            &wallet,
            &resolver,
            0,
            1,
            KeyType::ECDSA_SECP256K1,
            &uncompressed(&public_key),
        );

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.outcome, IdentityKeyWalletCheckOutcomeFFI::NotCompared);
        assert!(out.derivation_path.is_null());
        unsafe { platform_wallet_check_identity_key_free(&mut out) };
    }

    #[test]
    fn should_match_an_ecdsa_key_at_its_slot() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(test_resolve);
        let (path, public_key, private_key) = slot_key(1, 2);

        let (result, mut out) = check(
            &wallet,
            &resolver,
            1,
            2,
            KeyType::ECDSA_SECP256K1,
            &public_key,
        );

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
        unsafe { platform_wallet_check_identity_key_free(&mut out) };
    }

    #[test]
    fn should_report_a_mismatch_for_a_key_from_another_slot() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(test_resolve);
        let (_, other_public_key, _) = slot_key(1, 2);
        let (own_path, own_public_key, _) = slot_key(1, 3);

        let (result, mut out) = check(
            &wallet,
            &resolver,
            1,
            3,
            KeyType::ECDSA_SECP256K1,
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
    }

    /// Re-derive writes Keychain keys from this check, so another
    /// wallet's mnemonic is refused rather than compared.
    #[test]
    fn should_refuse_another_wallets_mnemonic_for_one_key() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(other_resolve);
        let (_, public_key, _) = slot_key(1, 2);

        let (result, mut out) = check(
            &wallet,
            &resolver,
            1,
            2,
            KeyType::ECDSA_SECP256K1,
            &public_key,
        );

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.outcome, IdentityKeyWalletCheckOutcomeFFI::NotDerived);
        assert!(out.derivation_path.is_null());
        assert_eq!(out.private_key_bytes, [0u8; 32]);
        unsafe { platform_wallet_check_identity_key_free(&mut out) };
    }

    #[test]
    fn should_refuse_an_unknown_key_type_purpose_or_security_level() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(test_resolve);
        let data = [2u8; 33];
        for (key_type, purpose, security_level) in
            [(u8::MAX, 0, 0), (0, u8::MAX, 0), (0, 0, u8::MAX)]
        {
            let mut out = IdentityKeyWalletCheckFFI::empty();
            let result = unsafe {
                platform_wallet_check_identity_key_with_resolver(
                    wallet.handle,
                    resolver.0,
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
    }

    #[test]
    fn should_refuse_null_pointers_and_an_unknown_wallet() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(test_resolve);
        let data = [2u8; 33];
        let call = |wallet_handle: Handle,
                    resolver: *mut MnemonicResolverHandle,
                    data: *const u8,
                    out: *mut IdentityKeyWalletCheckFFI| unsafe {
            platform_wallet_check_identity_key_with_resolver(
                wallet_handle,
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
            call(wallet.handle, std::ptr::null_mut(), data.as_ptr(), &mut out),
            call(wallet.handle, resolver.0, std::ptr::null(), &mut out),
            call(
                wallet.handle,
                resolver.0,
                data.as_ptr(),
                std::ptr::null_mut(),
            ),
        ] {
            assert_eq!(code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
        assert_eq!(
            call(Handle::MAX, resolver.0, data.as_ptr(), &mut out),
            PlatformWalletFFIResultCode::NotFound
        );
    }

    #[test]
    fn should_free_a_null_check() {
        unsafe { platform_wallet_check_identity_key_free(std::ptr::null_mut()) };
    }

    // ── A whole identity ──────────────────────────────────────────────

    fn input(
        key_id: u32,
        key_type: KeyType,
        security_level: SecurityLevel,
        data: &[u8],
        disabled_at: Option<u64>,
    ) -> IdentityKeyCheckInputFFI {
        IdentityKeyCheckInputFFI {
            key_id,
            key_type: key_type as u8,
            purpose: Purpose::AUTHENTICATION as u8,
            security_level: security_level as u8,
            public_key_data: data.as_ptr(),
            public_key_data_len: data.len(),
            is_disabled: disabled_at.is_some(),
            disabled_at: disabled_at.unwrap_or(0),
        }
    }

    fn check_identity(
        wallet: &TestWallet,
        resolver: &Resolver,
        identity_index: u32,
        inputs: &[IdentityKeyCheckInputFFI],
        unreadable_key_count: usize,
    ) -> (PlatformWalletFFIResult, IdentityWalletCheckFFI) {
        let mut out = IdentityWalletCheckFFI::empty();
        let result = unsafe {
            platform_wallet_check_identity_keys_with_resolver(
                wallet.handle,
                resolver.0,
                identity_index,
                inputs.as_ptr(),
                inputs.len(),
                unreadable_key_count,
                &mut out,
            )
        };
        (result, out)
    }

    fn outcomes(check: &IdentityWalletCheckFFI) -> Vec<IdentityKeyWalletCheckOutcomeFFI> {
        if check.keys.is_null() {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(check.keys, check.keys_len) }
            .iter()
            .map(|key| key.outcome)
            .collect()
    }

    /// The public-entry-point regression: the wallet's own identity,
    /// checked with another wallet's mnemonic, is refused rather than
    /// reported NotOwned; with its own mnemonic it is Owned.
    #[test]
    fn should_refuse_another_wallets_mnemonic_instead_of_calling_the_identity_foreign() {
        let wallet = TestWallet::new();
        let (_, master_key, _) = slot_key(1, 0);
        let inputs = [input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &master_key,
            None,
        )];

        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(other_resolve), 1, &inputs, 0);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Undetermined);
        assert!(out.keys.is_null());
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };

        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(test_resolve), 1, &inputs, 0);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Owned);
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
    }

    /// A master rotation leaves the former, foreign master disabled; the
    /// enabled master is the wallet's, so the identity is.
    #[test]
    fn should_own_an_identity_whose_enabled_master_replaced_a_foreign_one() {
        let wallet = TestWallet::new();
        let (_, former_master, _) = slot_key(1, 9);
        let (_, current_master, _) = slot_key(1, 1);
        let inputs = [
            input(
                0,
                KeyType::ECDSA_SECP256K1,
                SecurityLevel::MASTER,
                &former_master,
                Some(DISABLED_AT),
            ),
            input(
                1,
                KeyType::ECDSA_SECP256K1,
                SecurityLevel::MASTER,
                &current_master,
                None,
            ),
        ];

        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(test_resolve), 1, &inputs, 0);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Owned);
        assert_eq!(
            outcomes(&out),
            [
                IdentityKeyWalletCheckOutcomeFFI::Mismatch,
                IdentityKeyWalletCheckOutcomeFFI::Matches
            ]
        );
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
    }

    #[test]
    fn should_own_an_identity_whose_master_matches_beside_a_foreign_key() {
        let wallet = TestWallet::new();
        let (_, master_key, _) = slot_key(1, 0);
        let (_, foreign_key, _) = slot_key(1, 7);
        let inputs = [
            input(
                0,
                KeyType::ECDSA_SECP256K1,
                SecurityLevel::MASTER,
                &master_key,
                None,
            ),
            input(
                1,
                KeyType::ECDSA_SECP256K1,
                SecurityLevel::HIGH,
                &foreign_key,
                None,
            ),
        ];

        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(test_resolve), 1, &inputs, 0);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Owned);
        assert_eq!(
            outcomes(&out),
            [
                IdentityKeyWalletCheckOutcomeFFI::Matches,
                IdentityKeyWalletCheckOutcomeFFI::Mismatch
            ]
        );
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
    }

    /// A foreign master is NotOwned, but not while a key the caller could
    /// not describe could be the wallet's.
    #[test]
    fn should_not_own_an_identity_whose_only_enabled_master_is_foreign() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(test_resolve);
        let (_, foreign_master, _) = slot_key(1, 5);
        let inputs = [input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &foreign_master,
            None,
        )];

        for (unreadable_key_count, ownership) in [
            (0, IdentityWalletOwnershipFFI::NotOwned),
            (1, IdentityWalletOwnershipFFI::Undetermined),
        ] {
            let (result, mut out) =
                check_identity(&wallet, &resolver, 1, &inputs, unreadable_key_count);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(
                out.ownership, ownership,
                "{unreadable_key_count} unreadable"
            );
            assert_eq!(outcomes(&out), [IdentityKeyWalletCheckOutcomeFFI::Mismatch]);
            unsafe { platform_wallet_check_identity_keys_free(&mut out) };
        }
    }

    /// The wallet's own master encoded uncompressed is not compared, so
    /// it can't make the identity look foreign, and it needs no seed.
    #[test]
    fn should_not_compare_an_uncompressed_master() {
        let wallet = TestWallet::new();
        let (_, master_key, _) = slot_key(1, 0);
        let uncompressed_master = uncompressed(&master_key);
        let inputs = [input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &uncompressed_master,
            None,
        )];

        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(not_found_resolve), 1, &inputs, 0);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Undetermined);
        assert_eq!(
            outcomes(&out),
            [IdentityKeyWalletCheckOutcomeFFI::NotCompared]
        );
        let key = unsafe { &*out.keys };
        assert!(key.derivation_path.is_null());
        assert!(key.derived_public_key_data.is_null());
        assert_eq!(key.private_key_bytes, [0u8; 32]);
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
    }

    /// An identity whose keys the wallet derives none of needs no seed;
    /// one with a derived key still does.
    #[test]
    fn should_check_an_identity_without_derived_keys_without_reading_the_mnemonic() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(not_found_resolve);
        let hecate_data = [1u8; 64];
        let mut hecate = input(
            3,
            KeyType::REDPALLAS_HECATE,
            SecurityLevel::MEDIUM,
            &hecate_data,
            None,
        );
        hecate.purpose = Purpose::HECATE_SIGNER as u8;

        let (result, mut out) = check_identity(&wallet, &resolver, 0, &[hecate], 0);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Undetermined);
        assert_eq!(
            outcomes(&out),
            [IdentityKeyWalletCheckOutcomeFFI::NotDerived]
        );
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };

        let ecdsa_data = [2u8; 33];
        let ecdsa = input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &ecdsa_data,
            None,
        );
        let (result, mut out) = check_identity(&wallet, &resolver, 0, &[hecate, ecdsa], 0);
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        assert!(out.keys.is_null());
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
    }

    #[test]
    fn should_refuse_an_identity_key_with_an_unknown_discriminant() {
        let wallet = TestWallet::new();
        let data = [2u8; 33];
        let mut unknown = input(
            1,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::HIGH,
            &data,
            None,
        );
        unknown.key_type = u8::MAX;
        let inputs = [
            input(
                0,
                KeyType::ECDSA_SECP256K1,
                SecurityLevel::MASTER,
                &data,
                None,
            ),
            unknown,
        ];

        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(test_resolve), 0, &inputs, 0);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert!(out.keys.is_null());
        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
    }

    #[test]
    fn should_refuse_null_identity_check_pointers_and_an_unknown_wallet() {
        let wallet = TestWallet::new();
        let resolver = Resolver::new(test_resolve);
        let data = [2u8; 33];
        let valid = [input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &data,
            None,
        )];
        let mut no_data = input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &data,
            None,
        );
        no_data.public_key_data = std::ptr::null();
        let call = |wallet_handle: Handle,
                    resolver: *mut MnemonicResolverHandle,
                    keys: *const IdentityKeyCheckInputFFI,
                    keys_len: usize,
                    out: *mut IdentityWalletCheckFFI| unsafe {
            platform_wallet_check_identity_keys_with_resolver(
                wallet_handle,
                resolver,
                0,
                keys,
                keys_len,
                0,
                out,
            )
            .code
        };
        let mut out = IdentityWalletCheckFFI::empty();
        for code in [
            call(
                wallet.handle,
                std::ptr::null_mut(),
                valid.as_ptr(),
                1,
                &mut out,
            ),
            call(wallet.handle, resolver.0, std::ptr::null(), 1, &mut out),
            call(wallet.handle, resolver.0, &no_data, 1, &mut out),
            call(
                wallet.handle,
                resolver.0,
                valid.as_ptr(),
                1,
                std::ptr::null_mut(),
            ),
        ] {
            assert_eq!(code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
        assert_eq!(
            call(Handle::MAX, resolver.0, valid.as_ptr(), 1, &mut out),
            PlatformWalletFFIResultCode::NotFound
        );

        // No keys is a null array with length 0, and says nothing about
        // whose identity it is.
        assert_eq!(
            call(wallet.handle, resolver.0, std::ptr::null(), 0, &mut out),
            PlatformWalletFFIResultCode::Success
        );
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Undetermined);
        assert!(out.keys.is_null());
    }

    #[test]
    fn should_free_an_identity_check_twice_and_on_null() {
        let wallet = TestWallet::new();
        let (_, master_key, _) = slot_key(0, 0);
        let inputs = [input(
            0,
            KeyType::ECDSA_SECP256K1,
            SecurityLevel::MASTER,
            &master_key,
            None,
        )];
        let (result, mut out) =
            check_identity(&wallet, &Resolver::new(test_resolve), 0, &inputs, 0);
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!out.keys.is_null());

        unsafe { platform_wallet_check_identity_keys_free(&mut out) };
        assert!(out.keys.is_null());
        assert_eq!(out.keys_len, 0);
        assert_eq!(out.ownership, IdentityWalletOwnershipFFI::Undetermined);
        unsafe {
            platform_wallet_check_identity_keys_free(&mut out);
            platform_wallet_check_identity_keys_free(std::ptr::null_mut());
        }
    }
}
