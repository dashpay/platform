//! Contact request FFI functions
//!
//! Provides access to individual contact request fields

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use platform_wallet::ContactRequest;

// Storage for contact requests
lazy_static::lazy_static! {
    pub static ref CONTACT_REQUEST_STORAGE: HandleStorage<ContactRequest> = HandleStorage::new();
}

/// Create a new contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_create(
    sender_id: IdentifierBytes,
    recipient_id: IdentifierBytes,
    sender_key_index: u32,
    recipient_key_index: u32,
    account_reference: u32,
    encrypted_public_key_bytes: *const std::os::raw::c_uchar,
    encrypted_public_key_len: usize,
    core_height_created_at: u32,
    created_at: u64,
    out_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if encrypted_public_key_bytes.is_null() || out_handle.is_null() {
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

    let sender = match sender_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid sender identifier: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let recipient = match recipient_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid recipient identifier: {}", e),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let encrypted_key =
        unsafe { std::slice::from_raw_parts(encrypted_public_key_bytes, encrypted_public_key_len) }
            .to_vec();

    let contact_request = ContactRequest::new(
        sender,
        recipient,
        sender_key_index,
        recipient_key_index,
        account_reference,
        encrypted_key,
        core_height_created_at,
        created_at,
    );

    let handle = CONTACT_REQUEST_STORAGE.insert(contact_request);
    unsafe { *out_handle = handle };

    PlatformWalletFFIResult::Success
}

/// Create a contact request handle from a managed identity's sent request
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_sent_contact_request(
    identity_handle: Handle,
    recipient_id: IdentifierBytes,
    out_request_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_request_handle.is_null() {
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

    let id = match recipient_id.to_identifier() {
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
            if let Some(request) = identity.sent_contact_requests.get(&id) {
                let handle = CONTACT_REQUEST_STORAGE.insert(request.clone());
                unsafe { *out_request_handle = handle };
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

/// Create a contact request handle from a managed identity's incoming request
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_incoming_contact_request(
    identity_handle: Handle,
    sender_id: IdentifierBytes,
    out_request_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_request_handle.is_null() {
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
        .with_item(identity_handle, |identity| {
            if let Some(request) = identity.incoming_contact_requests.get(&id) {
                let handle = CONTACT_REQUEST_STORAGE.insert(request.clone());
                unsafe { *out_request_handle = handle };
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

/// Get sender ID from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_sender_id(
    request_handle: Handle,
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            unsafe { *out_id = request.sender_id.into() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get recipient ID from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_recipient_id(
    request_handle: Handle,
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            unsafe { *out_id = request.recipient_id.into() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get sender key index from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_sender_key_index(
    request_handle: Handle,
    out_index: *mut u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_index.is_null() {
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            unsafe { *out_index = request.sender_key_index };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get recipient key index from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_recipient_key_index(
    request_handle: Handle,
    out_index: *mut u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_index.is_null() {
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            unsafe { *out_index = request.recipient_key_index };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get account reference from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_account_reference(
    request_handle: Handle,
    out_account_ref: *mut u32,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_account_ref.is_null() {
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            unsafe { *out_account_ref = request.account_reference };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get encrypted public key from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_encrypted_public_key(
    request_handle: Handle,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_bytes.is_null() || out_len.is_null() {
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            let bytes = request.encrypted_public_key.clone().into_boxed_slice();
            let len = bytes.len();
            let ptr = Box::into_raw(bytes) as *mut u8;

            unsafe {
                *out_bytes = ptr;
                *out_len = len;
            }
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get creation timestamp from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_created_at(
    request_handle: Handle,
    out_timestamp: *mut u64,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_timestamp.is_null() {
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

    CONTACT_REQUEST_STORAGE
        .with_item(request_handle, |request| {
            unsafe { *out_timestamp = request.created_at };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact request handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Destroy contact request handle
#[no_mangle]
pub extern "C" fn contact_request_destroy(request_handle: Handle) -> PlatformWalletFFIResult {
    if CONTACT_REQUEST_STORAGE.remove(request_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;

    #[test]
    fn test_contact_request_getters() {
        unsafe {
            let sender_id = Identifier::from([1u8; 32]);
            let recipient_id = Identifier::from([2u8; 32]);
            let encrypted_key = vec![5u8; 96];

            let request = ContactRequest::new(
                sender_id,
                recipient_id,
                0,
                1,
                42,
                encrypted_key.clone(),
                100_000,
                1_700_000_000,
            );

            let handle = CONTACT_REQUEST_STORAGE.insert(request);
            let mut error = PlatformWalletFFIError::success();

            // Test sender ID
            let mut out_id = IdentifierBytes { bytes: [0u8; 32] };
            let result = contact_request_get_sender_id(handle, &mut out_id, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(out_id.bytes, [1u8; 32]);

            // Test recipient ID
            let result = contact_request_get_recipient_id(handle, &mut out_id, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(out_id.bytes, [2u8; 32]);

            // Test sender key index
            let mut sender_key_idx = 0u32;
            let result =
                contact_request_get_sender_key_index(handle, &mut sender_key_idx, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(sender_key_idx, 0);

            // Test recipient key index
            let mut recipient_key_idx = 0u32;
            let result =
                contact_request_get_recipient_key_index(handle, &mut recipient_key_idx, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(recipient_key_idx, 1);

            // Test account reference
            let mut account_ref = 0u32;
            let result =
                contact_request_get_account_reference(handle, &mut account_ref, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(account_ref, 42);

            // Test created_at
            let mut created_at = 0u64;
            let result = contact_request_get_created_at(handle, &mut created_at, &mut error);
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(created_at, 1_700_000_000);

            // Test encrypted public key
            let mut bytes_ptr: *mut u8 = std::ptr::null_mut();
            let mut len: usize = 0;
            let result = contact_request_get_encrypted_public_key(
                handle,
                &mut bytes_ptr,
                &mut len,
                &mut error,
            );
            assert_eq!(result, PlatformWalletFFIResult::Success);
            assert_eq!(len, 96);
            assert!(!bytes_ptr.is_null());

            let bytes_slice = std::slice::from_raw_parts(bytes_ptr, len);
            assert_eq!(bytes_slice, &encrypted_key[..]);

            // Clean up
            crate::platform_wallet_bytes_free(bytes_ptr, len);
            contact_request_destroy(handle);
        }
    }
}
