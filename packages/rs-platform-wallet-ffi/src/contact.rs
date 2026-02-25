use crate::contact_request::CONTACT_REQUEST_STORAGE;
use crate::error::*;
use crate::handle::*;
use crate::types::*;

/// Get all sent contact request IDs
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_sent_contact_request_ids(
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
pub unsafe extern "C" fn managed_identity_get_incoming_contact_request_ids(
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
pub unsafe extern "C" fn managed_identity_get_established_contact_ids(
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
pub unsafe extern "C" fn managed_identity_is_contact_established(
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

/// Send a contact request from this identity to another
/// The request will be added to sent_contact_requests
/// If there's already an incoming request from the recipient, the contact will be automatically established
#[no_mangle]
pub unsafe extern "C" fn managed_identity_send_contact_request(
    identity_handle: Handle,
    request_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let request_result = CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone());

    let request = match request_result {
        Some(r) => r,
        None => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidHandle;
        }
    };

    MANAGED_IDENTITY_STORAGE
        .with_item_mut(identity_handle, |identity| {
            identity.add_sent_contact_request(request);
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

/// Accept an incoming contact request
/// This will add the request to incoming_contact_requests
/// If there's already a sent request to the sender, the contact will be automatically established
#[no_mangle]
pub unsafe extern "C" fn managed_identity_accept_contact_request(
    identity_handle: Handle,
    request_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let request_result = CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone());

    let request = match request_result {
        Some(r) => r,
        None => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidHandle;
        }
    };

    MANAGED_IDENTITY_STORAGE
        .with_item_mut(identity_handle, |identity| {
            identity.add_incoming_contact_request(request);
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

/// Reject an incoming contact request
/// This will remove the request from incoming_contact_requests
#[no_mangle]
pub unsafe extern "C" fn managed_identity_reject_contact_request(
    identity_handle: Handle,
    sender_id: IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let id = match sender_id.to_identifier() {
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
        .with_item_mut(identity_handle, |identity| {
            if identity.remove_incoming_contact_request(&id).is_some() {
                PlatformWalletFFIResult::Success
            } else {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorContactNotFound,
                            "Contact request not found",
                        );
                    }
                }
                PlatformWalletFFIResult::ErrorContactNotFound
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
    fn test_get_sent_contact_request_ids() {
        let identity = create_test_identity();
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
        let identity = create_test_identity();
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
        let identity = create_test_identity();
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
        let identity = create_test_identity();
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
