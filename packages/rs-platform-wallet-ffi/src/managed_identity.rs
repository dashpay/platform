use crate::error::*;
use crate::handle::*;
use crate::types::*;
use platform_wallet::managed_identity::ManagedIdentity;
use std::os::raw::c_char;

/// Create a new ManagedIdentity from a DPP Identity
#[no_mangle]
pub extern "C" fn managed_identity_create_from_identity_bytes(
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

    match dpp::prelude::Identity::from_buffer(bytes) {
        Ok(identity) => {
            let managed = ManagedIdentity::new(identity);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);
            unsafe { *out_handle = handle };
            PlatformWalletFFIResult::Success
        }
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorDeserialization,
                        format!("Failed to deserialize identity: {}", e),
                    );
                }
            }
            PlatformWalletFFIResult::ErrorDeserialization
        }
    }
}

/// Get the identity ID
#[no_mangle]
pub extern "C" fn managed_identity_get_id(
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
pub extern "C" fn managed_identity_get_balance(
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
pub extern "C" fn managed_identity_get_label(
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
pub extern "C" fn managed_identity_set_label(
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
pub extern "C" fn managed_identity_get_last_updated_balance_block_time(
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
pub extern "C" fn managed_identity_set_last_updated_balance_block_time(
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
pub extern "C" fn managed_identity_get_last_synced_keys_block_time(
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

// Note: managed_identity_to_json is not currently available because
// ManagedIdentity does not implement Serialize. This would require
// significant work to add custom serialization for all internal types.

/// Destroy ManagedIdentity and free resources
#[no_mangle]
pub extern "C" fn managed_identity_destroy(identity_handle: Handle) -> PlatformWalletFFIResult {
    if MANAGED_IDENTITY_STORAGE.remove(identity_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_and_set_label() {
        let managed = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let label = std::ffi::CString::new("Test Identity").unwrap();
        let mut error = PlatformWalletFFIError::success();

        let result = managed_identity_set_label(handle, label.as_ptr(), &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        let mut label_ptr: *mut c_char = std::ptr::null_mut();
        let result = managed_identity_get_label(handle, &mut label_ptr, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert!(!label_ptr.is_null());

        let retrieved_label = unsafe { std::ffi::CStr::from_ptr(label_ptr).to_str().unwrap() };
        assert_eq!(retrieved_label, "Test Identity");

        // Cleanup
        platform_wallet_string_free(label_ptr);
        managed_identity_destroy(handle);
    }

    #[test]
    fn test_get_balance() {
        let managed = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let mut balance: u64 = 0;
        let mut error = PlatformWalletFFIError::success();

        let result = managed_identity_get_balance(handle, &mut balance, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Cleanup
        managed_identity_destroy(handle);
    }

    #[test]
    fn test_block_time_operations() {
        let managed = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let block_time = BlockTime {
            height: 100,
            core_height: 200,
            timestamp: 1234567890,
        };

        let mut error = PlatformWalletFFIError::success();

        // Set block time
        let result =
            managed_identity_set_last_updated_balance_block_time(handle, block_time, &mut error);
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
