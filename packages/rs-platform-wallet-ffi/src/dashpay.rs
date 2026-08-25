//! FFI bindings for DashPay contact-request lifecycle + payments on
//! the platform-wallet [`IdentityWallet`](platform_wallet::IdentityWallet).
//!
//! Replaces the local-state-only
//! `managed_identity_{send,accept}_contact_request` FFI
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
//! - [`platform_wallet_ignore_contact_sender`] /
//!   [`platform_wallet_unignore_contact_sender`] — ignore (per-sender
//!   mute, = block, reversible) / un-ignore a sender. Local-only; no
//!   on-chain artifact.
//! - [`platform_wallet_fetch_sent_contact_requests`] — query
//!   Platform for the identity's sent requests.
//! - [`platform_wallet_send_payment`] — send a Dash payment to an
//!   established contact via their `DashpayExternalAccount`. Runs
//!   the Core transaction path internally.

use std::ffi::CStr;
use std::os::raw::c_char;

use platform_wallet::ContactRequest;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

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

/// Outcome of the layered [`platform_wallet_get_managed_identity`] lookup,
/// split out so the dual-error decision (dashpay/platform#4060, findings
/// `03ff842bd7d7` / `d7c06333de19`) is unit-testable without a fully-seeded
/// `PlatformWallet` fixture.
enum ManagedIdentityOutcome<T> {
    /// A present, live wallet manages the identity.
    Found(T),
    /// The `wallet_handle` no longer resolves to a live, managed wallet:
    /// either absent from `PLATFORM_WALLET_STORAGE` (the outer lookup miss) OR
    /// present as a handle but already removed from the shared `WalletManager`
    /// map (a `get_wallet_info` miss — a stale handle racing `removeWallet`,
    /// which clears the manager entry *before* destroying the handle). Both are
    /// real wallet failures and must surface as `ErrorInvalidHandle`, never be
    /// swallowed as "identity not managed".
    InvalidHandle,
    /// A valid, live wallet that simply does not manage the identity — the only
    /// outcome Kotlin's `translateManagedIdentityNotFoundToZero` turns into a
    /// zero handle.
    NotManaged,
}

/// Classify the nested lookup `Option` layers into the FFI result contract:
///  - outer  = `wallet_handle` presence in `PLATFORM_WALLET_STORAGE`,
///  - middle = `get_wallet_info` presence in the shared `WalletManager` map,
///  - inner  = `managed_identity` presence on that wallet.
///
/// The two distinct `None`-producing failures (handle absent, and wallet
/// removed from the manager map) must NOT collapse into `NotManaged` — see
/// [`ManagedIdentityOutcome`]. Pure and generic so both error arms are directly
/// unit-testable.
fn classify_managed_identity_outcome<T>(
    outcome: Option<Option<Option<T>>>,
) -> ManagedIdentityOutcome<T> {
    match outcome {
        None | Some(None) => ManagedIdentityOutcome::InvalidHandle,
        Some(Some(None)) => ManagedIdentityOutcome::NotManaged,
        Some(Some(Some(managed))) => ManagedIdentityOutcome::Found(managed),
    }
}

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

    // THREE distinct outcomes must stay distinct — they must NOT collapse into
    // one `NotFound` (dashpay/platform#4060, finding 03ff842bd7d7). Using `?`
    // inside the closure previously folded the `get_wallet_info` miss into the
    // same closure-`None` as the `managed_identity` miss, so a wallet removed
    // from the manager map (but not yet handle-destroyed) reported as "identity
    // not managed". `.map()` keeps the `get_wallet_info` presence in its own
    // layer instead:
    //   outer `None`           -> handle absent from PLATFORM_WALLET_STORAGE
    //   `Some(None)`           -> handle live, wallet REMOVED from the manager map
    //                             (a stale handle racing `removeWallet`)
    //   `Some(Some(None))`     -> valid live wallet, identity not managed
    //   `Some(Some(Some(mi)))` -> found
    // Kotlin's `translateManagedIdentityNotFoundToZero` (Dashpay.kt) turns ONLY
    // `NotFound` into a zero handle, so the two handle-invalid cases must surface
    // as `ErrorInvalidHandle` and never masquerade as an empty (zero-balance)
    // unmanaged identity. See [`classify_managed_identity_outcome`].
    let outcome = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wm = wallet.wallet_manager().blocking_read();
        wm.get_wallet_info(&wallet.wallet_id())
            .map(|info| info.identity_manager.managed_identity(&id).cloned())
    });
    let managed = match classify_managed_identity_outcome(outcome) {
        ManagedIdentityOutcome::Found(managed) => managed,
        ManagedIdentityOutcome::InvalidHandle => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidHandle,
                format!("platform wallet handle {wallet_handle} is not backed by a live wallet"),
            );
        }
        ManagedIdentityOutcome::NotManaged => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::NotFound,
                format!("wallet {wallet_handle} does not manage the requested identity"),
            );
        }
    };
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
    // Publish an FFI-safe sentinel before any fallible work so every early
    // return leaves `*out_array` well-defined — a caller that runs symmetric
    // cleanup-on-error then feeds an empty (null, 0) array into
    // `platform_wallet_contact_request_handle_array_free`, never stale stack
    // bytes.
    unsafe { *out_array = ContactRequestHandleArray::empty() };

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.dashpay().sync_contact_requests().await })
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
/// `core_signer_handle` is the wallet-HD resolver signer (the same handle the
/// drain takes): the Rust side derives the friendship xpub, the ECDH shared
/// secret, and the DIP-15 `accountReference` through it, so no resident seed is
/// needed and watch-only / external-signable wallets work. Caller retains
/// ownership of both handles for the duration of the call.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`.
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
    core_signer_handle: *mut MnemonicResolverHandle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_request_handle);
    check_ptr!(signer_handle);
    check_ptr!(core_signer_handle);

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
        // Defense-in-depth: bound the copy at the DIP-15 auto-accept proof
        // ceiling (102 bytes) BEFORE allocating, so a malformed binding or a
        // hostile (ptr, len) pair can't force an oversized allocation. The SDK
        // re-validates the exact 38..=102 range after this.
        if auto_accept_proof_len > 102 {
            return crate::error::PlatformWalletFFIResult::err(
                crate::error::PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "auto_accept_proof length {auto_accept_proof_len} exceeds the DIP-15 maximum of 102 bytes"
                ),
            );
        }
        Some(std::slice::from_raw_parts(auto_accept_proof, auto_accept_proof_len).to_vec())
    };

    let signer_addr = signer_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the drain FFI — the caller pins
        // both handles for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity
                .dashpay()
                .send_contact_request_with_external_signer(
                    &sender,
                    &recipient,
                    label,
                    platform_wallet::AutoAcceptProofSource::from_option(proof),
                    signer,
                    &provider,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let request = unwrap_result_or_return!(result);
    *out_request_handle = CONTACT_REQUEST_STORAGE.insert(request);
    PlatformWalletFFIResult::ok()
}

