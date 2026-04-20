use crate::error::*;
use crate::handle::*;
use crate::types::*;
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
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if identity_bytes.is_null() || out_handle.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let bytes = unsafe { std::slice::from_raw_parts(identity_bytes, identity_len) };

    // Deserialize Identity from bytes
    let identity = match dpp::identity::Identity::deserialize_from_bytes_no_limit(bytes) {
        Ok(id) => id,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorDeserialization,
                        format!("Failed to deserialize identity: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorDeserialization;
        }
    };

    // Create ManagedIdentity from the deserialized Identity
    let managed_identity = ManagedIdentity::new(identity, 0);

    // Store in handle storage
    let handle = MANAGED_IDENTITY_STORAGE.insert(managed_identity);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Get the identity ID
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_id(
    identity_handle: Handle,
    out_id: *mut IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_id.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            unsafe { *out_id = identity.identity.id().into() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get the identity balance
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_balance(
    identity_handle: Handle,
    out_balance: *mut u64,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_balance.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            unsafe { *out_balance = identity.identity.balance() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get the label
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_label(
    identity_handle: Handle,
    out_label: *mut *mut c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_label.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            if let Some(label) = &identity.label {
                match std::ffi::CString::new(label.clone()) {
                    Ok(c_str) => {
                        unsafe { *out_label = c_str.into_raw() };
                        PlatformWalletFFIResult::Success
                    }
                    Err(_) => {
                        unsafe { *out_label = std::ptr::null_mut() };
                        PlatformWalletFFIResult::ErrorUtf8Conversion
                    }
                }
            } else {
                unsafe { *out_label = std::ptr::null_mut() };
                PlatformWalletFFIResult::Success
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Set the label
#[no_mangle]
pub unsafe extern "C" fn managed_identity_set_label(
    identity_handle: Handle,
    label: *const c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let label_str = if label.is_null() {
        None
    } else {
        unsafe {
            match std::ffi::CStr::from_ptr(label).to_str() {
                Ok(s) => Some(s.to_string()),
                Err(_) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            "Invalid UTF-8 in label",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorUtf8Conversion;
                }
            }
        }
    };

    MANAGED_IDENTITY_STORAGE
        .with_item_mut(identity_handle, |identity| {
            identity.label = label_str;
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get last updated balance block time
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_last_updated_balance_block_time(
    identity_handle: Handle,
    out_block_time: *mut BlockTime,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_block_time.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            if let Some(bt) = identity.last_updated_balance_block_time {
                unsafe { *out_block_time = bt.into() };
                PlatformWalletFFIResult::Success
            } else {
                // Return zeroed block time if None
                unsafe {
                    *out_block_time = BlockTime {
                        height: 0,
                        core_height: 0,
                        timestamp: 0,
                    };
                }
                PlatformWalletFFIResult::Success
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Set last updated balance block time
#[no_mangle]
pub unsafe extern "C" fn managed_identity_set_last_updated_balance_block_time(
    identity_handle: Handle,
    block_time: BlockTime,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    MANAGED_IDENTITY_STORAGE
        .with_item_mut(identity_handle, |identity| {
            identity.last_updated_balance_block_time = Some(block_time.into());
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get last synced keys block time
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_last_synced_keys_block_time(
    identity_handle: Handle,
    out_block_time: *mut BlockTime,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_block_time.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            if let Some(bt) = identity.last_synced_keys_block_time {
                unsafe { *out_block_time = bt.into() };
                PlatformWalletFFIResult::Success
            } else {
                // Return zeroed block time if None
                unsafe {
                    *out_block_time = BlockTime {
                        height: 0,
                        core_height: 0,
                        timestamp: 0,
                    };
                }
                PlatformWalletFFIResult::Success
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get the identity revision.
///
/// Revisions advance on every platform-side state transition
/// (key add/remove, balance change, etc.). Value 0 indicates the
/// identity was just created and has not been mutated.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_revision(
    identity_handle: Handle,
    out_revision: *mut u64,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_revision.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            unsafe { *out_revision = identity.identity.revision() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Flat C representation of an `IdentityPublicKey` for the Swift
/// wrapper. `disabled_at_is_some` / `disabled_at` together encode
/// the `Option<TimestampMillis>` because C can't express Option
/// directly. `data_ptr` / `data_len` own a heap-allocated copy of
/// the public-key payload; the whole array plus the individual data
/// buffers are released by
/// [`managed_identity_free_public_keys`].
#[repr(C)]
pub struct IdentityPublicKeyFFI {
    /// DPP `KeyID` — matches `IdentityPublicKeyV0::id`.
    pub key_id: u32,
    /// `Purpose` as its `#[repr(u8)]` discriminant.
    pub purpose: u8,
    /// `SecurityLevel` as its `#[repr(u8)]` discriminant.
    pub security_level: u8,
    /// `KeyType` as its `#[repr(u8)]` discriminant.
    pub key_type: u8,
    pub read_only: bool,
    /// Set iff `disabled_at.is_some()`.
    pub disabled_at_is_some: bool,
    /// Milliseconds-since-epoch timestamp; ignore unless
    /// `disabled_at_is_some` is true.
    pub disabled_at: u64,
    /// Heap-owned public-key bytes (compressed secp256k1, BLS,
    /// hash160, etc. — depends on `key_type`). Freed with the whole
    /// array by `managed_identity_free_public_keys`.
    pub data_ptr: *mut u8,
    pub data_len: usize,
}

/// Snapshot every `IdentityPublicKey` on the identity into a flat
/// heap-allocated array. The caller takes ownership of the array
/// pointer and of the inner `data_ptr` blobs; both are released by
/// a single call to [`managed_identity_free_public_keys`].
///
/// On success `out_keys` / `out_count` are populated. `out_count` is
/// zero for an identity with no keys and `out_keys` is `null`.
/// Contract bounds are intentionally omitted — the first-pass use
/// site (address-funded registration) never sets them, and bridging
/// the `ContractBounds` enum would expand the FFI surface.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_public_keys(
    identity_handle: Handle,
    out_keys: *mut *mut IdentityPublicKeyFFI,
    out_count: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_keys.is_null() || out_count.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "Null pointer provided",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            let keys = identity.identity.public_keys();
            if keys.is_empty() {
                unsafe {
                    *out_keys = std::ptr::null_mut();
                    *out_count = 0;
                }
                return PlatformWalletFFIResult::Success;
            }

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

            let count = buf.len();
            let boxed = buf.into_boxed_slice();
            let array_ptr = Box::into_raw(boxed) as *mut IdentityPublicKeyFFI;
            unsafe {
                *out_keys = array_ptr;
                *out_count = count;
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Release an array previously returned by
/// [`managed_identity_get_public_keys`]. Walks the array to free
/// every `data_ptr` blob before releasing the array itself.
///
/// Safe to call with `keys = null` / `count = 0` — both are no-ops.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_free_public_keys(
    keys: *mut IdentityPublicKeyFFI,
    count: usize,
) {
    if keys.is_null() || count == 0 {
        return;
    }
    // Reconstruct the slice box so we can iterate + free inner data.
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

// Note: managed_identity_to_json is not currently available because
// ManagedIdentity does not implement Serialize. This would require
// significant work to add custom serialization for all internal types.

/// Destroy ManagedIdentity and free resources
#[no_mangle]
pub unsafe extern "C" fn managed_identity_destroy(
    identity_handle: Handle,
) -> PlatformWalletFFIResult {
    if MANAGED_IDENTITY_STORAGE.remove(identity_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
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
    fn test_get_and_set_label() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let label = std::ffi::CString::new("Test Identity").unwrap();
            let mut error = PlatformWalletFFIError::success();

            let result = managed_identity_set_label(handle, label.as_ptr(), &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);

            let mut label_ptr: *mut c_char = std::ptr::null_mut();
            let result = managed_identity_get_label(handle, &mut label_ptr, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert!(!label_ptr.is_null());

            let retrieved_label = std::ffi::CStr::from_ptr(label_ptr).to_str().unwrap();
            assert_eq!(retrieved_label, "Test Identity");

            // Cleanup
            platform_wallet_string_free(label_ptr);
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
            let mut error = PlatformWalletFFIError::success();

            let result = managed_identity_get_balance(handle, &mut balance, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);

            // Cleanup
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

            let mut error = PlatformWalletFFIError::success();

            // Set block time
            let result = managed_identity_set_last_updated_balance_block_time(
                handle, block_time, &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);

            // Get block time
            let mut retrieved_bt = BlockTime {
                height: 0,
                core_height: 0,
                timestamp: 0,
            };
            let result = managed_identity_get_last_updated_balance_block_time(
                handle,
                &mut retrieved_bt,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(retrieved_bt.height, 100);
            assert_eq!(retrieved_bt.core_height, 200);
            assert_eq!(retrieved_bt.timestamp, 1234567890);

            // Cleanup
            managed_identity_destroy(handle);
        }
    }
}
