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
//! - [`platform_wallet_send_contact_request_with_signer`] — submit a
//!   contact request document, signing the document state-transition
//!   through the supplied `SignerHandle`. Returns a handle into
//!   `CONTACT_REQUEST_STORAGE` pointing at the freshly-created
//!   [`ContactRequest`](platform_wallet::ContactRequest).
//! - [`platform_wallet_sync_contact_requests`] — fetch all
//!   received contact requests for every managed identity. Returns
//!   an array of handles via `platform_wallet_contact_request_handle_array_free`.
//! - [`platform_wallet_accept_contact_request_with_signer`] —
//!   reciprocate an incoming request, returning a handle into
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
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::contact_request::CONTACT_REQUEST_STORAGE;
use crate::error::*;
use crate::established_contact::ESTABLISHED_CONTACT_STORAGE;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

// ---------------------------------------------------------------------------
// Managed identity lookup
// ---------------------------------------------------------------------------

/// Look up the live [`ManagedIdentity`](platform_wallet::ManagedIdentity)
/// for `identity_id` under `wallet_handle` and return a fresh handle
/// into the shared `MANAGED_IDENTITY_STORAGE`.
///
/// The returned handle is a *snapshot clone* — it doesn't track
/// further mutations on the Rust-side live identity. Call again
/// after each mutation (e.g. after a sync round) to pick up fresh
/// state. Release via [`crate::managed_identity_destroy`].
///
/// This wraps `WalletManager::get_wallet_info(...).identity_manager
/// .managed_identity(...)` so callers on the new
/// `ManagedPlatformWallet` / `PlatformWallet` path can read
/// `ManagedIdentity` fields (contact requests, established contacts,
/// DPNS names, etc.) without spinning up a separate
/// `IdentityManager` handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_managed_identity(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_managed_identity_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_managed_identity_handle);
    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        let info = wm.get_wallet_info(&wallet.wallet_id())?;
        info.identity_manager.managed_identity(&id).cloned()
    });
    let inner = unwrap_option_or_return!(option);
    let managed = unwrap_option_or_return!(inner);
    unsafe { *out_managed_identity_handle = MANAGED_IDENTITY_STORAGE.insert(managed) };
    PlatformWalletFFIResult::ok()
}

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
    /// Construct an empty array (null pointer, zero count).
    pub fn empty() -> Self {
        Self {
            handles: std::ptr::null_mut(),
            count: 0,
        }
    }

    /// Copy a slice of contact requests into the global handle storage.
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
///
/// Pointer-only signature: by-value `ContactRequestHandleArray`
/// (a 16-byte aggregate) sat at the AAPCS64 / Swift-ABI cliff.
/// Pass `&mut array`; on return the buffer is freed and the
/// fields are reset to a safe empty state so a double-free no-ops.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_contact_request_handle_array_free(
    array: *mut ContactRequestHandleArray,
) {
    if array.is_null() {
        return;
    }
    let array = unsafe { &mut *array };
    if array.handles.is_null() || array.count == 0 {
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts_mut(array.handles, array.count) };
    let _ = unsafe { Box::from_raw(slice as *mut [Handle]) };
    array.handles = std::ptr::null_mut();
    array.count = 0;
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
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.sync_contact_requests().await })
    });
    let result = unwrap_option_or_return!(option);
    let list = unwrap_result_or_return!(result);
    unsafe { *out_array = ContactRequestHandleArray::from_requests(list) };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Send / accept contact request — external-signer variants
// ---------------------------------------------------------------------------