/// Send a contact request from a scanned DIP-15 auto-accept QR
/// (`dash:?du=<username>&dapk=<key_blob>`). Resolves the QR's username to the
/// owner's identity, decodes the handed auto-accept key, signs the proof over
/// this send's `accountReference`, and broadcasts — so the owner can auto-accept
/// it. Inserts the resulting request at `*out_request_handle`.
///
/// # Safety
/// - `sender_identity_id` must point to 32 readable bytes.
/// - `uri` must be a valid NUL-terminated UTF-8 C string.
/// - `signer_handle` / `core_signer_handle` must be valid, non-destroyed handles
///   (the caller pins both for the duration of this call).
/// - `out_request_handle` must be a valid `*mut Handle`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_send_contact_request_from_qr(
    wallet_handle: Handle,
    sender_identity_id: *const u8,
    uri: *const c_char,
    signer_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(uri);
    check_ptr!(signer_handle);
    check_ptr!(core_signer_handle);
    check_ptr!(out_request_handle);

    let sender = unwrap_result_or_return!(read_identifier(sender_identity_id));
    let uri = unwrap_result_or_return!(CStr::from_ptr(uri).to_str()).to_string();
    let signer_addr = signer_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the send FFI — the caller pins both
        // handles for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity
                .dashpay()
                .send_contact_request_from_qr(&sender, &uri, signer, &provider)
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
/// [`platform_wallet_sync_contact_requests`]). `core_signer_handle` is the
/// wallet-HD resolver signer (as for
/// [`platform_wallet_send_contact_request_with_signer`]): the reciprocal send
/// and the external-account registration source all key material through it, so
/// no resident seed is needed.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_accept_contact_request_with_signer(
    wallet_handle: Handle,
    request_handle: Handle,
    signer_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_established_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_established_handle);
    check_ptr!(signer_handle);
    check_ptr!(core_signer_handle);

    let request_option = CONTACT_REQUEST_STORAGE.with_item(request_handle, |req| req.clone());
    let request = unwrap_option_or_return!(request_option);

    let signer_addr = signer_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the drain FFI — the caller pins
        // both handles for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move {
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity
                .dashpay()
                .accept_contact_request_with_external_signer(&request, signer, &provider)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let contact = unwrap_result_or_return!(result);
    *out_established_handle = ESTABLISHED_CONTACT_STORAGE.insert(contact);
    PlatformWalletFFIResult::ok()
}

// ---------------------------------------------------------------------------
// Ignore / un-ignore a contact sender (per-sender mute, local-only)
// ---------------------------------------------------------------------------

