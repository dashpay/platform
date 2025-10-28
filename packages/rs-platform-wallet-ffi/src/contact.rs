use crate::error::*;
use crate::handle::*;
use crate::types::*;
use std::os::raw::c_char;

/// Get all sent contact request IDs
#[no_mangle]
pub extern "C" fn managed_identity_get_sent_contact_request_ids(
    identity_handle: Handle,
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

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            let ids: Vec<dpp::prelude::Identifier> =
                identity.sent_contact_requests.keys().cloned().collect();
            let array = IdentifierArray::new(ids);
            unsafe { *out_array = array };
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

/// Get all incoming contact request IDs
#[no_mangle]
pub extern "C" fn managed_identity_get_incoming_contact_request_ids(
    identity_handle: Handle,
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

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            let ids: Vec<dpp::prelude::Identifier> =
                identity.incoming_contact_requests.keys().cloned().collect();
            let array = IdentifierArray::new(ids);
            unsafe { *out_array = array };
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

/// Get all established contact IDs
#[no_mangle]
pub extern "C" fn managed_identity_get_established_contact_ids(
    identity_handle: Handle,
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

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            let ids: Vec<dpp::prelude::Identifier> =
                identity.established_contacts.keys().cloned().collect();
            let array = IdentifierArray::new(ids);
            unsafe { *out_array = array };
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

/// Check if a contact is established
#[no_mangle]
pub extern "C" fn managed_identity_is_contact_established(
    identity_handle: Handle,
    contact_id: IdentifierBytes,
    out_is_established: *mut bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_is_established.is_null() {
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

    let id = match contact_id.to_identifier() {
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

    MANAGED_IDENTITY_STORAGE
        .with_item(identity_handle, |identity| {
            unsafe { *out_is_established = identity.established_contacts.contains_key(&id) };
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;

    #[test]
    fn test_get_sent_contact_request_ids() {
        let identity = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut error = PlatformWalletFFIError::success();

        let result = managed_identity_get_sent_contact_request_ids(handle, &mut array, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(array.count, 0); // Should be empty for new identity

        // Cleanup
        platform_wallet_identifier_array_free(array);
        crate::managed_identity_destroy(handle);
    }

    #[test]
    fn test_get_incoming_contact_request_ids() {
        let identity = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut error = PlatformWalletFFIError::success();

        let result =
            managed_identity_get_incoming_contact_request_ids(handle, &mut array, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(array.count, 0);

        // Cleanup
        platform_wallet_identifier_array_free(array);
        crate::managed_identity_destroy(handle);
    }

    #[test]
    fn test_get_established_contact_ids() {
        let identity = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let mut array = IdentifierArray {
            items: std::ptr::null_mut(),
            count: 0,
        };
        let mut error = PlatformWalletFFIError::success();

        let result = managed_identity_get_established_contact_ids(handle, &mut array, &mut error);
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(array.count, 0);

        // Cleanup
        platform_wallet_identifier_array_free(array);
        crate::managed_identity_destroy(handle);
    }

    #[test]
    fn test_is_contact_established() {
        let identity = dpp::tests::fixtures::get_identity_fixture(None);
        let managed = platform_wallet::managed_identity::ManagedIdentity::new(identity);
        let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

        let contact_id = Identifier::random();
        let id_bytes: IdentifierBytes = contact_id.into();
        let mut error = PlatformWalletFFIError::success();

        let mut is_established = true;
        let result = managed_identity_is_contact_established(
            handle,
            id_bytes,
            &mut is_established,
            &mut error,
        );
        assert_eq!(result, PlatformWalletFFIResult::Success);
        assert_eq!(is_established, false);

        // Cleanup
        crate::managed_identity_destroy(handle);
    }
}
