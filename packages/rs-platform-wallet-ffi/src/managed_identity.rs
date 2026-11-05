use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{check_ptr, deref_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dash_sdk::Sdk;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use platform_wallet::{fetch_identity_public_keys, ManagedIdentity};
use rs_sdk_ffi::{DashSDKError, DashSDKErrorCode, FFIError};
use std::collections::BTreeMap;
use std::os::raw::c_char;
use std::os::raw::c_void;

/// Create a new ManagedIdentity from a DPP Identity serialized bytes
#[no_mangle]
pub unsafe extern "C" fn managed_identity_create_from_identity_bytes(
    identity_bytes: *const std::os::raw::c_uchar,
    identity_len: usize,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_bytes);
    check_ptr!(out_handle);

    let bytes = unsafe { std::slice::from_raw_parts(identity_bytes, identity_len) };

    let identity = unwrap_result_or_return!(
        dpp::identity::Identity::deserialize_from_bytes_untrusted_no_limit(bytes)
    );

    let managed_identity = ManagedIdentity::new(identity, 0);
    let handle = MANAGED_IDENTITY_STORAGE.insert(managed_identity);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::ok()
}

/// Get the identity ID into a 32-byte out-buffer.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_id(
    identity_handle: Handle,
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_id);

    let option =
        MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| identity.identity.id());
    let id = unwrap_option_or_return!(option);
    unsafe { write_identifier(out_id, &id) };
    PlatformWalletFFIResult::ok()
}

/// Get the identity balance
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_balance(
    identity_handle: Handle,
    out_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_balance);

    let option =
        MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| identity.identity.balance());
    *out_balance = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get the label.
///
/// `ManagedIdentity` no longer carries a `label` field — labels live
/// on Swift `PersistentIdentity.alias` instead. Stub returning null.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_label(
    identity_handle: Handle,
    out_label: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_label);
    // Null the out-pointer before the fallible handle lookup so even the
    // invalid-handle error path leaves it well-defined.
    unsafe { *out_label = std::ptr::null_mut() };

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |_identity| ());
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Set the label — no-op stub.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_set_label(
    identity_handle: Handle,
    _label: *const c_char,
) -> PlatformWalletFFIResult {
    let option = MANAGED_IDENTITY_STORAGE.with_item_mut(identity_handle, |_identity| ());
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get last updated balance block time
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_last_updated_balance_block_time(
    identity_handle: Handle,
    out_block_time: *mut BlockTime,
) -> PlatformWalletFFIResult {
    check_ptr!(out_block_time);

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity.last_updated_balance_block_time
    });
    let bt = unwrap_option_or_return!(option);
    unsafe {
        *out_block_time = match bt {
            Some(bt) => bt.into(),
            None => BlockTime {
                height: 0,
                core_height: 0,
                timestamp: 0,
            },
        };
    }
    PlatformWalletFFIResult::ok()
}

/// Set the block time on this detached managed-identity handle only.
///
/// Wallet lookups return snapshot clones. This setter neither updates the live
/// wallet's verified balance watermark nor persists metadata. Obtain a fresh
/// snapshot after `platform_wallet_refresh_identity_balance` to observe the
/// authoritative wallet state; this function is not a live-watermark reset API.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_set_last_updated_balance_block_time(
    identity_handle: Handle,
    block_time: *const BlockTime,
) -> PlatformWalletFFIResult {
    let bt = deref_ptr!(block_time);
    let owned: platform_wallet::BlockTime = platform_wallet::BlockTime {
        height: bt.height,
        core_height: bt.core_height,
        timestamp: bt.timestamp,
    };
    let option = MANAGED_IDENTITY_STORAGE.with_item_mut(identity_handle, |identity| {
        identity.last_updated_balance_block_time = Some(owned);
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get last synced keys block time
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_last_synced_keys_block_time(
    identity_handle: Handle,
    out_block_time: *mut BlockTime,
) -> PlatformWalletFFIResult {
    check_ptr!(out_block_time);

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity.last_synced_keys_block_time
    });
    let bt = unwrap_option_or_return!(option);
    unsafe {
        *out_block_time = match bt {
            Some(bt) => bt.into(),
            None => BlockTime {
                height: 0,
                core_height: 0,
                timestamp: 0,
            },
        };
    }
    PlatformWalletFFIResult::ok()
}

/// Get the identity revision.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_revision(
    identity_handle: Handle,
    out_revision: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_revision);

    let option = MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| identity.identity.revision());
    *out_revision = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Flat C representation of an `IdentityPublicKey` for the Swift
