//! Established contact FFI functions
//!
//! Provides access to established contact details and the associated contact requests

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::EstablishedContact;

/// The bytes of a raw Orchard payment address: an 11-byte diversifier and a
/// 32-byte transmission key.
const CONTACT_SHIELDED_ADDRESS_LEN: usize = 43;

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

/// Read the shielded payment address the contact gave in their contact
/// request (`EstablishedContact::contact_shielded_address`).
///
/// `out_has_address` says whether the contact has one; when it does, the raw
/// 43 bytes (an 11-byte diversifier and a 32-byte transmission key) are
/// written to `out_address`, which is zeroed otherwise. The wallet decided
/// what it takes when it read the request; nothing is decided here.
///
/// # Safety
/// - `out_has_address` must be a valid, writable pointer.
/// - `out_address` must point to 43 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn established_contact_get_shielded_address(
    contact_handle: Handle,
    out_has_address: *mut bool,
    out_address: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_has_address);
    check_ptr!(out_address);

    let option = ESTABLISHED_CONTACT_STORAGE
        .with_item(contact_handle, |contact| contact.contact_shielded_address);
    let address = unwrap_option_or_return!(option);
    unsafe {
        *out_has_address = address.is_some();
        std::ptr::copy_nonoverlapping(
            address
                .unwrap_or([0u8; CONTACT_SHIELDED_ADDRESS_LEN])
                .as_ptr(),
            out_address,
            CONTACT_SHIELDED_ADDRESS_LEN,
        );
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::prelude::Identifier;
    use platform_wallet::ContactRequest;

    fn established(address: Option<[u8; 43]>) -> Handle {
        let us = Identifier::from([1u8; 32]);
        let them = Identifier::from([2u8; 32]);
        let mut contact = EstablishedContact::new(
            them,
            ContactRequest::new(us, them, 0, 0, 0, vec![0u8; 96], 100, 1),
            ContactRequest::new(them, us, 0, 0, 0, vec![0u8; 96], 101, 2),
        );
        contact.contact_shielded_address = address;
        ESTABLISHED_CONTACT_STORAGE.insert(contact)
    }

    #[test]
    fn should_return_the_shielded_address_of_an_established_contact() {
        let handle = established(Some([0x5a; 43]));
        let mut has_address = false;
        let mut address = [0u8; 43];

        let result = unsafe {
            established_contact_get_shielded_address(handle, &mut has_address, address.as_mut_ptr())
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(has_address);
        assert_eq!(address, [0x5a; 43]);
        unsafe { established_contact_destroy(handle) };
    }

    #[test]
    fn should_return_no_shielded_address_for_a_contact_without_one() {
        let handle = established(None);
        let mut has_address = true;
        let mut address = [0xffu8; 43];

        let result = unsafe {
            established_contact_get_shielded_address(handle, &mut has_address, address.as_mut_ptr())
        };

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert!(!has_address);
        assert_eq!(address, [0u8; 43]);
        unsafe { established_contact_destroy(handle) };
    }
}
