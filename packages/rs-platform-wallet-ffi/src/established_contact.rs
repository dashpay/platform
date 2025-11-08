//! Established contact FFI functions
//!
//! Provides access to established contact details and the associated contact requests

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use platform_wallet::EstablishedContact;

// Storage for established contacts
lazy_static::lazy_static! {
    pub static ref ESTABLISHED_CONTACT_STORAGE: HandleStorage<EstablishedContact> = HandleStorage::new();
}

/// Get an established contact by ID from a managed identity
#[no_mangle]
pub extern "C" fn managed_identity_get_established_contact(
    identity_handle: Handle,
    contact_id: IdentifierBytes,
    out_contact_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_contact_handle.is_null() {
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

    let contact_identifier = match contact_id.to_identifier() {
        Ok(id) => id,
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
            match identity.established_contacts.get(&contact_identifier) {
                Some(contact) => {
                    let handle = ESTABLISHED_CONTACT_STORAGE.insert(contact.clone());
                    unsafe { *out_contact_handle = handle };
                    PlatformWalletFFIResult::Success
                }
                None => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorContactNotFound,
                                "Established contact not found",
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorContactNotFound
                }
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

/// Get the contact identity ID from an established contact
#[no_mangle]
pub extern "C" fn established_contact_get_contact_id(
    contact_handle: Handle,
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

    ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| {
            unsafe { *out_id = contact.contact_identity_id.into() };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get a handle to the outgoing contact request from an established contact
#[no_mangle]
pub extern "C" fn established_contact_get_outgoing_request(
    contact_handle: Handle,
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

    ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| {
            let handle = crate::contact_request::CONTACT_REQUEST_STORAGE
                .insert(contact.outgoing_request.clone());
            unsafe { *out_request_handle = handle };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Get a handle to the incoming contact request from an established contact
#[no_mangle]
pub extern "C" fn established_contact_get_incoming_request(
    contact_handle: Handle,
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

    ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| {
            let handle = crate::contact_request::CONTACT_REQUEST_STORAGE
                .insert(contact.incoming_request.clone());
            unsafe { *out_request_handle = handle };
            PlatformWalletFFIResult::Success
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid contact handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

/// Destroy an established contact handle and free resources
#[no_mangle]
pub extern "C" fn established_contact_destroy(contact_handle: Handle) -> PlatformWalletFFIResult {
    if ESTABLISHED_CONTACT_STORAGE.remove(contact_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

// Tests for this module are in tests/comprehensive_tests.rs