/// Ignore a contact sender (per-sender mute, = block, reversible).
///
/// Drops the sender's pending incoming request and records the sender as
/// ignored so the recurring sync sweep suppresses ALL of their requests
/// (including rotated ones) from the main pending list. Ignore is
/// **local-only** — no on-chain artifact (syncing it would leak who you
/// ignored); it is persisted through the changeset → SwiftData pipeline so
/// it survives a relaunch. Reverse with
/// [`platform_wallet_unignore_contact_sender`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_ignore_contact_sender(
    wallet_handle: Handle,
    our_identity_id: *const u8,
    contact_identity_id: *const u8,
) -> PlatformWalletFFIResult {
    let our_id = unwrap_result_or_return!(unsafe { read_identifier(our_identity_id) });
    let contact_id = unwrap_result_or_return!(unsafe { read_identifier(contact_identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            identity
                .dashpay()
                .ignore_contact_sender(&our_id, &contact_id)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Un-ignore a contact sender (reverse
/// [`platform_wallet_ignore_contact_sender`]).
///
/// Removes the sender from the ignore set AND rewinds the received
/// high-water cursor so the next sweep re-fetches the sender's on-chain
/// requests (otherwise the cursor has already passed them and they'd never
/// reappear). A no-op (returns OK) when the sender wasn't ignored.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_unignore_contact_sender(
    wallet_handle: Handle,
    our_identity_id: *const u8,
    contact_identity_id: *const u8,
) -> PlatformWalletFFIResult {
    let our_id = unwrap_result_or_return!(unsafe { read_identifier(our_identity_id) });
    let contact_id = unwrap_result_or_return!(unsafe { read_identifier(contact_identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            identity
                .dashpay()
                .unignore_contact_sender(&our_id, &contact_id)
                .await
        })
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
    // Sentinel first: identifier parsing and the gRPC query below are both
    // fallible, so publish an empty array before them to keep every early
    // return FFI-safe for a cleanup-on-error caller.
    unsafe { *out_array = ContactRequestHandleArray::empty() };
    let id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.dashpay().sent_contact_requests(&id).await })
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
///
/// The funding inputs are signed through the supplied
/// [`MnemonicResolverHandle`] — the same vtable shape used by
/// [`crate::core_wallet::core_wallet_send_to_addresses`] — which the FFI
/// wraps in a [`MnemonicResolverCoreSigner`] for the lifetime of this call.
/// The wallet seed is never made resident; every signature is produced
/// inside the signer's atomic derive-and-sign step.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`. Ownership is retained by the caller —
///   this function does NOT destroy it.
/// - `out_fee_duffs` may be NULL to ignore the fee; when non-null it must
///   point to valid writable `u64` storage, written only on success.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_send_dashpay_payment(
    wallet_handle: Handle,
    from_identity_id: *const u8,
    to_contact_identity_id: *const u8,
    amount_duffs: u64,
    memo: *const c_char,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_txid: *mut [u8; 32],
    out_fee_duffs: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_txid);

    let from_id = unwrap_result_or_return!(unsafe { read_identifier(from_identity_id) });
    let to_id = unwrap_result_or_return!(unsafe { read_identifier(to_contact_identity_id) });
    let memo_str: Option<String> = if memo.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(unsafe { CStr::from_ptr(memo) }.to_str()).to_string())
    };

    let signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: `signer_addr` came from `core_signer_handle`, which the
        // caller pinned alive for the duration of this call (see fn-level
        // safety doc). `MnemonicResolverCoreSigner` stores the handle as a
        // `usize` and is `Send + Sync`, so it can move into the worker task;
        // it is dropped when that task completes, before this call returns.
        let signer = unsafe {
            MnemonicResolverCoreSigner::new(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        // Same resolver handle, wrapped as a `ContactCryptoProvider`, so
        // `send_payment` can drain a deferred external-account build for this
        // contact before resolving the account. Same lifetime contract as the
        // signer above (no new FFI surface).
        let provider = unsafe {
            resolver_contact_crypto_provider(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move {
            identity
                .dashpay()
                .send_payment(&from_id, &to_id, amount_duffs, memo_str, &signer, &provider)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let (txid, _entry, fee_duffs) = unwrap_result_or_return!(result);
    // Exact network fee of the broadcast transaction — Σ(selected input
    // values) − Σ(output values), computed by the transaction builder
    // itself since rust-dashcore#872, so a sub-dust change remainder
    // folded into the fee is reflected here. Nullable so callers that
    // don't care can pass NULL.
    if !out_fee_duffs.is_null() {
        unsafe { *out_fee_duffs = fee_duffs };
    }
    use dashcore::hashes::Hash;
    let bytes = txid.to_raw_hash().to_byte_array();
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_txid.cast::<u8>(), 32);
    }
    PlatformWalletFFIResult::ok()
}

/// Glue adapter implementing platform-wallet's [`ContactCryptoProvider`] over
/// the resolver-backed [`MnemonicResolverCoreSigner`]. The orphan rule needs
/// the impl's type local to this crate, so the signer is wrapped here rather
/// than implemented directly on it in `rs-sdk-ffi`. Serves the deferred-crypto
/// drain, the live send/accept flow, AND contactInfo publish.
pub(crate) struct ResolverContactCryptoProvider {
    signer: MnemonicResolverCoreSigner,
}

/// Wrap a caller-owned resolver handle as a [`ContactCryptoProvider`] for a
/// `(wallet_id, network)` — the single construction the contact-crypto FFI
/// entry points (send / accept / drain / contactInfo) share.
///
/// # Safety
/// `core_signer_handle` must be a valid, non-destroyed `*mut MnemonicResolverHandle`;
/// the caller pins it for the duration of the provider's use.
pub(crate) unsafe fn resolver_contact_crypto_provider(
    core_signer_handle: *mut MnemonicResolverHandle,
    wallet_id: [u8; 32],
    network: key_wallet::Network,
) -> ResolverContactCryptoProvider {
    ResolverContactCryptoProvider {
        signer: MnemonicResolverCoreSigner::new(core_signer_handle, wallet_id, network),
    }
}

#[async_trait::async_trait]
impl platform_wallet::ContactCryptoProvider for ResolverContactCryptoProvider {
    async fn receiving_xpub(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<key_wallet::bip32::ExtendedPubKey, platform_wallet::PlatformWalletError> {
        use key_wallet::signer::ExtendedPubKeySigner;
        self.signer
            .extended_public_key(path)
            .await
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn ecdh_shared_secret(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        peer: &dashcore::secp256k1::PublicKey,
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, platform_wallet::PlatformWalletError> {
        self.signer
            .ecdh_shared_secret(path, peer)
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn export_auto_accept_private_key(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<dashcore::secp256k1::SecretKey, platform_wallet::PlatformWalletError> {
        let scalar = self
            .signer
            .export_auto_accept_private_key(path)
            .map_err(|e| {
                platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string())
            })?;
        dashcore::secp256k1::SecretKey::from_slice(scalar.as_ref())
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn export_invitation_private_key(
        &self,
        path: &key_wallet::bip32::DerivationPath,
    ) -> Result<dashcore::secp256k1::SecretKey, platform_wallet::PlatformWalletError> {
        let scalar = self
            .signer
            .export_invitation_private_key(path)
            .map_err(|e| {
                platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string())
            })?;
        dashcore::secp256k1::SecretKey::from_slice(scalar.as_ref())
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn account_reference(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        compact_xpub: &[u8],
        account_index: u32,
        version: u32,
    ) -> Result<u32, platform_wallet::PlatformWalletError> {
        self.signer
            .account_reference(path, compact_xpub, account_index, version)
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn unmask_account_reference(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        compact_xpub: &[u8],
        account_reference: u32,
    ) -> Result<(u32, u32), platform_wallet::PlatformWalletError> {
        self.signer
            .unmask_account_reference(path, compact_xpub, account_reference)
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn contact_info_seal(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        contact_id: &[u8; 32],
        private_data_plaintext: &[u8],
        private_data_iv: &[u8; 16],
    ) -> Result<platform_wallet::ContactInfoSealed, platform_wallet::PlatformWalletError> {
        let sealed = self
            .signer
            .contact_info_seal(
                root_path,
                derivation_index,
                contact_id,
                private_data_plaintext,
                private_data_iv,
            )
            .map_err(|e| {
                platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string())
            })?;
        Ok(platform_wallet::ContactInfoSealed {
            enc_to_user_id: sealed.enc_to_user_id,
            private_data: sealed.private_data,
        })
    }

    async fn contact_info_open(
        &self,
        root_path: &key_wallet::bip32::DerivationPath,
        derivation_index: u32,
        enc_to_user_id: &[u8; 32],
        private_data_blob: &[u8],
    ) -> Result<platform_wallet::ContactInfoOpened, platform_wallet::PlatformWalletError> {
        let opened = self
            .signer
            .contact_info_open(
                root_path,
                derivation_index,
                enc_to_user_id,
                private_data_blob,
            )
            .map_err(|e| {
                platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string())
            })?;
        Ok(platform_wallet::ContactInfoOpened {
            contact_id: opened.contact_id,
            private_data: opened.private_data,
        })
    }
}

/// Drain the persisted deferred-crypto queue using the Keychain signer for the
/// key material. Call when a signer is available (Keychain unlock, or any
/// signer-present DashPay action). Runs the provider-only ops (account build /
/// contactInfo decrypt) AND the DIP-15 auto-accept pass (which needs the
/// identity `signer_handle` to send the reciprocal). Writes the total number of
/// completed entries (drained + auto-accepted) to `out_drained`.
///
/// # Seed binding
///
/// Whenever there is drainable work, the resolver behind `core_signer_handle`
/// is first checked against this wallet's persisted BIP44 account-0 xpub
/// (`PlatformWallet::drain_pending_contact_crypto_verified`, the same gate the
/// startup sequence drains through). A resolver mapped to a different wallet
/// fails the call with `ErrorInvalidParameter` and derives NOTHING — the queue
/// is left intact for the next correct-seed drain. This is not advisory: a
/// wrong-seed drain writes contact receiving accounts that no later
/// correct-seed pass revisits (`register_contact_account` keys its existence
/// check on the contact pair, not on the xpub), so the corruption would be
/// permanent and its only symptom payments that never arrive. Any other
/// verification failure — a resolver that simply cannot answer — fails closed
/// the same way, with `ErrorWalletOperation`.
///
/// An empty queue skips the check entirely, so a poll with nothing to do still
/// costs no key material.
///
/// # Safety
/// - `signer_handle` (the identity document signer) is **optional**: pass null to
///   run only the provider-derived ops (account build / contactInfo decrypt) and
///   skip the auto-accept pass; pass a valid, non-destroyed `*mut SignerHandle`
///   to also auto-accept proof-bearing inbound requests.
/// - `core_signer_handle` (the wallet-HD resolver) must be a valid, non-destroyed
///   handle. The caller retains ownership of both for the duration of this call.
/// - `out_drained` must be a valid `*mut u32`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_drain_pending_contact_crypto(
    wallet_handle: Handle,
    signer_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_drained: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_drained);
    // Zero-init before any fallible work so a refused drain leaves a truthful
    // count rather than whatever the caller's stack held — same discipline as
    // the cached seed-binding verify.
    unsafe { *out_drained = 0 };

    // The identity signer is optional — null means "provider-only drain".
    let signer_addr = if signer_handle.is_null() {
        0usize
    } else {
        signer_handle as usize
    };
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as platform_wallet_send_dashpay_payment —
        // the caller pins both handles for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        let wallet = wallet.clone();
        block_on_worker(async move {
            // The auto-accept pass needs the identity signer for the reciprocal;
            // `None` skips it, matching a null `signer_handle`.
            let signer: Option<&VTableSigner> = if signer_addr != 0 {
                Some(&*(signer_addr as *const VTableSigner))
            } else {
                None
            };
            // Unbounded, as this entry point has always been: it is called off
            // the main thread by a host that decided the work is worth waiting
            // for, not from the Core-SPV-gating startup path that owns a budget.
            wallet
                .drain_pending_contact_crypto_verified(&provider, signer, None)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(total) => {
            unsafe {
                *out_drained = total as u32;
            }
            PlatformWalletFFIResult::ok()
        }
        // Same code the standalone verify reports for a mis-mapped resolver, so
        // a host recognizes the wrong-seed condition identically whether it
        // checked up front or was refused at the drain.
        Err(e @ platform_wallet::PlatformWalletError::SeedMismatch { .. }) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e.to_string(),
            )
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            e.to_string(),
        ),
    }
}

/// Number of deferred **account-build** contact-crypto ops queued for this
/// wallet (the `RegisterReceiving` / `RegisterExternal` ops that build a
/// contact's payment account and need a signer unlock). Writes the count to
/// `out_count`.
///
/// Signerless read of in-memory state — no signer handle needed, safe to poll.
/// `> 0` means some contacts are waiting for an unlock to finish setup; it is a
/// wallet-scoped upper bound (aggregates the wallet's identities; may include
/// ops that resolve to channel-broken on the next drain). `ContactInfoDecrypt`
/// is excluded — it re-enqueues every sweep, so it is structurally always
/// present and is not an actionable backlog.
///
/// # Safety
/// - `out_count` must be a valid `*mut u32`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_pending_contact_crypto_count(
    wallet_handle: Handle,
    out_count: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_count);

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.dashpay().pending_contact_crypto_count().await })
    });
    let count = unwrap_option_or_return!(option);
    unsafe {
        *out_count = count as u32;
    }
    PlatformWalletFFIResult::ok()
}

