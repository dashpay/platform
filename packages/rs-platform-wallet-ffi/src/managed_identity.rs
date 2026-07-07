use crate::error::*;
use crate::handle::*;
use crate::types::*;
use crate::{check_ptr, deref_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::serialization::PlatformDeserializable;
use platform_wallet::ManagedIdentity;
use std::os::raw::c_char;

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
        dpp::identity::Identity::deserialize_from_bytes_no_limit(bytes)
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

/// Set last updated balance block time.
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
        let keys = identity.identity.public_keys();
        let mut buf: Vec<IdentityPublicKeyFFI> = Vec::with_capacity(keys.len());
        for (&key_id, pk) in keys {
            let data_bytes = pk.data().as_slice().to_vec();
            let data_len = data_bytes.len();
            let data_boxed = data_bytes.into_boxed_slice();
            let data_ptr = Box::into_raw(data_boxed) as *mut u8;

            let (disabled_some, disabled_val) = match pk.disabled_at() {
                Some(ts) => (true, ts),
                None => (false, 0u64),
            };

            buf.push(IdentityPublicKeyFFI {
                key_id,
                purpose: pk.purpose() as u8,
                security_level: pk.security_level() as u8,
                key_type: pk.key_type() as u8,
                read_only: pk.read_only(),
                disabled_at_is_some: disabled_some,
                disabled_at: disabled_val,
                data_ptr,
                data_len,
            });
        }
        buf
    });
    let buf = unwrap_option_or_return!(option);

    if buf.is_empty() {
        // Out-params already hold the (null, 0) sentinel written above.
        return PlatformWalletFFIResult::ok();
    }
    let count = buf.len();
    let boxed = buf.into_boxed_slice();
    let array_ptr = Box::into_raw(boxed) as *mut IdentityPublicKeyFFI;
    unsafe {
        *out_keys = array_ptr;
        *out_count = count;
    }
    PlatformWalletFFIResult::ok()
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
        if !entry.data_ptr.is_null() && entry.data_len > 0 {
            let data_slice =
                unsafe { std::slice::from_raw_parts_mut(entry.data_ptr, entry.data_len) };
            let _ = unsafe { Box::from_raw(data_slice as *mut [u8]) };
            entry.data_ptr = std::ptr::null_mut();
            entry.data_len = 0;
        }
    }
    let _ = unsafe { Box::from_raw(slice as *mut [IdentityPublicKeyFFI]) };
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
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::prelude::Identifier;
    use std::collections::BTreeMap;

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
}
