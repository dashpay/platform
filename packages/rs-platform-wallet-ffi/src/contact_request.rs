//! Contact request FFI functions
//!
//! Provides access to individual contact request fields

use crate::error::*;
use crate::handle::*;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::ContactRequest;

// Storage for contact requests
lazy_static::lazy_static! {
    pub static ref CONTACT_REQUEST_STORAGE: HandleStorage<ContactRequest> = HandleStorage::new();
}

/// Create a new contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_create(
    sender_id: *const u8,
    recipient_id: *const u8,
    sender_key_index: u32,
    recipient_key_index: u32,
    account_reference: u32,
    encrypted_public_key_bytes: *const std::os::raw::c_uchar,
    encrypted_public_key_len: usize,
    core_height_created_at: u32,
    created_at: u64,
    out_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(encrypted_public_key_bytes);
    check_ptr!(out_handle);

    let sender = unwrap_result_or_return!(unsafe { read_identifier(sender_id) });
    let recipient = unwrap_result_or_return!(unsafe { read_identifier(recipient_id) });

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

    PlatformWalletFFIResult::ok()
}

/// Create a contact request handle from a managed identity's sent request
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_sent_contact_request(
    identity_handle: Handle,
    recipient_id: *const u8,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_request_handle);

    let id = unwrap_result_or_return!(unsafe { read_identifier(recipient_id) });

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity.dashpay().sent_contact_requests().get(&id).cloned()
    });
    let option = unwrap_option_or_return!(option);
    let request = unwrap_option_or_return!(option);

    unsafe { *out_request_handle = CONTACT_REQUEST_STORAGE.insert(request) };
    PlatformWalletFFIResult::ok()
}

/// Create a contact request handle from a managed identity's incoming request
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_incoming_contact_request(
    identity_handle: Handle,
    sender_id: *const u8,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_request_handle);

    let id = unwrap_result_or_return!(unsafe { read_identifier(sender_id) });

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .incoming_contact_requests()
            .get(&id)
            .cloned()
    });
    let inner = unwrap_option_or_return!(option);
    let request = unwrap_option_or_return!(inner);
    unsafe { *out_request_handle = CONTACT_REQUEST_STORAGE.insert(request) };
    PlatformWalletFFIResult::ok()
}

/// Get sender ID from contact request into a 32-byte out-buffer.
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_sender_id(
    request_handle: Handle,
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_id);

    let option = CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.sender_id);
    let id = unwrap_option_or_return!(option);
    unsafe { write_identifier(out_id, &id) };
    PlatformWalletFFIResult::ok()
}

/// Get recipient ID from contact request into a 32-byte out-buffer.
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_recipient_id(
    request_handle: Handle,
    out_id: *mut u8,
) -> PlatformWalletFFIResult {
    check_ptr!(out_id);

    let option = CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.recipient_id);
    let id = unwrap_option_or_return!(option);
    unsafe { write_identifier(out_id, &id) };
    PlatformWalletFFIResult::ok()
}

/// Get sender key index from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_sender_key_index(
    request_handle: Handle,
    out_index: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_index);

    let option =
        CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.sender_key_index);
    *out_index = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get recipient key index from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_recipient_key_index(
    request_handle: Handle,
    out_index: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_index);

    let option =
        CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.recipient_key_index);
    *out_index = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get account reference from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_account_reference(
    request_handle: Handle,
    out_account_ref: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_account_ref);

    let option =
        CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.account_reference);
    *out_account_ref = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get encrypted public key from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_encrypted_public_key(
    request_handle: Handle,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_bytes);
    check_ptr!(out_len);
    // Sentinel first: the handle lookup below is fallible, and
    // `platform_wallet_bytes_free` reconstructs a `Vec` from any non-null
    // pointer / non-zero length pair — a cleanup-on-error caller must never
    // see stack garbage here.
    unsafe {
        *out_bytes = std::ptr::null_mut();
        *out_len = 0;
    }

    let option = CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| {
        request.encrypted_public_key.clone()
    });
    let bytes = unwrap_option_or_return!(option).into_boxed_slice();
    let len = bytes.len();
    let ptr = Box::into_raw(bytes) as *mut u8;
    unsafe {
        *out_bytes = ptr;
        *out_len = len;
    }
    PlatformWalletFFIResult::ok()
}

/// Get creation timestamp from contact request
#[no_mangle]
pub unsafe extern "C" fn contact_request_get_created_at(
    request_handle: Handle,
    out_timestamp: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_timestamp);

    let option = CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.created_at);
    *out_timestamp = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Destroy contact request handle
#[no_mangle]
pub extern "C" fn contact_request_destroy(request_handle: Handle) -> PlatformWalletFFIResult {
    if CONTACT_REQUEST_STORAGE.remove(request_handle).is_some() {
        PlatformWalletFFIResult::ok()
    } else {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "Invalid contact request handle",
        )
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

            // Test sender ID
            let mut out_id = [0u8; 32];
            let result = contact_request_get_sender_id(handle, out_id.as_mut_ptr());
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(out_id, [1u8; 32]);

            // Test recipient ID
            let result = contact_request_get_recipient_id(handle, out_id.as_mut_ptr());
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(out_id, [2u8; 32]);

            // Test sender key index
            let mut sender_key_idx = 0u32;
            let result = contact_request_get_sender_key_index(handle, &mut sender_key_idx);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(sender_key_idx, 0);

            // Test recipient key index
            let mut recipient_key_idx = 0u32;
            let result = contact_request_get_recipient_key_index(handle, &mut recipient_key_idx);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(recipient_key_idx, 1);

            // Test account reference
            let mut account_ref = 0u32;
            let result = contact_request_get_account_reference(handle, &mut account_ref);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(account_ref, 42);

            // Test created_at
            let mut created_at = 0u64;
            let result = contact_request_get_created_at(handle, &mut created_at);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(created_at, 1_700_000_000);

            // Test encrypted public key
            let mut bytes_ptr: *mut u8 = std::ptr::null_mut();
            let mut len: usize = 0;
            let result = contact_request_get_encrypted_public_key(handle, &mut bytes_ptr, &mut len);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
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