/// wrapper.
#[repr(C)]
pub struct IdentityPublicKeyFFI {
    pub key_id: u32,
    pub purpose: u8,
    pub security_level: u8,
    pub key_type: u8,
    pub read_only: bool,
    pub disabled_at_is_some: bool,
    pub disabled_at: u64,
    pub data_ptr: *mut u8,
    pub data_len: usize,
    /// Usage limits (protocol version 14): the credits the key may spend
    /// over its lifetime when `total_budget_is_some`, and the block time in
    /// milliseconds from which it can no longer sign when
    /// `expires_at_is_some`. A version 0 key has neither.
    pub total_budget_is_some: bool,
    pub total_budget: u64,
    pub expires_at_is_some: bool,
    pub expires_at: u64,
}

/// One key read by [`platform_wallet_fetch_identity_public_keys`]: the key,
/// and whether it had expired at the block time of the proof it was read
/// with. Arrays of it are freed with
/// [`platform_wallet_free_fetched_identity_public_keys`].
#[repr(C)]
pub struct FetchedIdentityPublicKeyFFI {
    pub key: IdentityPublicKeyFFI,
    /// Decided in platform-wallet by dpp's rule (`is_expired_at`, the expiry
    /// instant included): `true` means the key can no longer sign; `false`
    /// means it had not expired at the proof's time, and it may have since.
    pub expired_at_proof_time: bool,
}

/// Snapshot every `IdentityPublicKey` on the identity into a flat
/// heap-allocated array. Free with [`managed_identity_free_public_keys`].
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_public_keys(
    identity_handle: Handle,
    out_keys: *mut *mut IdentityPublicKeyFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_keys);
    check_ptr!(out_count);
    // Sentinel first: the handle lookup below is fallible, and
    // `managed_identity_free_public_keys` reconstructs the array (and each
    // entry's owned `data_ptr`) from any non-null pointer / non-zero count
    // pair — a cleanup-on-error caller must never see stack garbage here.
    unsafe {
        *out_keys = std::ptr::null_mut();
        *out_count = 0;
    }

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        public_keys_to_ffi(identity.identity.public_keys())
    });
    let buf = unwrap_option_or_return!(option);
    unsafe { write_array_out(buf, out_keys, out_count) };
    PlatformWalletFFIResult::ok()
}

/// Flatten an identity's key map into owned [`IdentityPublicKeyFFI`]
/// entries, every key included (disabled ones too). Hand the array out
/// through [`write_array_out`] so [`managed_identity_free_public_keys`]
/// can release it.
fn public_keys_to_ffi(keys: &BTreeMap<KeyID, IdentityPublicKey>) -> Vec<IdentityPublicKeyFFI> {
    keys.iter()
        .map(|(&key_id, key)| public_key_to_ffi(key_id, key))
        .collect()
}

/// One key as an [`IdentityPublicKeyFFI`]. Its `data_ptr` owns a heap copy
/// of the key bytes, released by [`free_public_key_data`].
fn public_key_to_ffi(key_id: KeyID, pk: &IdentityPublicKey) -> IdentityPublicKeyFFI {
    let data_bytes = pk.data().as_slice().to_vec();
    let data_len = data_bytes.len();
    let data_ptr = Box::into_raw(data_bytes.into_boxed_slice()) as *mut u8;

    let (disabled_at_is_some, disabled_at) = match pk.disabled_at() {
        Some(ts) => (true, ts),
        None => (false, 0u64),
    };
    let (total_budget_is_some, total_budget) = match pk.total_budget() {
        Some(credits) => (true, credits),
        None => (false, 0u64),
    };
    let (expires_at_is_some, expires_at) = match pk.expires_at() {
        Some(ts) => (true, ts),
        None => (false, 0u64),
    };

    IdentityPublicKeyFFI {
        key_id,
        purpose: pk.purpose() as u8,
        security_level: pk.security_level() as u8,
        key_type: pk.key_type() as u8,
        read_only: pk.read_only(),
        disabled_at_is_some,
        disabled_at,
        data_ptr,
        data_len,
        total_budget_is_some,
        total_budget,
        expires_at_is_some,
        expires_at,
    }
}

