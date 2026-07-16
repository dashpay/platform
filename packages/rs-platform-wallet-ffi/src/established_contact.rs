//! Established contact FFI functions
//!
//! Provides access to established contact details and the associated contact requests

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::EstablishedContact;

// Storage for established contacts
lazy_static::lazy_static! {
    pub static ref ESTABLISHED_CONTACT_STORAGE: HandleStorage<EstablishedContact> = HandleStorage::new();
}

/// Get an established contact by ID from a managed identity.
/// `contact_id` is a `*const u8` to a 32-byte identifier buffer.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_established_contact(
    identity_handle: Handle,
    contact_id: *const u8,
    out_contact_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_contact_handle);

    let contact_identifier = unwrap_result_or_return!(unsafe { read_identifier(contact_id) });

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .established_contacts()
            .get(&contact_identifier)
            .cloned()
    });
    let inner = unwrap_option_or_return!(option);
    let contact = unwrap_option_or_return!(inner);
    unsafe { *out_contact_handle = ESTABLISHED_CONTACT_STORAGE.insert(contact) };
    PlatformWalletFFIResult::ok()
}

/// Get the contact identity ID from an established contact into a
/// 32-byte out-buffer.
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_contact_id(
    contact_handle: Handle,
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_id);

    let option = ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| contact.contact_identity_id);
    let id = unwrap_option_or_return!(option);
    unsafe { write_identifier(out_id, &id) };
    PlatformWalletFFIResult::ok()
}

/// Get a handle to the outgoing contact request from an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_outgoing_request(
    contact_handle: Handle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_request_handle);

    let option = ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| contact.outgoing_request.clone());
    let req = unwrap_option_or_return!(option);
    unsafe {
        *out_request_handle = crate::contact_request::CONTACT_REQUEST_STORAGE.insert(req);
    }
    PlatformWalletFFIResult::ok()
}

/// Get a handle to the incoming contact request from an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_incoming_request(
    contact_handle: Handle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_request_handle);

    let option = ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| contact.incoming_request.clone());
    let req = unwrap_option_or_return!(option);
    unsafe {
        *out_request_handle = crate::contact_request::CONTACT_REQUEST_STORAGE.insert(req);
    }
    PlatformWalletFFIResult::ok()
}

/// Get the contact identity ID from an established contact (alias
/// for [`established_contact_get_contact_id`]). `out_id` must point
/// at writable storage of at least 32 bytes.
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_contact_identity_id(
    contact_handle: Handle,
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    unsafe { established_contact_get_contact_id(contact_handle, out_id) }
}

/// Get the alias for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_alias(
    contact_handle: Handle,
    out_alias: *mut *mut std::os::raw::c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_alias);
    *out_alias = std::ptr::null_mut();

    let option =
        ESTABLISHED_CONTACT_STORAGE.with_item(contact_handle, |contact| contact.alias.clone());
    let option = unwrap_option_or_return!(option);
    let alias = unwrap_option_or_return!(option);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(alias));
    unsafe { *out_alias = c_str.into_raw() };
    PlatformWalletFFIResult::ok()
}