/// Build a DIP-15 auto-accept QR URI (`dash:?du=<username>&dapk=<key_blob>`) for
/// the identity `owner_identity_id`, valid for 1 hour. The QR's `du` is the
/// owner's DPNS name (a scanner resolves it back to the owner's identity).
/// `username` is the locally-cached name when available; pass an **empty** C
/// string (or one resolving to empty) to resolve the name on-chain instead —
/// needed for imported/restored identities whose name isn't cached locally.
/// Writes a heap C string to `*out_uri`; the caller frees it with
/// `platform_wallet_string_free`.
///
/// Derives the wallet's auto-accept private key through the resolver (the one
/// deliberate raw-key export — the key is a bearer credential the QR shares) and
/// encodes the `dapk` blob + URI Rust-side.
///
/// # Safety
/// - `owner_identity_id` must point to 32 readable bytes.
/// - `username` must be a valid NUL-terminated UTF-8 C string.
/// - `core_signer_handle` must be a valid, non-destroyed `*mut MnemonicResolverHandle`
///   (the caller pins it for the duration of this call).
/// - `out_uri` must be a valid `*mut *mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_build_auto_accept_qr(
    wallet_handle: Handle,
    owner_identity_id: *const u8,
    username: *const c_char,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_uri: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(username);
    check_ptr!(core_signer_handle);
    check_ptr!(out_uri);
    // Zero-init the out-param before any fallible work so an early return
    // (bad id, bad UTF-8, missing wallet, async failure, interior-NUL URI)
    // leaves a safe null rather than uninitialized memory the caller might
    // free — same discipline as the profile getters.
    unsafe { *out_uri = std::ptr::null_mut() };

    // `read_identifier` null-checks the pointer (matches the sibling QR FFIs).
    let owner = unwrap_result_or_return!(read_identifier(owner_identity_id));
    let username = unwrap_result_or_return!(CStr::from_ptr(username).to_str()).to_string();
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the send/drain FFIs — the caller
        // pins the resolver handle for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move {
            identity
                .dashpay()
                .build_auto_accept_qr(&owner, &username, &provider)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let uri = unwrap_result_or_return!(result);
    let c_uri = match std::ffi::CString::new(uri) {
        Ok(c) => c,
        Err(_) => {
            return PlatformWalletFFIResult::from(
                "auto-accept URI contained an interior NUL".to_string(),
            )
        }
    };
    unsafe {
        *out_uri = c_uri.into_raw();
    }
    PlatformWalletFFIResult::ok()
}