/// Move `buf` into the `(out_ptr, out_count)` pair as a heap array, which
/// the free function of its element type releases. An empty `buf` writes
/// the `(null, 0)` sentinel.
///
/// # Safety
/// `out_ptr` and `out_count` must be valid for writes.
unsafe fn write_array_out<T>(buf: Vec<T>, out_ptr: *mut *mut T, out_count: *mut usize) {
    if buf.is_empty() {
        unsafe {
            *out_ptr = std::ptr::null_mut();
            *out_count = 0;
        }
        return;
    }
    let count = buf.len();
    let array_ptr = Box::into_raw(buf.into_boxed_slice()) as *mut T;
    unsafe {
        *out_ptr = array_ptr;
        *out_count = count;
    }
}

/// Read an identity's public keys from Platform, without loading or
/// touching any managed identity.
///
/// Marshals [`platform_wallet::fetch_identity_public_keys`], which runs the
/// proved keys-only query (every key, disabled ones included) and decides
/// that an identity Platform has no keys for does not exist. Each key comes
/// back as a [`FetchedIdentityPublicKeyFFI`]: the [`IdentityPublicKeyFFI`]
/// [`managed_identity_get_public_keys`] also returns, with its expiry at the
/// proof's time; contract bounds are not part of it. On success
/// `*out_found` says whether Platform has the identity; when it does not,
/// the key out-params hold `(null, 0)`. Either way `*out_block_time_ms` is
/// the Platform block time, in milliseconds, of the proof the answer was
/// verified against; the proof covers it (see
/// [`platform_wallet::FetchedIdentityPublicKeys::block_time_ms`]).
///
/// That time is a lower bound on Platform's current time: the SDK accepts a
/// proof up to its time tolerance old, and any proof on a builder without
/// one, so a caller that needs a fresh answer must build the SDK with a time
/// tolerance. Each entry's `expired_at_proof_time` is decided in
/// platform-wallet by dpp's rule at that time: `true` means the key can no
/// longer sign; `false` means it had not expired then, and it may have since.
///
/// `dash_sdk_identity_fetch_public_keys` in rs-sdk-ffi runs the same query
/// and answers JSON; this one hands back typed entries for callers that
/// need them typed.
///
/// Free a non-null array with
/// [`platform_wallet_free_fetched_identity_public_keys`].
///
/// This lives in platform-wallet-ffi rather than rs-sdk-ffi because it
/// owns the key types, and rs-sdk-ffi cannot depend on this
/// crate. It reports errors the way rs-sdk-ffi queries do: the return is
/// null on success, otherwise a `DashSDKError` classified like theirs
/// (network, timeout, ...), which the caller frees with
/// `dash_sdk_error_free`. A null argument is an `InvalidParameter` error.
///
/// `sdk_ptr` points at a live `dash_sdk::Sdk`, as handed out by
/// `dash_sdk_get_inner_sdk_ptr` in rs-sdk-ffi; it is taken as
/// `*const c_void` for the same reason as in `platform_wallet_manager_create`.
/// The caller keeps that `Sdk` alive for the duration of the call.
///
/// # Safety
/// - `sdk_ptr` must point at a valid `dash_sdk::Sdk`.
/// - `identity_id` must point at 32 readable bytes.
/// - `out_found`, `out_block_time_ms`, `out_keys` and `out_count` must each
///   be null or valid for writes. Every non-null one is reset (`false`, `0`,
///   `(null, 0)`) before anything else is checked.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fetch_identity_public_keys(
    sdk_ptr: *const c_void,
    identity_id: *const u8,
    out_found: *mut bool,
    out_block_time_ms: *mut u64,
    out_keys: *mut *mut FetchedIdentityPublicKeyFFI,
    out_count: *mut usize,
) -> *mut DashSDKError {
    // Sentinel first, each out-param on its own, so a caller that frees on
    // every return never sees stack garbage, whichever argument is null.
    unsafe {
        if !out_found.is_null() {
            *out_found = false;
        }
        if !out_block_time_ms.is_null() {
            *out_block_time_ms = 0;
        }
        if !out_keys.is_null() {
            *out_keys = std::ptr::null_mut();
        }
        if !out_count.is_null() {
            *out_count = 0;
        }
    }
    for (is_null, name) in [
        (sdk_ptr.is_null(), "sdk_ptr"),
        (identity_id.is_null(), "identity_id"),
        (out_found.is_null(), "out_found"),
        (out_block_time_ms.is_null(), "out_block_time_ms"),
        (out_keys.is_null(), "out_keys"),
        (out_count.is_null(), "out_count"),
    ] {
        if is_null {
            return Box::into_raw(Box::new(DashSDKError::new(
                DashSDKErrorCode::InvalidParameter,
                format!("{name} is null"),
            )));
        }
    }

    let mut id_bytes = [0u8; 32];
    id_bytes.copy_from_slice(unsafe { std::slice::from_raw_parts(identity_id, 32) });
    let id = Identifier::from(id_bytes);
    let sdk = unsafe { (*(sdk_ptr as *const Sdk)).clone() };

    let fetched = block_on_worker(async move { fetch_identity_public_keys(&sdk, id).await });
    let fetched = match fetched {
        Ok(fetched) => fetched,
        Err(e) => return Box::into_raw(Box::new(DashSDKError::from(FFIError::from(e)))),
    };
    unsafe {
        *out_block_time_ms = fetched.block_time_ms;
        if let Some(keys) = &fetched.keys {
            let buf: Vec<FetchedIdentityPublicKeyFFI> = keys
                .iter()
                .map(|(&key_id, key)| FetchedIdentityPublicKeyFFI {
                    key: public_key_to_ffi(key_id, key),
                    expired_at_proof_time: fetched.is_expired_at_proof_time(key),
                })
                .collect();
            write_array_out(buf, out_keys, out_count);
            *out_found = true;
        }
    }
    std::ptr::null_mut()
}

