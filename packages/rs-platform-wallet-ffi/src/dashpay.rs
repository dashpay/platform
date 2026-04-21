//! FFI bindings for DashPay contact-request lifecycle + payments on
//! the platform-wallet [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Replaces the local-state-only
//! `managed_identity_{send,accept,reject}_contact_request` FFI
//! family with Platform-broadcasting equivalents. Those local
//! helpers still exist for in-memory manipulation (e.g. tests,
//! initial bootstrap), but iOS flows should now drive from here so
//! documents actually hit Platform and `ManagedIdentity` state +
//! persister changesets stay consistent with on-chain reality.
//!
//! Entry points:
//!
//! - [`platform_wallet_send_contact_request`] — submit a contact
//!   request document. Returns a handle into
//!   `CONTACT_REQUEST_STORAGE` pointing at the freshly-created
//!   [`ContactRequest`](platform_wallet::ContactRequest).
//! - [`platform_wallet_sync_contact_requests`] — fetch all
//!   received contact requests for every managed identity. Returns
//!   an array of handles via `platform_wallet_contact_request_handle_array_free`.
//! - [`platform_wallet_accept_contact_request`] — reciprocate an
//!   incoming request, returning a handle into
//!   `ESTABLISHED_CONTACT_STORAGE`.
//! - [`platform_wallet_reject_contact_request`] — drop an incoming
//!   request locally (on-chain contactInfo tombstone is a future
//!   follow-up).
//! - [`platform_wallet_fetch_sent_contact_requests`] — query
//!   Platform for the identity's sent requests.
//! - [`platform_wallet_send_payment`] — send a Dash payment to an
//!   established contact via their `DashpayExternalAccount`. Runs
//!   the Core transaction path internally.

use std::ffi::CStr;
use std::os::raw::c_char;

use platform_wallet::ContactRequest;

use crate::contact_request::CONTACT_REQUEST_STORAGE;
use crate::error::*;
use crate::established_contact::ESTABLISHED_CONTACT_STORAGE;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;

// ---------------------------------------------------------------------------
// Array helpers
// ---------------------------------------------------------------------------

/// Heap-allocated array of `Handle` values. Returned by
/// [`platform_wallet_sync_contact_requests`] /
/// [`platform_wallet_fetch_sent_contact_requests`]; released via
/// [`platform_wallet_contact_request_handle_array_free`].
#[repr(C)]
pub struct ContactRequestHandleArray {
    pub handles: *mut Handle,
    pub count: usize,
}

impl ContactRequestHandleArray {
    /// Construct an empty array (null pointer, zero count). Used as
    /// the default / failure return.
    pub fn empty() -> Self {
        Self {
            handles: std::ptr::null_mut(),
            count: 0,
        }
    }

    /// Copy a slice of contact requests into the global handle
    /// storage and return a fresh array of handles.
    fn from_requests(requests: Vec<ContactRequest>) -> Self {
        if requests.is_empty() {
            return Self::empty();
        }
        let mut handles: Vec<Handle> = Vec::with_capacity(requests.len());
        for req in requests {
            handles.push(CONTACT_REQUEST_STORAGE.insert(req));
        }
        let count = handles.len();
        let boxed = handles.into_boxed_slice();
        let ptr = Box::into_raw(boxed) as *mut Handle;
        Self {
            handles: ptr,
            count,
        }
    }
}

/// Release an array previously returned by
/// [`platform_wallet_sync_contact_requests`] or
/// [`platform_wallet_fetch_sent_contact_requests`]. Does NOT free
/// the individual `ContactRequest` handles — they stay valid in
/// `CONTACT_REQUEST_STORAGE` so the caller can continue reading
/// fields off them via the existing `contact_request_get_*` FFI.
/// Use [`crate::contact_request_destroy`] to release a handle when
/// done with it.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_contact_request_handle_array_free(
    array: ContactRequestHandleArray,
) {
    if array.handles.is_null() || array.count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(array.handles, array.count) };
    let _ = unsafe { Box::from_raw(slice as *mut [Handle]) };
}

// ---------------------------------------------------------------------------
// Send contact request
// ---------------------------------------------------------------------------