/// Set the alias for an established contact
/// **Handle-local only**: mutates the clone held by this handle, NOT the
/// wallet manager's contact state — the change is never persisted and is
/// lost when the handle is freed. Real writes go through
/// `platform_wallet_set_dashpay_contact_info_with_signer`.
#[no_mangle]
pub unsafe extern "C" fn established_contact_set_alias(
    contact_handle: Handle,
    alias: *const std::os::raw::c_char,
) -> PlatformWalletFFIResult {
    let alias_str = if alias.is_null() {
        None
    } else {
        unsafe {
            Some(unwrap_result_or_return!(std::ffi::CStr::from_ptr(alias).to_str()).to_string())
        }
    };

    let option = ESTABLISHED_CONTACT_STORAGE.with_item_mut(contact_handle, |contact| {
        if let Some(a) = alias_str {
            contact.set_alias(a);
        }
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Clear the alias for an established contact
/// **Handle-local only**: mutates the clone held by this handle, NOT the
/// wallet manager's contact state — the change is never persisted and is
/// lost when the handle is freed. Real writes go through
/// `platform_wallet_set_dashpay_contact_info_with_signer`.
#[no_mangle]
pub unsafe extern "C" fn established_contact_clear_alias(
    contact_handle: Handle,
) -> PlatformWalletFFIResult {
    let option = ESTABLISHED_CONTACT_STORAGE.with_item_mut(contact_handle, |contact| {
        contact.clear_alias();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get the note for an established contact
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_note(
    contact_handle: Handle,
    out_note: *mut *mut std::os::raw::c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_note);
    // Null the out-pointer before the fallible lookup so a cleanup-on-error
    // caller that unconditionally `platform_wallet_string_free`s the variable
    // frees null, not garbage — matching the null-sentinel-first convention
    // used across the rest of this FFI surface.
    unsafe { *out_note = std::ptr::null_mut() };

    let option =
        ESTABLISHED_CONTACT_STORAGE.with_item(contact_handle, |contact| contact.note.clone());
    let option = unwrap_option_or_return!(option);
    let note = unwrap_option_or_return!(option);
    let c_str = unwrap_result_or_return!(std::ffi::CString::new(note));
    unsafe { *out_note = c_str.into_raw() };
    PlatformWalletFFIResult::ok()
}

/// Set the note for an established contact
/// **Handle-local only**: mutates the clone held by this handle, NOT the
/// wallet manager's contact state — the change is never persisted and is
/// lost when the handle is freed. Real writes go through
/// `platform_wallet_set_dashpay_contact_info_with_signer`.
#[no_mangle]
pub unsafe extern "C" fn established_contact_set_note(
    contact_handle: Handle,
    note: *const std::os::raw::c_char,
) -> PlatformWalletFFIResult {
    let note_str = if note.is_null() {
        None
    } else {
        unsafe {
            Some(unwrap_result_or_return!(std::ffi::CStr::from_ptr(note).to_str()).to_string())
        }
    };

    let option = ESTABLISHED_CONTACT_STORAGE.with_item_mut(contact_handle, |contact| {
        if let Some(n) = note_str {
            contact.set_note(n);
        }
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Clear the note for an established contact
/// **Handle-local only**: mutates the clone held by this handle, NOT the
/// wallet manager's contact state — the change is never persisted and is
/// lost when the handle is freed. Real writes go through
/// `platform_wallet_set_dashpay_contact_info_with_signer`.
#[no_mangle]
pub unsafe extern "C" fn established_contact_clear_note(
    contact_handle: Handle,
) -> PlatformWalletFFIResult {
    let option = ESTABLISHED_CONTACT_STORAGE.with_item_mut(contact_handle, |contact| {
        contact.clear_note();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Check if an established contact is hidden
#[no_mangle]
pub unsafe extern "C" fn established_contact_is_hidden(
    contact_handle: Handle,
    out_is_hidden: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_is_hidden);

    let option = ESTABLISHED_CONTACT_STORAGE.with_item(contact_handle, |contact| contact.is_hidden);
    *out_is_hidden = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Check whether an established contact's DashPay payment channel is
/// permanently broken.
///
/// `true` means the account-building sweep hit a permanent failure
/// (decrypt/decode of the counterparty xpub, or a key-index validation
/// failure) and stopped retrying. The UI should disable "Send Dash" and
/// surface "Payment channel broken — ask the contact to send a new
/// request"; the flag clears automatically when a superseding contact
/// request (re-)establishes the relationship.
#[no_mangle]
pub unsafe extern "C" fn established_contact_is_payment_channel_broken(
    contact_handle: Handle,
    out_is_broken: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_is_broken);

    let option = ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| contact.payment_channel_broken);
    *out_is_broken = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Hide an established contact from the contact list
/// **Handle-local only**: mutates the clone held by this handle, NOT the
/// wallet manager's contact state — the change is never persisted and is
/// lost when the handle is freed. Real writes go through
/// `platform_wallet_set_dashpay_contact_info_with_signer`.
#[no_mangle]
pub unsafe extern "C" fn established_contact_hide(
    contact_handle: Handle,
) -> PlatformWalletFFIResult {
    let option = ESTABLISHED_CONTACT_STORAGE.with_item_mut(contact_handle, |contact| {
        contact.hide();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Unhide an established contact
/// **Handle-local only**: mutates the clone held by this handle, NOT the
/// wallet manager's contact state — the change is never persisted and is
/// lost when the handle is freed. Real writes go through
/// `platform_wallet_set_dashpay_contact_info_with_signer`.
#[no_mangle]
pub unsafe extern "C" fn established_contact_unhide(
    contact_handle: Handle,
) -> PlatformWalletFFIResult {
    let option = ESTABLISHED_CONTACT_STORAGE.with_item_mut(contact_handle, |contact| {
        contact.unhide();
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Destroy an established contact handle and free resources
#[no_mangle]
pub unsafe extern "C" fn established_contact_destroy(
    contact_handle: Handle,
) -> PlatformWalletFFIResult {
    let option = ESTABLISHED_CONTACT_STORAGE.remove(contact_handle);
    let _ = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

// Tests for this module are in tests/comprehensive_tests.rs
