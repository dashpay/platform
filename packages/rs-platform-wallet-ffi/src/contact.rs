use crate::contact_request::CONTACT_REQUEST_STORAGE;
use crate::error::*;
use crate::handle::*;
use crate::identity_manager::ffi_noop_persister;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Get all sent contact request IDs
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_sent_contact_request_ids(
    identity_handle: Handle,
    out_array: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    // Sentinel first: the handle lookup below is fallible, and
    // `platform_wallet_identifier_array_free` reconstructs a `Vec` from any
    // non-null pointer/count pair — see `IdentifierArray::empty`.
    unsafe { *out_array = IdentifierArray::empty() };

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .sent_contact_requests()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    });
    let ids = unwrap_option_or_return!(option);
    unsafe { *out_array = IdentifierArray::new(ids) };
    PlatformWalletFFIResult::ok()
}

/// Get all incoming contact request IDs
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_incoming_contact_request_ids(
    identity_handle: Handle,
    out_array: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    // Sentinel first — see `IdentifierArray::empty`.
    unsafe { *out_array = IdentifierArray::empty() };

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .incoming_contact_requests()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    });
    let ids = unwrap_option_or_return!(option);
    unsafe { *out_array = IdentifierArray::new(ids) };
    PlatformWalletFFIResult::ok()
}

/// Get all established contact IDs
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_established_contact_ids(
    identity_handle: Handle,
    out_array: *mut IdentifierArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    // Sentinel first — see `IdentifierArray::empty`.
    unsafe { *out_array = IdentifierArray::empty() };

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .established_contacts()
            .keys()
            .cloned()
            .collect::<Vec<_>>()
    });
    let ids = unwrap_option_or_return!(option);
    unsafe { *out_array = IdentifierArray::new(ids) };
    PlatformWalletFFIResult::ok()
}

/// Check if a contact is established. `contact_id` is a `*const u8`
/// to a 32-byte identifier buffer.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_is_contact_established(
    identity_handle: Handle,
    contact_id: *const u8,
    out_is_established: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_is_established);

    let id = unwrap_result_or_return!(unsafe { read_identifier(contact_id) });

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity.dashpay().established_contacts().contains_key(&id)
    });
    *out_is_established = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Send a contact request from this identity to another
/// The request will be added to sent_contact_requests
/// If there's already an incoming request from the recipient, the contact will be automatically established
#[no_mangle]
pub unsafe extern "C" fn managed_identity_send_contact_request(
    identity_handle: Handle,
    request_handle: Handle,
) -> PlatformWalletFFIResult {
    let request_result = CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone());

    let request = unwrap_option_or_return!(request_result);

    let option = MANAGED_IDENTITY_STORAGE.with_item_mut(identity_handle, |identity| {
        // Return the persist result so a failure surfaces through the FFI
        // result instead of being swallowed — correct for any persister on this
        // handle path (today the infallible `ffi_noop_persister`).
        identity.add_sent_contact_request(request, &ffi_noop_persister())
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}

/// Accept an incoming contact request
/// This will add the request to incoming_contact_requests
/// If there's already a sent request to the sender, the contact will be automatically established
#[no_mangle]
pub unsafe extern "C" fn managed_identity_accept_contact_request(
    identity_handle: Handle,
    request_handle: Handle,
) -> PlatformWalletFFIResult {
    let request_result = CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone());

    let request = unwrap_option_or_return!(request_result);

    let option = MANAGED_IDENTITY_STORAGE.with_item_mut(identity_handle, |identity| {
        // Return the persist result so a failure surfaces through the FFI
        // result instead of being swallowed — correct for any persister on this
        // handle path (today the infallible `ffi_noop_persister`).
        identity.add_incoming_contact_request(request, &ffi_noop_persister())
    });
    unwrap_result_or_return!(unwrap_option_or_return!(option));
    PlatformWalletFFIResult::ok()
}

/// Ignore a contact sender (per-sender mute, = block, reversible).
///
/// Local in-memory path on a managed-identity handle (no persister) —
/// drops the sender's pending incoming request and records them in
/// `ignored_senders`. The durable, persisted path is the wallet-scoped
/// `platform_wallet_ignore_contact_sender`.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_ignore_contact_sender(
    identity_handle: Handle,
    sender_id: *const u8,
) -> PlatformWalletFFIResult {
    let id = unwrap_result_or_return!(unsafe { read_identifier(sender_id) });

    let option = MANAGED_IDENTITY_STORAGE.with_item_mut(identity_handle, |identity| {
        // `ignore_sender` returns a `ContactChangeSet`, not a `Result` — there is
        // no error to surface. This handle has no persister, so the changeset is
        // intentionally dropped; the durable `platform_wallet_ignore_contact_sender`
        // path persists it.
        drop(identity.ignore_sender(&id));
    });
    unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
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
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut array = IdentifierArray {
                items: std::ptr::null_mut(),
                count: 0,
            };

            let result = managed_identity_get_sent_contact_request_ids(handle, &mut array);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(array.count, 0); // Should be empty for new identity

            // Cleanup
            platform_wallet_identifier_array_free(&mut array);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_incoming_contact_request_ids() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut array = IdentifierArray {
                items: std::ptr::null_mut(),
                count: 0,
            };

            let result = managed_identity_get_incoming_contact_request_ids(handle, &mut array);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(array.count, 0);

            // Cleanup
            platform_wallet_identifier_array_free(&mut array);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_get_established_contact_ids() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let mut array = IdentifierArray {
                items: std::ptr::null_mut(),
                count: 0,
            };

            let result = managed_identity_get_established_contact_ids(handle, &mut array);
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert_eq!(array.count, 0);

            // Cleanup
            platform_wallet_identifier_array_free(&mut array);
            crate::managed_identity_destroy(handle);
        }
    }

    #[test]
    fn test_is_contact_established() {
        unsafe {
            let identity = create_test_identity();
            let managed = platform_wallet::ManagedIdentity::new(identity, 0);
            let handle = MANAGED_IDENTITY_STORAGE.insert(managed);

            let contact_id = Identifier::random();
            let id_bytes: [u8; 32] = contact_id.to_buffer();

            let mut is_established = true;
            let result = managed_identity_is_contact_established(
                handle,
                id_bytes.as_ptr(),
                &mut is_established,
            );
            assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
            assert!(!is_established);

            // Cleanup
            crate::managed_identity_destroy(handle);
        }
    }
}
