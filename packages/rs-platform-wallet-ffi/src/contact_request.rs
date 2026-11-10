//! Contact request FFI functions
//!
//! Provides access to individual contact request fields

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use platform_wallet::{ContactRequest, PrivateContactReadiness};

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

/// What a contact request from one of a wallet's identities waits for, as the
/// wallet decides it (`platform_wallet::PrivateContactReadiness`), for a host
/// to show rather than infer from the refusals it got.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrivateContactReadinessFFI {
    /// Whether the wallet's private payments are restored for the identity.
    /// While false, its contact requests and acceptances, public or private,
    /// are refused with `ErrorPrivateContactPaymentsNotRestored` (72) until
    /// the shielded store is configured or the wallet's shielded account
    /// bound, or a contact request's own retry of a failed restoration
    /// succeeds; no private sync lifts it. A build without the `shielded`
    /// feature never does.
    pub private_payments_restored: bool,
    /// Whether a private sync read DashPay's private contact request store to
    /// its end for the identity with its current keys. Every contact request
    /// and acceptance reads it again from the cursor before it is decided, and
    /// is refused with `ErrorPrivateContactStoreNotRead` (74) while that read
    /// cannot finish.
    pub private_store_read: bool,
    /// Whether a private contact request the identity paid for may still be
    /// unresolved: the marker its private scan state stores, or a payment the
    /// shielded store's journal keeps unresolved. A journal record discharged
    /// since is applied before the read.
    pub unresolved_private_payment: bool,
    /// Whether the last restoration of the wallet's private payments failed,
    /// while `private_payments_restored` is false for it: then 72 holds until
    /// a restoration succeeds, not only until one runs. The reason is
    /// `platform_wallet_get_private_payments_restore_failure`.
    pub private_payments_restore_failed: bool,
}

impl From<PrivateContactReadiness> for PrivateContactReadinessFFI {
    fn from(readiness: PrivateContactReadiness) -> Self {
        Self {
            private_payments_restored: readiness.private_payments_restored,
            private_store_read: readiness.private_store_read,
            unresolved_private_payment: readiness.unresolved_private_payment,
            private_payments_restore_failed: readiness.private_payments_restore_failure.is_some(),
        }
    }
}

