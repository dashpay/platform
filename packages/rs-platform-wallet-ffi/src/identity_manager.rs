use crate::error::*;
use crate::handle::*;
use crate::types::*;
use platform_wallet::wallet::persister::{NoPlatformPersistence, WalletPersister};
use platform_wallet::IdentityManager;
use std::sync::Arc;

pub(crate) fn ffi_noop_persister() -> WalletPersister {
    WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
}

/// Create a new empty IdentityManager
#[no_mangle]
pub unsafe extern "C" fn identity_manager_create(
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

/// Add a managed identity to the manager.
///
/// Stand-alone identity-manager handles aren't bound to a wallet, so
/// the identity lands in the out-of-wallet bucket — the same place
/// observed identities go. Real wallet flows route through
/// [`crate::IdentityWallet`] APIs which thread `wallet_id` themselves.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_add_identity(
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
            match manager.add_out_of_wallet_identity(identity.identity, &ffi_noop_persister()) {
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

/// Remove an identity from the manager.
///
/// `identity_id` is a `*const u8` pointing at a 32-byte identifier
/// buffer. Pointer-passing rather than by-value `IdentifierBytes`
/// keeps the ABI safe across `@_silgen_name` (Swift would otherwise
/// hand the callee a garbage register slot for >16-byte aggregates).
#[no_mangle]
pub unsafe extern "C" fn identity_manager_remove_identity(
    manager_handle: Handle,
    identity_id: *const u8,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let id = match unsafe { read_identifier(identity_id) } {
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
            if manager.remove_identity(&id, &ffi_noop_persister()).is_ok() {
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

/// Get an identity by ID. `identity_id` is a `*const u8` to a
/// 32-byte buffer; see [`identity_manager_remove_identity`] for the
/// rationale on pointer-passing vs by-value.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_get_identity(
    manager_handle: Handle,
    identity_id: *const u8,
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

    let id = match unsafe { read_identifier(identity_id) } {
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

/// Get all identity IDs across both buckets.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_get_all_identity_ids(
    manager_handle: Handle,
    out_array: *mut IdentifierArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    use dpp::identity::accessors::IdentityGettersV0;

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
            let ids: Vec<dpp::prelude::Identifier> = manager
                .all_identities()
                .into_iter()
                .map(|i| i.id())
                .collect();
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

/// Get the count of identities across both buckets.
#[no_mangle]
pub unsafe extern "C" fn identity_manager_get_identity_count(
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
            unsafe { *out_count = manager.identity_count() };
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
pub unsafe extern "C" fn identity_manager_destroy(
    manager_handle: Handle,
) -> PlatformWalletFFIResult {
    if IDENTITY_MANAGER_STORAGE.remove(manager_handle).is_some() {
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
    use platform_wallet::ManagedIdentity;
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
    fn test_create_identity_manager() {
        unsafe {
            let mut handle: Handle = NULL_HANDLE;
            let mut error = PlatformWalletFFIError::success();

            let result = identity_manager_create(&mut handle, &mut error);

            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_ne!(handle, NULL_HANDLE);

            // Cleanup
            identity_manager_destroy(handle);
        }
    }

    #[test]
    fn test_get_identity_count() {
        unsafe {
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
    }

    #[test]
    fn test_add_and_lookup_out_of_wallet_identity() {
        // Standalone manager handle has no wallet — `add_identity`
        // routes into the out-of-wallet bucket. Verify lookup works
        // round-trip and the count reflects the insert.
        unsafe {
            let mut manager_handle: Handle = NULL_HANDLE;
            let mut error = PlatformWalletFFIError::success();

            identity_manager_create(&mut manager_handle, &mut error);

            let identity = create_test_identity();
            let id_bytes: [u8; 32] = [1u8; 32];
            let managed_identity = ManagedIdentity::new(identity, 0);
            let identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed_identity);

            identity_manager_add_identity(manager_handle, identity_handle, &mut error);

            let mut count: usize = 0;
            identity_manager_get_identity_count(manager_handle, &mut count, &mut error);
            assert_eq!(count, 1);

            let mut got: Handle = NULL_HANDLE;
            let result = identity_manager_get_identity(
                manager_handle,
                id_bytes.as_ptr(),
                &mut got,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_ne!(got, NULL_HANDLE);

            // Cleanup
            identity_manager_destroy(manager_handle);
        }
    }

    #[test]
    fn test_destroy_invalid_handle() {
        unsafe {
            let result = identity_manager_destroy(9999);
            assert_eq!(result, PlatformWalletFFIResult::ErrorInvalidHandle);
        }
    }
}