/// Send a contact request to `recipient_id` using an
/// externally-supplied signer for the document state-transition.
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
/// [`crate::contact_request_destroy`]. `signer_handle` must be a
/// valid, non-destroyed handle produced by
/// `dash_sdk_signer_create_with_ctx`; caller retains ownership.
///
/// CAVEAT — ECDH derivation: the Rust side still derives the
/// sender's ECDH private key from the wallet seed for the contact
/// request encryption step. Watch-only wallets (no seed Rust-side)
/// will fail at that step. See the docstring on
/// [`IdentityWallet::send_contact_request_with_external_signer`](platform_wallet::IdentityWallet::send_contact_request_with_external_signer)
/// for the planned follow-up to push ECDH across the FFI as well.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_send_contact_request_with_signer(
    wallet_handle: Handle,
    sender_identity_id: *const u8,
    recipient_identity_id: *const u8,
    account_label: *const c_char,
    auto_accept_proof: *const u8,
    auto_accept_proof_len: usize,
    signer_handle: *mut SignerHandle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_request_handle);
    check_ptr!(signer_handle);

    let sender = unwrap_result_or_return!(read_identifier(sender_identity_id));
    let recipient = unwrap_result_or_return!(read_identifier(recipient_identity_id));
    let label = if account_label.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(CStr::from_ptr(account_label).to_str()).to_string())
    };
    let proof: Option<Vec<u8>> = if auto_accept_proof.is_null() || auto_accept_proof_len == 0 {
        None
    } else {
        Some(std::slice::from_raw_parts(auto_accept_proof, auto_accept_proof_len).to_vec())
    };

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity
                .send_contact_request_with_external_signer(
                    &sender, &recipient, label, proof, signer,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let request = unwrap_result_or_return!(result);
    *out_request_handle = CONTACT_REQUEST_STORAGE.insert(request);
    PlatformWalletFFIResult::ok()
}

/// Accept an incoming contact request using an externally-supplied
/// signer for the reciprocal request's document state-transition.
///
/// Sends a reciprocal request back to the sender via the supplied
/// `signer_handle` and returns a handle into
/// `ESTABLISHED_CONTACT_STORAGE` pointing at the newly-established
/// contact. Release via [`crate::established_contact_destroy`].
///
/// `request_handle` must be a live handle from
/// `CONTACT_REQUEST_STORAGE` (typically obtained via
/// `managed_identity_get_incoming_contact_request` or
/// [`platform_wallet_sync_contact_requests`]). Same ECDH caveat
/// applies as for [`platform_wallet_send_contact_request_with_signer`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_accept_contact_request_with_signer(
    wallet_handle: Handle,
    request_handle: Handle,
    signer_handle: *mut SignerHandle,
    out_established_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_established_handle);
    check_ptr!(signer_handle);

    let request_option = CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone());
    let request = unwrap_option_or_return!(request_option);

    let signer_addr = signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity
                .accept_contact_request_with_external_signer(&request, signer)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let contact = unwrap_result_or_return!(result);
    *out_established_handle = ESTABLISHED_CONTACT_STORAGE.insert(contact);
    PlatformWalletFFIResult::ok()
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
    our_identity_id: *const u8,
    contact_identity_id: *const u8,
) -> PlatformWalletFFIResult {
    let our_id = unwrap_result_or_return!(unsafe { read_identifier(our_identity_id) });
    let contact_id = unwrap_result_or_return!(unsafe { read_identifier(contact_identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.reject_contact_request(&our_id, &contact_id).await })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Fetch sent contact requests (query-side)
// ---------------------------------------------------------------------------

/// Query Platform for contact requests sent by `identity_id`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_fetch_sent_contact_requests(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_array: *mut ContactRequestHandleArray,
) -> PlatformWalletFFIResult {
    check_ptr!(out_array);
    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.sent_contact_requests(&id).await })
    });
    let result = unwrap_option_or_return!(option);
    let list = unwrap_result_or_return!(result);
    unsafe { *out_array = ContactRequestHandleArray::from_requests(list) };
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Send payment
// ---------------------------------------------------------------------------

/// Send a Dash payment from `from_identity_id` to `to_contact_identity_id`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_send_dashpay_payment(
    wallet_handle: Handle,
    from_identity_id: *const u8,
    to_contact_identity_id: *const u8,
    amount_duffs: u64,
    memo: *const c_char,
    out_txid: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(out_txid);

    let from_id = unwrap_result_or_return!(unsafe { read_identifier(from_identity_id) });
    let to_id = unwrap_result_or_return!(unsafe { read_identifier(to_contact_identity_id) });
    let memo_str: Option<String> = if memo.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(unsafe { CStr::from_ptr(memo) }.to_str()).to_string())
    };
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            identity
                .send_payment(&from_id, &to_id, amount_duffs, memo_str)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let (txid, _entry) = unwrap_result_or_return!(result);
    use dashcore::hashes::Hash;
    let bytes = txid.to_raw_hash().to_byte_array();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_txid.cast::<u8>(), 32);
    }
    PlatformWalletFFIResult::ok()
}