/// Release an array previously returned by
/// [`managed_identity_get_public_keys`].
#[no_mangle]
pub unsafe extern "C" fn managed_identity_free_public_keys(
    keys: *mut IdentityPublicKeyFFI,
    count: usize,
) {
    if keys.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(keys, count) };
    for entry in slice.iter_mut() {
        unsafe { free_public_key_data(entry) };
    }
    let _ = unsafe { Box::from_raw(slice as *mut [IdentityPublicKeyFFI]) };
}

/// Release an array previously returned by
/// [`platform_wallet_fetch_identity_public_keys`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_free_fetched_identity_public_keys(
    keys: *mut FetchedIdentityPublicKeyFFI,
    count: usize,
) {
    if keys.is_null() || count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(keys, count) };
    for entry in slice.iter_mut() {
        unsafe { free_public_key_data(&mut entry.key) };
    }
    let _ = unsafe { Box::from_raw(slice as *mut [FetchedIdentityPublicKeyFFI]) };
}

/// Release the key bytes [`public_key_to_ffi`] copied, and clear the pointer.
///
/// # Safety
/// `entry.data_ptr` is null or owns `entry.data_len` bytes from
/// [`public_key_to_ffi`].
unsafe fn free_public_key_data(entry: &mut IdentityPublicKeyFFI) {
    if !entry.data_ptr.is_null() && entry.data_len > 0 {
        let data_slice = unsafe { std::slice::from_raw_parts_mut(entry.data_ptr, entry.data_len) };
        let _ = unsafe { Box::from_raw(data_slice as *mut [u8]) };
        entry.data_ptr = std::ptr::null_mut();
        entry.data_len = 0;
    }
}