/// Verify the resolver signer resolves the seed that owns this wallet, before
/// trusting it to sign. Derives the wallet's BIP44 account-0 xpub through the
/// signer and compares it to the persisted account xpub; a mismatch means the
/// signer is mapped to the wrong wallet and the call fails with
/// `ErrorInvalidParameter`. Run once at unlock (alongside the deferred-crypto
/// drain) so a mis-mapped Keychain slot can never sign for a wallet it does not
/// own — the wrong-seed detection without a resident seed.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`; ownership is retained by the caller.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_verify_seed_binds_to_wallet(
    wallet_handle: Handle,
    core_signer_handle: *mut MnemonicResolverHandle,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);

    let signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the drain FFI — the caller pins the
        // resolver handle for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        let wallet = wallet.clone();
        block_on_worker(async move { wallet.verify_seed_binds(&provider).await })
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok(()) => PlatformWalletFFIResult::ok(),
        Err(e @ platform_wallet::PlatformWalletError::SeedMismatch { .. }) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e.to_string(),
            )
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            e.to_string(),
        ),
    }
}

/// Marker-aware variant of [`platform_wallet_verify_seed_binds_to_wallet`].
/// The binding check is a pure function of (mnemonic, network) against the
/// wallet's persisted account-0 xpub, so one successful verify holds for
/// every later launch **as long as the mnemonic Keychain item is untouched**.
/// The host passes `keychain_stamp` — an opaque identity/generation stamp of
/// that item (e.g. its creation+modification dates) that changes on every
/// write to it — and Rust binds the marker to BOTH the xpub and the stamp.
/// A marker that still matches proves the previously verified item is the
/// one in the Keychain and skips the resolver entirely — no mnemonic fetch,
/// no derivation. A rewritten mnemonic item, a re-imported wallet, or a
/// missing stamp all fall through to the full resolver check.
///
/// Contract: `*out_marker` is set to a heap C string (free with
/// [`platform_wallet_string_free`](crate::platform_wallet_string_free)) ONLY
/// when the full check ran, bound, and a stamp was supplied — that is the
/// host's signal to persist the new marker. It stays null when the supplied
/// marker matched (nothing to persist) and when no stamp was supplied
/// (nothing safe to cache; the host keeps verifying every launch). A
/// mismatched derivation fails with `ErrorInvalidParameter` exactly like the
/// non-cached variant, leaving `*out_marker` null.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`; ownership is retained by the caller.
/// - `verified_marker` is **optional**: null means "no marker persisted yet"
///   (first launch); otherwise it must be a valid NUL-terminated UTF-8 C
///   string.
/// - `keychain_stamp` is **optional**: null means "stamp unavailable" and
///   disables the cache for this call; otherwise it must be a valid
///   NUL-terminated UTF-8 C string.
/// - `out_marker` must be a valid `*mut *mut c_char`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_verify_seed_binds_to_wallet_cached(
    wallet_handle: Handle,
    core_signer_handle: *mut MnemonicResolverHandle,
    verified_marker: *const c_char,
    keychain_stamp: *const c_char,
    out_marker: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_marker);
    // Zero-init the out-param before any fallible work so an early return
    // leaves a safe null rather than uninitialized memory the caller might
    // free — same discipline as the QR / profile getters.
    unsafe { *out_marker = std::ptr::null_mut() };

    let marker: Option<String> = if verified_marker.is_null() {
        None
    } else {
        Some(
            unwrap_result_or_return!(unsafe { CStr::from_ptr(verified_marker) }.to_str())
                .to_string(),
        )
    };
    let stamp: Option<String> = if keychain_stamp.is_null() {
        None
    } else {
        Some(
            unwrap_result_or_return!(unsafe { CStr::from_ptr(keychain_stamp) }.to_str())
                .to_string(),
        )
    };
    let signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as the drain FFI — the caller pins the
        // resolver handle for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        let wallet = wallet.clone();
        block_on_worker(async move {
            wallet
                .verify_seed_binds_with_marker(&provider, marker.as_deref(), stamp.as_deref())
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    match result {
        Ok((platform_wallet::SeedBindingVerification::MarkerMatched, _)) => {
            PlatformWalletFFIResult::ok()
        }
        Ok((platform_wallet::SeedBindingVerification::Verified, new_marker)) => {
            if let Some(new_marker) = new_marker {
                let c_marker = match std::ffi::CString::new(new_marker) {
                    Ok(c) => c,
                    Err(_) => {
                        return PlatformWalletFFIResult::from(
                            "seed-binding marker contained an interior NUL".to_string(),
                        )
                    }
                };
                unsafe {
                    *out_marker = c_marker.into_raw();
                }
            }
            PlatformWalletFFIResult::ok()
        }
        Err(e @ platform_wallet::PlatformWalletError::SeedMismatch { .. }) => {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                e.to_string(),
            )
        }
        Err(e) => PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            e.to_string(),
        ),
    }
}

