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
pub unsafe extern "C" fn managed_identity_get_established_contact(
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
pub unsafe extern "C" fn established_contact_get_contact_id(
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
pub unsafe extern "C" fn established_contact_get_outgoing_request(
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
pub unsafe extern "C" fn established_contact_get_incoming_request(
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

/// Get the contact identity ID from an established contact (alias for established_contact_get_contact_id)
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_contact_identity_id(
    contact_handle: Handle,
    out_id: *mut IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    established_contact_get_contact_id(contact_handle, out_id, out_error)
}

/// Get the alias for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_alias(
    contact_handle: Handle,
    out_alias: *mut *mut std::os::raw::c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_alias.is_null() {
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
            if let Some(alias) = &contact.alias {
                match std::ffi::CString::new(alias.clone()) {
                    Ok(c_str) => {
                        unsafe { *out_alias = c_str.into_raw() };
                        PlatformWalletFFIResult::Success
                    }
                    Err(_) => {
                        unsafe { *out_alias = std::ptr::null_mut() };
                        PlatformWalletFFIResult::ErrorUtf8Conversion
                    }
                }
            } else {
                unsafe { *out_alias = std::ptr::null_mut() };
                PlatformWalletFFIResult::Success
            }
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

/// Set the alias for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_set_alias(
    contact_handle: Handle,
    alias: *const std::os::raw::c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let alias_str = if alias.is_null() {
        None
    } else {
        unsafe {
            match std::ffi::CStr::from_ptr(alias).to_str() {
                Ok(s) => Some(s.to_string()),
                Err(_) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            "Invalid UTF-8 in alias",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorUtf8Conversion;
                }
            }
        }
    };

    ESTABLISHED_CONTACT_STORAGE
        .with_item_mut(contact_handle, |contact| {
            if let Some(a) = alias_str {
                contact.set_alias(a);
            }
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

/// Clear the alias for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_clear_alias(
    contact_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    ESTABLISHED_CONTACT_STORAGE
        .with_item_mut(contact_handle, |contact| {
            contact.clear_alias();
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

/// Get the note for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_note(
    contact_handle: Handle,
    out_note: *mut *mut std::os::raw::c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_note.is_null() {
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
            if let Some(note) = &contact.note {
                match std::ffi::CString::new(note.clone()) {
                    Ok(c_str) => {
                        unsafe { *out_note = c_str.into_raw() };
                        PlatformWalletFFIResult::Success
                    }
                    Err(_) => {
                        unsafe { *out_note = std::ptr::null_mut() };
                        PlatformWalletFFIResult::ErrorUtf8Conversion
                    }
                }
            } else {
                unsafe { *out_note = std::ptr::null_mut() };
                PlatformWalletFFIResult::Success
            }
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

/// Set the note for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_set_note(
    contact_handle: Handle,
    note: *const std::os::raw::c_char,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let note_str = if note.is_null() {
        None
    } else {
        unsafe {
            match std::ffi::CStr::from_ptr(note).to_str() {
                Ok(s) => Some(s.to_string()),
                Err(_) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            "Invalid UTF-8 in note",
                        );
                    }
                    return PlatformWalletFFIResult::ErrorUtf8Conversion;
                }
            }
        }
    };

    ESTABLISHED_CONTACT_STORAGE
        .with_item_mut(contact_handle, |contact| {
            if let Some(n) = note_str {
                contact.set_note(n);
            }
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

/// Clear the note for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_clear_note(
    contact_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    ESTABLISHED_CONTACT_STORAGE
        .with_item_mut(contact_handle, |contact| {
            contact.clear_note();
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

/// Check if an established contact is hidden
#[no_mangle]
pub unsafe extern "C" fn established_contact_is_hidden(
    contact_handle: Handle,
    out_is_hidden: *mut bool,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_is_hidden.is_null() {
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
            unsafe { *out_is_hidden = contact.is_hidden };
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

/// Hide an established contact from the contact list
#[no_mangle]
pub unsafe extern "C" fn established_contact_hide(
    contact_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    ESTABLISHED_CONTACT_STORAGE
        .with_item_mut(contact_handle, |contact| {
            contact.hide();
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

/// Unhide an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_unhide(
    contact_handle: Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    ESTABLISHED_CONTACT_STORAGE
        .with_item_mut(contact_handle, |contact| {
            contact.unhide();
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
pub unsafe extern "C" fn established_contact_destroy(
    contact_handle: Handle,
) -> PlatformWalletFFIResult {
    if ESTABLISHED_CONTACT_STORAGE.remove(contact_handle).is_some() {
        PlatformWalletFFIResult::Success
    } else {
        PlatformWalletFFIResult::ErrorInvalidHandle
    }
}

// Tests for this module are in tests/comprehensive_tests.rs