/// Send a contact request to `recipient_id`.
///
/// All the usual parameters (key indices, account index, ECDH
/// inputs) are resolved internally by the Rust side from the
/// sender's local `ManagedIdentity`. Optional:
/// - `account_label`: a NUL-terminated UTF-8 string, or null. The
///   label is encrypted by the SDK.
/// - `auto_accept_proof` / `auto_accept_proof_len`: optional byte
///   slice; pass `(null, 0)` to omit.
///
/// Returns a handle into `CONTACT_REQUEST_STORAGE` via
/// `out_request_handle`. Release via
/// [`crate::contact_request_destroy`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_send_contact_request(
    wallet_handle: Handle,
    sender_identity_id: IdentifierBytes,
    recipient_identity_id: IdentifierBytes,
    account_label: *const c_char,
    auto_accept_proof: *const u8,
    auto_accept_proof_len: usize,
    out_request_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_request_handle.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_request_handle is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let sender = match sender_identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid sender identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let recipient = match recipient_identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid recipient identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    let label = if account_label.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(account_label) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            "account_label is not valid UTF-8",
                        );
                    }
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };
    let proof: Option<Vec<u8>> = if auto_accept_proof.is_null() || auto_accept_proof_len == 0 {
        None
    } else {
        Some(
            unsafe { std::slice::from_raw_parts(auto_accept_proof, auto_accept_proof_len) }
                .to_vec(),
        )
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move {
                identity
                    .send_contact_request(&sender, &recipient, label, proof)
                    .await
            });
            match result {
                Ok(request) => {
                    let handle = CONTACT_REQUEST_STORAGE.insert(request);
                    unsafe { *out_request_handle = handle };
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("send_contact_request failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

// ---------------------------------------------------------------------------
// Sync received contact requests from Platform
// ---------------------------------------------------------------------------

/// Fetch and apply received contact requests for every managed
/// identity on the wallet. Returns an array of handles pointing at
/// the newly-discovered incoming requests (the ones that weren't
/// already in local state). Release the array via
/// [`platform_wallet_contact_request_handle_array_free`]; release
/// each handle via [`crate::contact_request_destroy`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_contact_requests(
    wallet_handle: Handle,
    out_array: *mut ContactRequestHandleArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_array is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move { identity.sync_contact_requests().await });
            match result {
                Ok(list) => {
                    let array = ContactRequestHandleArray::from_requests(list);
                    unsafe { *out_array = array };
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    unsafe { *out_array = ContactRequestHandleArray::empty() };
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("sync_contact_requests failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

// ---------------------------------------------------------------------------
// Accept contact request
// ---------------------------------------------------------------------------

/// Accept an incoming contact request by sending a reciprocal
/// request back to the sender. Returns a handle into
/// `ESTABLISHED_CONTACT_STORAGE` pointing at the newly-established
/// contact. Release via [`crate::established_contact_destroy`].
///
/// `request_handle` must be a live handle from
/// `CONTACT_REQUEST_STORAGE` (typically obtained via
/// `managed_identity_get_incoming_contact_request` or
/// [`platform_wallet_sync_contact_requests`]).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_accept_contact_request(
    wallet_handle: Handle,
    request_handle: Handle,
    out_established_handle: *mut Handle,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_established_handle.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_established_handle is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let request = match CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone()) {
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

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result =
                block_on_worker(async move { identity.accept_contact_request(&request).await });
            match result {
                Ok(contact) => {
                    let handle = ESTABLISHED_CONTACT_STORAGE.insert(contact);
                    unsafe { *out_established_handle = handle };
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("accept_contact_request failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

// ---------------------------------------------------------------------------
// Reject contact request
// ---------------------------------------------------------------------------

/// Reject an incoming contact request. Drops the request from
/// `incoming_contact_requests` on the managed identity. A future
/// follow-up (noted on the Rust side) will also write a
/// `display_hidden` contactInfo document to Platform so the
/// rejection persists across devices; today the effect is local
/// only.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_reject_contact_request(
    wallet_handle: Handle,
    our_identity_id: IdentifierBytes,
    contact_identity_id: IdentifierBytes,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let our_id = match our_identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let contact_id = match contact_identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid contact identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move {
                identity.reject_contact_request(&our_id, &contact_id).await
            });
            match result {
                Ok(()) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("reject_contact_request failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

// ---------------------------------------------------------------------------
// Fetch sent contact requests (query-side)
// ---------------------------------------------------------------------------

/// Query Platform for contact requests sent by `identity_id`.
/// Returns handles into `CONTACT_REQUEST_STORAGE`. Release the
/// array via [`platform_wallet_contact_request_handle_array_free`]
/// and each handle via [`crate::contact_request_destroy`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fetch_sent_contact_requests(
    wallet_handle: Handle,
    identity_id: IdentifierBytes,
    out_array: *mut ContactRequestHandleArray,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_array.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_array is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }
    let id = match identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move { identity.sent_contact_requests(&id).await });
            match result {
                Ok(list) => {
                    let array = ContactRequestHandleArray::from_requests(list);
                    unsafe { *out_array = array };
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    unsafe { *out_array = ContactRequestHandleArray::empty() };
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("fetch_sent_contact_requests failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}

// ---------------------------------------------------------------------------
// Send payment
// ---------------------------------------------------------------------------

/// Send a Dash payment from `from_identity_id` to
/// `to_contact_identity_id` (must be an established DashPay
/// contact). `amount_duffs` is the payment amount in duffs
/// (1 DASH = 100,000,000 duffs).
///
/// On success, `out_txid` is populated with the 32-byte
/// transaction id of the broadcast Core transaction. The Rust
/// side also records a [`PaymentEntry`] on
/// [`ManagedIdentity`](platform_wallet::ManagedIdentity)
/// via the persister, so the Swift persister observes the
/// update through the identity changeset callback.
///
/// Memo field is optional; pass `memo = null` to omit.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_send_dashpay_payment(
    wallet_handle: Handle,
    from_identity_id: IdentifierBytes,
    to_contact_identity_id: IdentifierBytes,
    amount_duffs: u64,
    memo: *const c_char,
    out_txid: *mut [u8; 32],
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    if out_txid.is_null() {
        if !out_error.is_null() {
            unsafe {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorNullPointer,
                    "out_txid is null",
                );
            }
        }
        return PlatformWalletFFIResult::ErrorNullPointer;
    }

    let from_id = match from_identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid from_identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let to_id = match to_contact_identity_id.to_identifier() {
        Ok(i) => i,
        Err(e) => {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidIdentifier,
                        format!("Invalid to_identity identifier: {e}"),
                    );
                }
            }
            return PlatformWalletFFIResult::ErrorInvalidIdentifier;
        }
    };
    let memo_str: Option<String> = if memo.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(memo) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => {
                if !out_error.is_null() {
                    unsafe {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorUtf8Conversion,
                            "memo is not valid UTF-8",
                        );
                    }
                }
                return PlatformWalletFFIResult::ErrorUtf8Conversion;
            }
        }
    };
    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let identity = wallet.identity().clone();
            let result = block_on_worker(async move {
                identity
                    .send_payment(&from_id, &to_id, amount_duffs, memo_str)
                    .await
            });
            match result {
                Ok((txid, _entry)) => {
                    // `send_payment` returns `(Txid, PaymentEntry)`.
                    // The `PaymentEntry` is already recorded on the
                    // sender's `ManagedIdentity` via the persister, so
                    // it'll flow to Swift through the identity
                    // changeset callback. Here we just surface the
                    // `Txid` — copy the 32-byte little-endian
                    // representation into the out-param.
                    use dashcore::hashes::Hash;
                    let bytes = txid.to_raw_hash().to_byte_array();
                    unsafe {
                        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_txid.cast::<u8>(), 32);
                    }
                    PlatformWalletFFIResult::Success
                }
                Err(e) => {
                    if !out_error.is_null() {
                        unsafe {
                            *out_error = PlatformWalletFFIError::new(
                                PlatformWalletFFIResult::ErrorWalletOperation,
                                format!("send_dashpay_payment failed: {e}"),
                            );
                        }
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                unsafe {
                    *out_error = PlatformWalletFFIError::new(
                        PlatformWalletFFIResult::ErrorInvalidHandle,
                        "Invalid platform-wallet handle",
                    );
                }
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}