/// Total number of queued contact-crypto entries for this wallet — every op
/// kind, **including** the `ContactInfoDecrypt` refreshes that
/// [`platform_wallet_pending_contact_crypto_count`] deliberately excludes.
/// This is the drain scheduler's probe: a signer-present drain should run
/// whenever anything at all is queued, or ContactInfoDecrypt-only queues
/// (cross-device contact metadata) would never be applied. Keep using the
/// filtered count for user-facing "waiting to finish setup" surfaces. Writes
/// the count to `out_count`. Signerless read; safe to poll.
///
/// # Safety
/// - `out_count` must be a valid `*mut u32`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_drainable_contact_crypto_count(
    wallet_handle: Handle,
    out_count: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(out_count);

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        block_on_worker(async move { identity.dashpay().drainable_contact_crypto_count().await })
    });
    let count = unwrap_option_or_return!(option);
    unsafe {
        *out_count = count as u32;
    }
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Marshalling-boundary coverage for the verify entry point, replacing what
    // the removed attach FFI's input-validation tests upheld. The crypto
    // semantics (matching seed binds, wrong seed rejected) are pinned library-
    // side in `platform_wallet::...::seed_binding`.

    /// A null `core_signer_handle` is rejected with `ErrorNullPointer` (the
    /// `check_ptr!` contract) before any wallet lookup.
    #[test]
    fn verify_seed_binds_null_signer_is_null_pointer() {
        let r = unsafe { platform_wallet_verify_seed_binds_to_wallet(1, std::ptr::null_mut()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// An unknown `wallet_handle` surfaces `NotFound` via the `with_item`
    /// lookup miss. The signer handle is never dereferenced (the wallet lookup
    /// fails first), so a non-null dummy pointer is safe here.
    #[test]
    fn verify_seed_binds_unknown_wallet_is_not_found() {
        let dummy_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let r = unsafe { platform_wallet_verify_seed_binds_to_wallet(0xDEAD_BEEF, dummy_signer) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
    }

    /// The cached variant rejects a null `core_signer_handle` with
    /// `ErrorNullPointer` before any wallet lookup — same `check_ptr!`
    /// contract as the non-cached entry point.
    #[test]
    fn verify_seed_binds_cached_null_signer_is_null_pointer() {
        let mut out_marker: *mut c_char = std::ptr::null_mut();
        let r = unsafe {
            platform_wallet_verify_seed_binds_to_wallet_cached(
                1,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                &mut out_marker,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert!(out_marker.is_null());
    }

    /// The cached variant rejects a null `out_marker` with `ErrorNullPointer`.
    #[test]
    fn verify_seed_binds_cached_null_out_marker_is_null_pointer() {
        let dummy_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let r = unsafe {
            platform_wallet_verify_seed_binds_to_wallet_cached(
                1,
                dummy_signer,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// An unknown `wallet_handle` surfaces `NotFound` via the `with_item`
    /// lookup miss, and `*out_marker` is left at the zero-initialized null.
    /// A null `verified_marker` is the legitimate "no marker yet" input, so
    /// this also covers that path's marshalling. The signer handle is never
    /// dereferenced (the wallet lookup fails first).
    #[test]
    fn verify_seed_binds_cached_unknown_wallet_is_not_found() {
        let dummy_signer = std::ptr::dangling_mut::<MnemonicResolverHandle>();
        let mut out_marker: *mut c_char = std::ptr::dangling_mut();
        let r = unsafe {
            platform_wallet_verify_seed_binds_to_wallet_cached(
                0xDEAD_BEEF,
                dummy_signer,
                std::ptr::null(),
                std::ptr::null(),
                &mut out_marker,
            )
        };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert!(
            out_marker.is_null(),
            "out_marker is zero-initialized before the lookup"
        );
    }

    /// A null `out_count` is rejected with `ErrorNullPointer` (the `check_ptr!`
    /// contract) before any wallet lookup.
    #[test]
    fn pending_contact_crypto_count_null_out_is_null_pointer() {
        let r = unsafe { platform_wallet_pending_contact_crypto_count(1, std::ptr::null_mut()) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// An unknown `wallet_handle` surfaces `NotFound` via the `with_item`
    /// lookup miss; `out_count` is left untouched.
    #[test]
    fn pending_contact_crypto_count_unknown_wallet_is_not_found() {
        let mut count: u32 = 7;
        let r = unsafe { platform_wallet_pending_contact_crypto_count(0xDEAD_BEEF, &mut count) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(count, 7, "out_count is untouched on a lookup miss");
    }

    /// An unknown `wallet_handle` surfaces `ErrorInvalidHandle` — the OUTER
    /// `with_item` miss — NOT `NotFound` (dashpay/platform#4060). This is the
    /// load-bearing half of the dual-error contract: the Kotlin `Dashpay` layer
    /// translates only `NotFound` (a valid wallet that does not manage the id)
    /// into a zero handle, so a stale/closed wallet MUST arrive as
    /// `ErrorInvalidHandle` to avoid masquerading as an unmanaged identity. The
    /// 32-byte `identity_id` is read before the wallet lookup (`read_identifier`),
    /// so a real buffer is supplied; `out_managed_identity_handle` must be left
    /// untouched on the miss.
    ///
    /// The complementary inner outcome (a valid wallet lacking the managed
    /// identity → `NotFound`) needs a fully seeded wallet in
    /// `PLATFORM_WALLET_STORAGE`, which this unit-test module has no fixture for;
    /// that path is covered at the translation layer by the Kotlin
    /// `ManagedIdentityNotFoundTranslationTest`.
    #[test]
    fn get_managed_identity_unknown_wallet_is_invalid_handle() {
        let id = [0u8; 32];
        let mut out: Handle = 0;
        let r = unsafe { platform_wallet_get_managed_identity(0xDEAD_BEEF, id.as_ptr(), &mut out) };
        assert_eq!(r.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(out, 0, "out handle is untouched on an invalid-handle miss");
    }

    /// The dual-error decision (dashpay/platform#4060, findings 03ff842bd7d7 &
    /// d7c06333de19), exercised at the `classify_managed_identity_outcome` seam
    /// so BOTH error codes are covered without a fully-seeded `PlatformWallet`
    /// fixture (which this unit-test module cannot build). This is exactly the
    /// layer where the two `None` outcomes previously collapsed:
    ///
    /// - Outer `None` (handle absent from `PLATFORM_WALLET_STORAGE`) AND
    ///   `Some(None)` (handle live but the wallet was REMOVED from the shared
    ///   `WalletManager` map — the removed-but-not-destroyed race the finding
    ///   describes) both classify as `InvalidHandle` → `ErrorInvalidHandle`, so
    ///   Kotlin's `translateManagedIdentityNotFoundToZero` never swallows a real
    ///   wallet failure as a zero-balance unmanaged identity.
    /// - `Some(Some(None))` (valid live wallet that does not manage the id)
    ///   classifies as `NotManaged` → `NotFound`, the only outcome Kotlin
    ///   translates to a zero handle.
    /// - `Some(Some(Some(_)))` classifies as `Found`.
    ///
    /// The full FFI path for the outer-`None` arm is additionally covered by
    /// `get_managed_identity_unknown_wallet_is_invalid_handle` above.
    #[test]
    fn classify_managed_identity_outcome_distinguishes_removed_wallet_from_unmanaged() {
        // Handle absent from storage → invalid handle.
        assert!(matches!(
            classify_managed_identity_outcome::<u32>(None),
            ManagedIdentityOutcome::InvalidHandle
        ));
        // Removed-but-not-destroyed: handle live, wallet gone from the manager
        // map (get_wallet_info miss). Must NOT be "not managed".
        assert!(matches!(
            classify_managed_identity_outcome::<u32>(Some(None)),
            ManagedIdentityOutcome::InvalidHandle
        ));
        // Valid live wallet that simply does not manage the identity.
        assert!(matches!(
            classify_managed_identity_outcome::<u32>(Some(Some(None))),
            ManagedIdentityOutcome::NotManaged
        ));
        // Found.
        assert!(matches!(
            classify_managed_identity_outcome(Some(Some(Some(7u32)))),
            ManagedIdentityOutcome::Found(7)
        ));
    }
}