/// Destroy ManagedIdentity and free resources
#[no_mangle]
pub unsafe extern "C" fn managed_identity_destroy(
    identity_handle: Handle,
) -> PlatformWalletFFIResult {
    if MANAGED_IDENTITY_STORAGE.remove(identity_handle).is_some() {
        PlatformWalletFFIResult::ok()
    } else {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Invalid identity handle",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::runtime;
    use dash_sdk::query_types::IdentityPublicKeys;
    use dash_sdk::SdkBuilder;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::identity_public_key::v1::IdentityPublicKeyV1;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use rs_sdk_ffi::dash_sdk_error_free;
    use std::collections::BTreeMap;
    use std::ptr::{null, null_mut, NonNull};
    use std::slice;

    fn create_test_identity() -> Identity {
        let id = Identifier::from([1u8; 32]);
        let mut public_keys = BTreeMap::new();

        public_keys.insert(
            0,
            IdentityPublicKey::V0(
                dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
                    id: 0,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::MASTER,
                    read_only: false,
                    data: dpp::platform_value::BinaryData::new(vec![2u8; 33]),
                    disabled_at: None,
                    contract_bounds: None,
                },
            ),
        );

        let identity_v0 = IdentityV0 {
            id,
            public_keys,
            balance: 1000,
            revision: 1,
        };
        Identity::V0(identity_v0)
    }

    #[test]
    fn test_get_and_set_label_stub_returns_null() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let label = std::ffi::CString::new("Test Identity").unwrap();
            let _ = managed_identity_set_label(handle, label.as_ptr());

            let mut label_ptr: *mut c_char = std::ptr::null_mut();
            let _ = managed_identity_get_label(handle, &mut label_ptr);
            assert!(label_ptr.is_null());

            managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_balance() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut balance: u64 = 0;
            let _ = managed_identity_get_balance(handle, &mut balance);

            managed_identity_destroy(handle);
        }
    }

    #[test]
    fn should_keep_manual_balance_block_time_changes_on_detached_handles() {
        use crate::identity_manager::{
            identity_manager_add_identity, identity_manager_create, identity_manager_destroy,
            identity_manager_get_identity,
        };

        unsafe {
            let original =
                MANAGED_IDENTITY_STORAGE.insert(ManagedIdentity::new(create_test_identity(), 0));
            let mut manager = NULL_HANDLE;
            assert_eq!(
                identity_manager_create(&mut manager).code,
                PlatformWalletFFIResultCode::Success
            );
            assert_eq!(
                identity_manager_add_identity(manager, original).code,
                PlatformWalletFFIResultCode::Success
            );
            let mut snapshot = NULL_HANDLE;
            assert_eq!(
                identity_manager_get_identity(manager, [1u8; 32].as_ptr(), &mut snapshot).code,
                PlatformWalletFFIResultCode::Success
            );
            let stamp = BlockTime {
                height: u64::MAX,
                core_height: u32::MAX,
                timestamp: u64::MAX,
            };
            assert_eq!(
                managed_identity_set_last_updated_balance_block_time(snapshot, &stamp).code,
                PlatformWalletFFIResultCode::Success
            );
            assert_eq!(
                MANAGED_IDENTITY_STORAGE.with_item(snapshot, |identity| identity
                    .last_updated_balance_block_time
                    .unwrap()
                    .height),
                Some(u64::MAX)
            );
            assert_eq!(
                IDENTITY_MANAGER_STORAGE.with_item(manager, |manager| manager
                    .managed_identity(&Identifier::from([1; 32]))
                    .unwrap()
                    .last_updated_balance_block_time),
                Some(None)
            );

            // Even adding the edited snapshot to another standalone manager
            // imports only its DPP identity, not its manually assigned metadata.
            let mut other_manager = NULL_HANDLE;
            assert_eq!(
                identity_manager_create(&mut other_manager).code,
                PlatformWalletFFIResultCode::Success
            );
            assert_eq!(
                identity_manager_add_identity(other_manager, snapshot).code,
                PlatformWalletFFIResultCode::Success
            );
            assert_eq!(
                IDENTITY_MANAGER_STORAGE.with_item(other_manager, |manager| manager
                    .managed_identity(&Identifier::from([1; 32]))
                    .unwrap()
                    .last_updated_balance_block_time),
                Some(None)
            );
            managed_identity_destroy(snapshot);
            managed_identity_destroy(original);
            identity_manager_destroy(manager);
            identity_manager_destroy(other_manager);
        }
    }

    #[test]
    fn test_block_time_operations() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let block_time = BlockTime {
                height: 100,
                core_height: 200,
                timestamp: 1234567890,
            };

            let _ = managed_identity_set_last_updated_balance_block_time(handle, &block_time);

            let mut retrieved_bt = BlockTime {
                height: 0,
                core_height: 0,
                timestamp: 0,
            };
            let _ = managed_identity_get_last_updated_balance_block_time(handle, &mut retrieved_bt);
            assert_eq!(retrieved_bt.height, 100);
            assert_eq!(retrieved_bt.core_height, 200);
            assert_eq!(retrieved_bt.timestamp, 1234567890);

            managed_identity_destroy(handle);
        }
    }

    fn key_v0(id: KeyID, disabled_at: Option<u64>) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            read_only: true,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at,
            contract_bounds: None,
        })
    }

    fn key_v1(id: KeyID, total_budget: Option<u64>, expires_at: Option<u64>) -> IdentityPublicKey {
        let IdentityPublicKey::V0(v0) = key_v0(id, None) else {
            unreachable!()
        };
        IdentityPublicKey::V1(IdentityPublicKeyV1::from_v0_with_limits(
            v0,
            total_budget,
            expires_at,
        ))
    }

    #[test]
    fn should_flatten_every_key_with_its_usage_limits() {
        let mut keys = BTreeMap::new();
        keys.insert(0, key_v0(0, None));
        keys.insert(1, key_v0(1, Some(1_700_000_000_000)));
        keys.insert(2, key_v1(2, Some(5_000_000), Some(1_800_000_000_000)));
        keys.insert(3, key_v1(3, None, None));

        let buf = public_keys_to_ffi(&keys);
        assert_eq!(buf.len(), 4);

        let ids: Vec<u32> = buf.iter().map(|k| k.key_id).collect();
        assert_eq!(ids, vec![0, 1, 2, 3]);
        for entry in &buf {
            assert_eq!(entry.purpose, Purpose::AUTHENTICATION as u8);
            assert_eq!(entry.security_level, SecurityLevel::HIGH as u8);
            assert_eq!(entry.key_type, KeyType::ECDSA_SECP256K1 as u8);
            assert!(entry.read_only);
            assert_eq!(entry.data_len, 33);
            let data = unsafe { slice::from_raw_parts(entry.data_ptr, entry.data_len) };
            assert!(data.iter().all(|&b| b == entry.key_id as u8));
        }

        assert!(!buf[0].disabled_at_is_some);
        assert!(buf[1].disabled_at_is_some);
        assert_eq!(buf[1].disabled_at, 1_700_000_000_000);

        assert!(!buf[0].total_budget_is_some && !buf[0].expires_at_is_some);
        assert!(buf[2].total_budget_is_some);
        assert_eq!(buf[2].total_budget, 5_000_000);
        assert!(buf[2].expires_at_is_some);
        assert_eq!(buf[2].expires_at, 1_800_000_000_000);
        assert!(!buf[3].total_budget_is_some && !buf[3].expires_at_is_some);

        let mut out_keys: *mut IdentityPublicKeyFFI = null_mut();
        let mut out_count: usize = 0;
        unsafe {
            write_array_out(buf, &mut out_keys, &mut out_count);
            assert!(!out_keys.is_null());
            assert_eq!(out_count, 4);
            managed_identity_free_public_keys(out_keys, out_count);
        }
    }

    #[test]
    fn should_write_the_empty_sentinel_for_no_keys() {
        let buf = public_keys_to_ffi(&BTreeMap::new());
        assert!(buf.is_empty());

        let mut out_keys: *mut IdentityPublicKeyFFI = NonNull::dangling().as_ptr();
        let mut out_count: usize = 7;
        unsafe { write_array_out(buf, &mut out_keys, &mut out_count) };
        assert!(out_keys.is_null());
        assert_eq!(out_count, 0);
    }

    /// Owns a fetch's outputs and frees whatever it returned.
    struct FetchOutcome {
        error: *mut DashSDKError,
        found: bool,
        block_time_ms: u64,
        keys: *mut FetchedIdentityPublicKeyFFI,
        count: usize,
    }

    impl Drop for FetchOutcome {
        fn drop(&mut self) {
            unsafe {
                platform_wallet_free_fetched_identity_public_keys(self.keys, self.count);
                dash_sdk_error_free(self.error);
            }
        }
    }

    fn fetch_keys(sdk: &Sdk, id: [u8; 32]) -> FetchOutcome {
        // Garbage in every out-param, so the test sees what the call wrote.
        let mut outcome = FetchOutcome {
            error: null_mut(),
            found: true,
            block_time_ms: 7,
            keys: NonNull::dangling().as_ptr(),
            count: 7,
        };
        outcome.error = unsafe {
            platform_wallet_fetch_identity_public_keys(
                sdk as *const Sdk as *const c_void,
                id.as_ptr(),
                &mut outcome.found,
                &mut outcome.block_time_ms,
                &mut outcome.keys,
                &mut outcome.count,
            )
        };
        outcome
    }

    const BLOCK_TIME_MS: u64 = 1_750_000_000_000;

    fn mock_sdk_answering(id: [u8; 32], keys: Option<IdentityPublicKeys>) -> Sdk {
        let mut sdk = SdkBuilder::new_mock().build().expect("mock sdk");
        runtime().block_on(async {
            sdk.mock()
                .set_response_time_ms(BLOCK_TIME_MS)
                .expect_fetch_many::<KeyID, IdentityPublicKey, _, IdentityPublicKeys>(
                    Identifier::from(id),
                    keys,
                )
                .await
                .expect("identity keys expectation");
        });
        sdk
    }

    #[test]
    fn should_hand_out_the_keys_of_a_found_identity_with_the_block_time_and_expiry() {
        let id = [4u8; 32];
        let keys: IdentityPublicKeys = [
            (0, Some(key_v0(0, None))),
            (1, Some(key_v1(1, None, Some(BLOCK_TIME_MS)))),
            (2, Some(key_v1(2, None, Some(BLOCK_TIME_MS + 1)))),
        ]
        .into_iter()
        .collect();
        let sdk = mock_sdk_answering(id, Some(keys));

        let outcome = fetch_keys(&sdk, id);

        assert!(outcome.error.is_null());
        assert!(outcome.found);
        assert_eq!(outcome.block_time_ms, BLOCK_TIME_MS);
        assert_eq!(outcome.count, 3);
        let keys = unsafe { slice::from_raw_parts(outcome.keys, outcome.count) };
        assert_eq!(
            keys.iter().map(|k| k.key.key_id).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        // Expired at the proof's time: only the key whose expiry is that time.
        assert_eq!(
            keys.iter()
                .map(|k| k.expired_at_proof_time)
                .collect::<Vec<_>>(),
            vec![false, true, false]
        );
    }

    #[test]
    fn should_hand_out_not_found_as_the_empty_sentinel_with_the_block_time() {
        let id = [5u8; 32];
        let sdk = mock_sdk_answering(id, None);

        let outcome = fetch_keys(&sdk, id);

        assert!(outcome.error.is_null());
        assert!(!outcome.found);
        assert_eq!(outcome.block_time_ms, BLOCK_TIME_MS);
        assert!(outcome.keys.is_null());
        assert_eq!(outcome.count, 0);
    }

    #[test]
    fn should_hand_out_a_failed_query_as_an_error() {
        // No expectation: the mock transport fails the request.
        let sdk = SdkBuilder::new_mock().build().expect("mock sdk");

        let outcome = fetch_keys(&sdk, [6u8; 32]);

        assert!(!outcome.error.is_null());
        assert_ne!(unsafe { (*outcome.error).code }, DashSDKErrorCode::Success);
        assert!(!outcome.found);
        assert_eq!(outcome.block_time_ms, 0);
        assert!(outcome.keys.is_null());
        assert_eq!(outcome.count, 0);
    }

    #[test]
    fn should_reject_null_arguments_and_still_reset_the_outputs() {
        let sdk = SdkBuilder::new_mock().build().expect("mock sdk");
        let sdk_ptr = &sdk as *const Sdk as *const c_void;
        let id = [1u8; 32];

        /// `ptr`, or null when `null` is set.
        fn or_null<T>(null: bool, ptr: *mut T) -> *mut T {
            if null {
                null_mut()
            } else {
                ptr
            }
        }

        for null_arg in 0..6 {
            let mut found = true;
            let mut block_time_ms: u64 = 7;
            let mut out_keys: *mut FetchedIdentityPublicKeyFFI = NonNull::dangling().as_ptr();
            let mut out_count: usize = 7;

            let error = unsafe {
                platform_wallet_fetch_identity_public_keys(
                    if null_arg == 0 { null() } else { sdk_ptr },
                    if null_arg == 1 { null() } else { id.as_ptr() },
                    or_null(null_arg == 2, &mut found),
                    or_null(null_arg == 3, &mut block_time_ms),
                    or_null(null_arg == 4, &mut out_keys),
                    or_null(null_arg == 5, &mut out_count),
                )
            };

            assert!(!error.is_null(), "null argument {null_arg} was accepted");
            assert_eq!(unsafe { (*error).code }, DashSDKErrorCode::InvalidParameter);
            unsafe { dash_sdk_error_free(error) };
            // Every out-param that was passed is reset, whichever one is null.
            assert!(
                null_arg == 2 || !found,
                "found not reset, argument {null_arg} null"
            );
            assert!(
                null_arg == 3 || block_time_ms == 0,
                "time not reset, argument {null_arg} null"
            );
            assert!(
                null_arg == 4 || out_keys.is_null(),
                "keys not reset, argument {null_arg} null"
            );
            assert!(
                null_arg == 5 || out_count == 0,
                "count not reset, argument {null_arg} null"
            );
        }
    }
}