/// What a contact request from `identity_id`, one of the wallet's identities,
/// waits for, as the wallet decides it, into `out_readiness`
/// ([`PrivateContactReadinessFFI`]): so a host tells "restoring private
/// payments" from "reading the private store" and from a contact's pending
/// request by the wallet's own state. Not a pure read: a journal record
/// discharged since (a payment proven never to execute) is applied first,
/// briefly taking the wallet's write lock, as a send would see it; then it
/// reads live wallet state. Why a failed restoration failed is
/// `platform_wallet_get_private_payments_restore_failure`; one pair's hold is
/// `platform_wallet_get_private_pair_held`.
///
/// `out_readiness` is reset to all false first. An unknown `wallet_handle` is
/// `NotFound`; an identity the wallet does not hold is refused as its lookup
/// is.
///
/// # Safety
/// - `identity_id` must point to 32 readable bytes.
/// - `out_readiness` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_private_contact_readiness(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_readiness: *mut PrivateContactReadinessFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_readiness);
    unsafe { *out_readiness = PrivateContactReadinessFFI::default() };
    let identity_id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            identity
                .dashpay()
                .private_contact_readiness(&identity_id)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let readiness = unwrap_result_or_return!(result);
    unsafe { *out_readiness = readiness.into() };
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each readiness flag reaches its own FFI field, alone: a conversion
    /// that swapped or cleared one would show here.
    #[test]
    fn should_convert_each_readiness_flag_to_its_own_field() {
        let restored = PrivateContactReadinessFFI::from(PrivateContactReadiness {
            private_payments_restored: true,
            private_store_read: false,
            unresolved_private_payment: false,
            private_payments_restore_failure: None,
        });
        assert_eq!(
            restored,
            PrivateContactReadinessFFI {
                private_payments_restored: true,
                private_store_read: false,
                unresolved_private_payment: false,
                private_payments_restore_failed: false,
            }
        );
        let read = PrivateContactReadinessFFI::from(PrivateContactReadiness {
            private_payments_restored: false,
            private_store_read: true,
            unresolved_private_payment: false,
            private_payments_restore_failure: None,
        });
        assert_eq!(
            read,
            PrivateContactReadinessFFI {
                private_payments_restored: false,
                private_store_read: true,
                unresolved_private_payment: false,
                private_payments_restore_failed: false,
            }
        );
        let unresolved = PrivateContactReadinessFFI::from(PrivateContactReadiness {
            private_payments_restored: false,
            private_store_read: false,
            unresolved_private_payment: true,
            private_payments_restore_failure: None,
        });
        assert_eq!(
            unresolved,
            PrivateContactReadinessFFI {
                private_payments_restored: false,
                private_store_read: false,
                unresolved_private_payment: true,
                private_payments_restore_failed: false,
            }
        );
        let failed = PrivateContactReadinessFFI::from(PrivateContactReadiness {
            private_payments_restored: false,
            private_store_read: false,
            unresolved_private_payment: false,
            private_payments_restore_failure: Some("the journal does not read".to_string()),
        });
        assert_eq!(
            failed,
            PrivateContactReadinessFFI {
                private_payments_restored: false,
                private_store_read: false,
                unresolved_private_payment: false,
                private_payments_restore_failed: true,
            }
        );
    }
    use dpp::prelude::Identifier;

    /// The readiness of a fresh wallet's identity: nothing held, the store not
    /// read yet, no private payment; then the identity's own state, once a
    /// private sync read the store and once it was loaded with a private
    /// payment that may be unresolved. A null out pointer, an unknown wallet
    /// and an identity the wallet does not hold are refused, with the out
    /// value reset.
    #[test]
    fn should_read_a_wallet_identity_s_private_contact_readiness() {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;

        assert_eq!(
            unsafe {
                platform_wallet_get_private_contact_readiness(
                    1,
                    [0u8; 32].as_ptr(),
                    std::ptr::null_mut(),
                )
            }
            .code,
            PlatformWalletFFIResultCode::ErrorNullPointer
        );
        let mut readiness = PrivateContactReadinessFFI {
            private_payments_restored: true,
            private_store_read: true,
            unresolved_private_payment: true,
            private_payments_restore_failed: false,
        };
        let unknown = unsafe {
            platform_wallet_get_private_contact_readiness(
                Handle::MAX,
                [0u8; 32].as_ptr(),
                &mut readiness,
            )
        };
        assert_eq!(unknown.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(readiness, PrivateContactReadinessFFI::default(), "reset");

        let manager = crate::test_fixtures::mock_manager();
        let wallet = crate::runtime::runtime()
            .block_on(manager.create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[9; 64],
                WalletAccountCreationOptions::Default,
                Some(0),
            ))
            .expect("a wallet");
        let identity_id = Identifier::from([0xB2; 32]);
        let key = dpp::secp256k1::PublicKey::from_secret_key(
            &dpp::secp256k1::SecretKey::from_secret_bytes([4; 32]).expect("a secret key"),
        );
        let identity = Identity::V0(IdentityV0 {
            id: identity_id,
            public_keys: [(
                1,
                IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: 1,
                    purpose: Purpose::ENCRYPTION,
                    security_level: SecurityLevel::MEDIUM,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(key.serialize().to_vec()),
                    disabled_at: None,
                }),
            )]
            .into_iter()
            .collect(),
            balance: 0,
            revision: 0,
        });
        crate::runtime::runtime().block_on(async {
            let mut wm = wallet.wallet_manager().write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .expect("the wallet's info")
                .identity_manager
                .add_identity(identity, 0, wallet.wallet_id(), wallet.persister())
                .expect("the identity");
        });
        let handle = PLATFORM_WALLET_STORAGE.insert(wallet);

        let read = unsafe {
            platform_wallet_get_private_contact_readiness(
                handle,
                identity_id.to_buffer().as_ptr(),
                &mut readiness,
            )
        };
        assert_eq!(read.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(
            readiness,
            PrivateContactReadinessFFI {
                private_payments_restored: true,
                private_store_read: false,
                unresolved_private_payment: false,
                private_payments_restore_failed: false,
            }
        );
        // The identity's own state, not a fresh identity's: once a private
        // sync read the store to its end, and once loaded with a private
        // payment that may be unresolved.
        let readiness_now = |readiness: &mut PrivateContactReadinessFFI| {
            let read = unsafe {
                platform_wallet_get_private_contact_readiness(
                    handle,
                    identity_id.to_buffer().as_ptr(),
                    readiness,
                )
            };
            assert_eq!(read.code, PlatformWalletFFIResultCode::Success);
        };
        let with_identity =
            |change: &dyn Fn(&mut platform_wallet::wallet::identity::ManagedIdentity)| {
                PLATFORM_WALLET_STORAGE
                    .with_item(handle, |wallet| {
                        crate::runtime::runtime().block_on(async {
                            let mut wm = wallet.wallet_manager().write().await;
                            change(
                                wm.get_wallet_info_mut(&wallet.wallet_id())
                                    .expect("the wallet's info")
                                    .identity_manager
                                    .managed_identity_mut(&identity_id)
                                    .expect("the identity"),
                            );
                        })
                    })
                    .expect("the wallet");
            };
        with_identity(&|managed| managed.note_private_contact_requests_read_for_tests());
        readiness_now(&mut readiness);
        assert_eq!(
            readiness,
            PrivateContactReadinessFFI {
                private_payments_restored: true,
                private_store_read: true,
                unresolved_private_payment: false,
                private_payments_restore_failed: false,
            }
        );
        with_identity(&|managed| {
            managed.apply_private_contact_requests_position(
                platform_wallet::changeset::PrivateContactRequestsPosition {
                    position: 0,
                    key_fingerprint: [0; 32],
                    private_payments_unresolved: true,
                },
            )
        });
        readiness_now(&mut readiness);
        assert_eq!(
            readiness,
            PrivateContactReadinessFFI {
                private_payments_restored: false,
                private_store_read: true,
                unresolved_private_payment: true,
                private_payments_restore_failed: false,
            }
        );
        let stranger = unsafe {
            platform_wallet_get_private_contact_readiness(
                handle,
                [0xEE; 32].as_ptr(),
                &mut readiness,
            )
        };
        assert_ne!(stranger.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(readiness, PrivateContactReadinessFFI::default(), "reset");
        PLATFORM_WALLET_STORAGE.remove(handle);
    }

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
