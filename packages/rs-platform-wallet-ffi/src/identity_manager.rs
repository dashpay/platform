use crate::error::*;
use crate::handle::*;
use crate::types::*;
use platform_wallet::identity_manager::IdentityManager;
use std::os::raw::c_char;

/// Create a new empty IdentityManager
#[no_mangle]
pub extern "C" fn identity_manager_create(
    out_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_handle.is_null() {
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

    let manager = IdentityManager::default();
    let handle = IDENTITY_MANAGER_STORAGE.insert(manager);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Add a managed identity to the manager
#[no_mangle]
pub extern "C" fn identity_manager_add_identity(
    manager_handle: Handle,
    identity_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let identity_result =
        MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| identity.clone());

    let identity = match identity_result {
        Some(i) => i,
        None => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid identity handle",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidHandle;
        }
    };

    IDENTITY_MANAGER_STORAGE
        .with_item_mut(manager_handle, |manager| {
            match manager.add_identity(identity.identity) {
                Ok(_) => PlatformWalletFFIResult::Success,
                Err(_) => PlatformWalletFFIResult::ErrorWalletOperation,
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Remove an identity from the manager
#[no_mangle]
pub extern "C" fn identity_manager_remove_identity(
    manager_handle: Handle,
    identity_id: IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let id = match identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identifier: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    IDENTITY_MANAGER_STORAGE
        .with_item_mut(manager_handle, |manager| {
            if manager.remove_identity(&id).is_ok() {
                PlatformWalletFFIResult::Success
            } else {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorIdentityNotFound,
                            "Identity not found",
                        );
                    }
                }
                PlatformWalletFFIResult::ErrorIdentityNotFound
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get an identity by ID
#[no_mangle]
pub extern "C" fn identity_manager_get_identity(
    manager_handle: Handle,
    identity_id: IdentifierBytes,
    out_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_handle.is_null() {
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

    let id = match identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identifier: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    IDENTITY_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            match manager.managed_identity(&id) {
                Some(identity) => {
                    let handle = MANAGED_IDENTITY_STORAGE.insert(identity.clone());
                    unsafe { *out_handle = handle };
                    PlatformWalletFFIResult::Success
                }
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorIdentityNotFound,
                                "Identity not found",
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorIdentityNotFound
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get all identity IDs
#[no_mangle]
pub extern "C" fn identity_manager_get_all_identity_ids(
    manager_handle: Handle,
    out_array: *mut IdentifierArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
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

    IDENTITY_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            let ids: Vec<dpp::prelude::Identifier> = manager.identities.keys().cloned().collect();
            let array = IdentifierArray::new(ids);
            unsafe { *out_array = array };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get the primary identity ID
#[no_mangle]
pub extern "C" fn identity_manager_get_primary_identity_id(
    manager_handle: Handle,
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

    IDENTITY_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            if let Some(primary_id) = manager.primary_identity_id {
                unsafe { *out_id = primary_id.into() };
                PlatformWalletFFIResult::Success
            } else {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorIdentityNotFound,
                            "No primary identity set",
                        );
                    }
                }
                PlatformWalletFFIResult::ErrorIdentityNotFound
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Set the primary identity
#[no_mangle]
pub extern "C" fn identity_manager_set_primary_identity(
    manager_handle: Handle,
    identity_id: IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let id = match identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identifier: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    IDENTITY_MANAGER_STORAGE
        .with_item_mut(manager_handle, |manager| {
            manager.set_primary_identity(id);
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get the count of identities
#[no_mangle]
pub extern "C" fn identity_manager_get_identity_count(
    manager_handle: Handle,
    out_count: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_count.is_null() {
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

    IDENTITY_MANAGER_STORAGE
        .with_item(manager_handle, |manager| {
            unsafe { *out_count = manager.identities.len() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid manager handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Destroy IdentityManager and free resources
#[no_mangle]
pub extern "C" fn identity_manager_destroy(manager_handle: Handle) -> PlatformWalletFFIResult {
    if IDENTITY_MANAGER_STORAGE.remove(manager_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use platform_wallet::managed_identity::ManagedIdentity;
    use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    fn create_test_identity() -> Identity {
        let id = Identifier::from([1u8; 32]);
        let mut public_keys = BTreeMap::new();

        public_keys.insert(
            0,
            IdentityPublicKey::V0(dpp::identity::identity_public_key::v0::IdentityPublicKeyV0 {
                id: 0,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                read_only: false,
                data: dpp::platform_value::BinaryData::new(vec![2u8; 33]),
                disabled_at: None,
                contract_bounds: None,
            }),
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
    fn test_create_identity_manager() {
        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        let result = identity_manager_create(&mut handle, &mut error);

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_ne!(handle, NULL_HANDLE);

        // Cleanup
        identity_manager_destroy(handle);
    }

    #[test]
    fn test_get_identity_count() {
        let mut handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        identity_manager_create(&mut handle, &mut error);

        let mut count: usize = 0;
        let result = identity_manager_get_identity_count(handle, &mut count, &mut error);

        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(count, 0);

        // Cleanup
        identity_manager_destroy(handle);
    }

    #[test]
    fn test_set_and_get_primary_identity() {
        let mut manager_handle: Handle = NULL_HANDLE;
        let mut error = PlatformWalletFFIError::success();

        identity_manager_create(&mut manager_handle, &mut error);

        let identity_id = Identifier::random();
        let id_bytes: IdentifierBytes = identity_id.into();

        // Create and add a managed identity
        let identity = create_test_identity();
        let managed_identity = ManagedIdentity::new(identity);
        let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed_identity);

        identity_manager_add_identity(manager_handle, identity_handle, &mut error);

        // Set primary identity
        let result = identity_manager_set_primary_identity(manager_handle, id_bytes, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Get primary identity
        let mut retrieved_id = IdentifierBytes { bytes: [0u8; 32] };
        let result =
            identity_manager_get_primary_identity_id(manager_handle, &mut retrieved_id, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);

        // Cleanup
        identity_manager_destroy(manager_handle);
    }

    #[test]
    fn test_destroy_invalid_handle() {
        let result = identity_manager_destroy(9999);
        assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
    }
}
