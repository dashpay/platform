//! FFI bindings for DashPay private contact requests (protocol version 15).
//!
//! A private contact request is an entry of DashPay's `privateContactRequest`
//! store, sealed to one of the recipient's DashPay keys and paid for from the
//! shielded pool, so nothing on chain names either side. The wallet keeps a
//! private request as it keeps a public one, an ordinary
//! [`ContactRequest`](platform_wallet::ContactRequest), so the existing
//! contact-request rows and accessors read it.
//!
//! - [`platform_wallet_sync_private_contact_requests`]: read the store for the
//!   wallet's identities and ingest what it holds for them. The store is
//!   opened with keys that stay in the resolver-backed signer; each request's
//!   signature is checked against its sender's keys, which needs no secret.
//! - [`platform_wallet_manager_send_private_contact_request`] /
//!   [`platform_wallet_manager_accept_private_contact_request`]: send, or
//!   accept with a private reply, paying from an account's shielded notes.
//!   The request is signed by the identity signer the public send and accept
//!   take, and sealed in the resolver-backed signer. Built with the
//!   `shielded` feature only.
//! - [`platform_wallet_manager_accept_contact_request_replying_in_kind`]:
//!   the one accept a host calls. The wallet replies privately to a request
//!   that came privately, and with a document otherwise. Exported in every
//!   build; without the `shielded` feature a private reply fails with
//!   `ErrorShieldedBuildRequired`.
//! - [`managed_identity_get_private_contact`]: what a contact's private
//!   requests carry beyond the `ContactRequest`: whether they came or went
//!   privately, and the contact's shielded address.
//! - [`platform_wallet_get_private_pair_held`]: whether a contact request
//!   from one of the wallet's identities to a contact is held for a private
//!   one, as a send decides it, so a host offers no second send or reply.
//! - [`platform_wallet_get_public_contact_request_unconfirmed`]: whether a
//!   public contact request from one of the wallet's identities to a contact
//!   was broadcast and its outcome is not proved yet, which holds the pair
//!   as a private request pending does.
//! - [`platform_wallet_get_private_payments_restore_failure`]: why the last
//!   restoration of the wallet's private payments failed, which
//!   `platform_wallet_get_private_contact_readiness` reports only as a flag.
//!
//! The explicitly public send and accept refuse a contact whose requests came
//! or went privately with `ErrorPrivateContactRequestNeedsPrivateReply`.

use std::ffi::CString;
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

use platform_wallet::{PlatformWalletError, PrivateContact};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::contact_request::CONTACT_REQUEST_STORAGE;
use crate::dashpay::{resolver_contact_crypto_provider, ContactRequestHandleArray};
use crate::error::*;
use crate::established_contact::ESTABLISHED_CONTACT_STORAGE;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

#[cfg(not(feature = "shielded"))]
use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;

#[cfg(feature = "shielded")]
use std::ffi::CStr;
#[cfg(feature = "shielded")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "shielded")]
use platform_wallet::wallet::shielded::{CachedOrchardProver, NetworkShieldedCoordinator};
#[cfg(feature = "shielded")]
use platform_wallet::{AutoAcceptProofSource, PrivateReplyPayment};
#[cfg(feature = "shielded")]
use zeroize::Zeroizing;

#[cfg(feature = "shielded")]
use crate::dashpay::read_auto_accept_proof;
#[cfg(feature = "shielded")]
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
#[cfg(feature = "shielded")]
use crate::shielded_send::{
    catch_spend_panic, map_spend_result, poke_sync_on_unconfirmed, resolve_shielded_coordinator,
    resolve_wallet_and_coordinator,
};

/// The bytes of an Orchard payment address: an 11-byte diversifier and a
/// 32-byte transmission key.
const SHIELDED_ADDRESS_LEN: usize = 43;

/// A private sync, and the store read a send or accept makes first, check
/// that the resolver's seed is the wallet's before reading anything. A
/// mismatch takes the code every DashPay export gives it, and the seed
/// check reports (`ErrorInvalidParameter`); any other error is handed back.
fn seed_mismatch_result(
    error: PlatformWalletError,
) -> Result<PlatformWalletFFIResult, PlatformWalletError> {
    match error {
        error @ PlatformWalletError::SeedMismatch { .. } => Ok(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            error.to_string(),
        )),
        error => Err(error),
    }
}

/// What a pass over DashPay's private contact request store did
/// (`PrivateContactSyncReport`). Counts above `u32::MAX` saturate.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PrivateContactSyncReportFFI {
    /// The store position the pass stopped at.
    pub next_position: u64,
    /// The store pages read.
    pub pages_read: u32,
    /// Entries that opened for one of the wallet's identities but were
    /// refused.
    pub rejected_count: u32,
    /// Identities whose ingest did not reach disk: their entries are read
    /// again by the next pass.
    pub unpersisted_identity_count: u32,
    /// Whether the pass read to the store's last entry. When false, call
    /// again to read on.
    pub complete: bool,
}

// Expected `PrivateContactSyncReportFFI` layout on 64-bit targets:
//
//   0..=7     next_position               u64
//   8..=11    pages_read                  u32
//   12..=15   rejected_count              u32
//   16..=19   unpersisted_identity_count  u32
//   20        complete                    bool
//   21..=23   (tail padding to 8)
//
// Total size = 24, alignment = 8 (from the u64).
const _: [u8; 24] = [0u8; std::mem::size_of::<PrivateContactSyncReportFFI>()];
const _: [u8; 8] = [0u8; std::mem::align_of::<PrivateContactSyncReportFFI>()];
const _: () = assert!(std::mem::offset_of!(PrivateContactSyncReportFFI, pages_read) == 8);
const _: () = assert!(std::mem::offset_of!(PrivateContactSyncReportFFI, rejected_count) == 12);
const _: () =
    assert!(std::mem::offset_of!(PrivateContactSyncReportFFI, unpersisted_identity_count) == 16);
const _: () = assert!(std::mem::offset_of!(PrivateContactSyncReportFFI, complete) == 20);

fn saturating_count(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Read DashPay's private contact request store for every identity of the
/// wallet, at most `max_pages` pages from the earliest of their cursors, and
/// ingest what it holds for them: requests to them, each sender checked, and
/// requests they sent, recovered from the store alone. Contacts establish and
/// their DashPay accounts are queued as the document sync does.
///
/// `core_signer_handle` is the wallet-HD resolver signer: the store is opened
/// with keys derived inside it. Each request's signature is checked against
/// its sender's keys, which needs no secret. Each identity's cursor is stored
/// with the requests read before it and restored at load when the persister
/// attests `PLATFORM_WALLET_PERSISTENCE_CAPABILITY_PRIVATE_CONTACT_REQUEST_SCAN_STATE`
/// with atomic changesets; otherwise the first pass of a process reads the
/// store from its start.
///
/// Writes the requests the pass ingested, received and sent (including ones
/// already held), to `out_requests` (release with
/// `platform_wallet_contact_request_handle_array_free`, each handle with
/// `contact_request_destroy`) and the pass's counters to `out_report`.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`; the caller pins it for the call.
/// - `out_requests` and `out_report` must be valid, writable pointers.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_sync_private_contact_requests(
    wallet_handle: Handle,
    core_signer_handle: *mut MnemonicResolverHandle,
    max_pages: u32,
    out_requests: *mut ContactRequestHandleArray,
    out_report: *mut PrivateContactSyncReportFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_requests);
    check_ptr!(out_report);
    // Publish FFI-safe sentinels before any fallible work, so every early
    // return leaves both out-params well-defined.
    unsafe {
        *out_requests = ContactRequestHandleArray::empty();
        *out_report = PrivateContactSyncReportFFI::default();
    }

    // Look the wallet up under the registry guard, but run the pass outside
    // it: a pass reads many pages, and a registry read guard held across
    // that wait would stall `platform_wallet_destroy` and, through
    // parking_lot's writer preference, every other registry reader.
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        (
            wallet.identity().clone(),
            wallet.wallet_id(),
            wallet.network(),
        )
    });
    let (identity, wallet_id, network) = unwrap_option_or_return!(option);
    // SAFETY: the caller pins the resolver handle for the duration of this
    // call.
    let provider =
        unsafe { resolver_contact_crypto_provider(core_signer_handle, wallet_id, network) };
    let result = block_on_worker(async move {
        identity
            .dashpay()
            .sync_private_contact_requests(&provider, max_pages as usize)
            .await
    });
    let report = match result {
        Ok(report) => report,
        Err(error) => return seed_mismatch_result(error).unwrap_or_else(|error| error.into()),
    };
    unsafe {
        *out_report = PrivateContactSyncReportFFI {
            next_position: report.next_position,
            pages_read: saturating_count(report.pages_read),
            rejected_count: saturating_count(report.rejected.len()),
            unpersisted_identity_count: saturating_count(report.unpersisted_identities.len()),
            complete: report.complete,
        };
        *out_requests = ContactRequestHandleArray::from_requests(report.requests);
    }
    PlatformWalletFFIResult::ok()
}

/// What an identity's private contact requests with one contact carry beyond
/// the `ContactRequest` (`PrivateContact`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateContactFFI {
    /// The contact's request came privately: accepting it replies privately.
    pub received_privately: bool,
    /// This identity's request to the contact went privately.
    pub sent_privately: bool,
    /// Whether `shielded_address` holds the contact's address.
    pub has_shielded_address: bool,
    /// The Orchard address the contact asked to be paid at privately, when
    /// `has_shielded_address`; zeros otherwise.
    pub shielded_address: [u8; 43],
}

// Expected `PrivateContactFFI` layout on all targets:
//
//   0         received_privately    bool
//   1         sent_privately        bool
//   2         has_shielded_address  bool
//   3..=45    shielded_address      [u8; 43]
//
// Total size = 46, alignment = 1 (all-byte fields).
const _: [u8; 46] = [0u8; std::mem::size_of::<PrivateContactFFI>()];
const _: [u8; 1] = [0u8; std::mem::align_of::<PrivateContactFFI>()];
const _: () = assert!(std::mem::offset_of!(PrivateContactFFI, sent_privately) == 1);
const _: () = assert!(std::mem::offset_of!(PrivateContactFFI, has_shielded_address) == 2);
const _: () = assert!(std::mem::offset_of!(PrivateContactFFI, shielded_address) == 3);

impl PrivateContactFFI {
    fn empty() -> Self {
        Self {
            received_privately: false,
            sent_privately: false,
            has_shielded_address: false,
            shielded_address: [0; SHIELDED_ADDRESS_LEN],
        }
    }
}

impl From<PrivateContact> for PrivateContactFFI {
    fn from(contact: PrivateContact) -> Self {
        Self {
            received_privately: contact.received_privately,
            sent_privately: contact.sent_privately,
            has_shielded_address: contact.shielded_address.is_some(),
            shielded_address: contact
                .shielded_address
                .unwrap_or([0; SHIELDED_ADDRESS_LEN]),
        }
    }
}

/// Read what this identity's private contact requests with `contact_id`
/// carry into `out_contact`. Returns `NotFound` when neither tracked request
/// between them came or went privately. Whether one did is the stored
/// request's own, so it holds across restarts; the contact's shielded address
/// is kept in memory only and rebuilt by
/// `platform_wallet_sync_private_contact_requests`, so until a pass has run
/// it is absent.
///
/// # Safety
/// - `contact_id` must point to 32 readable bytes.
/// - `out_contact` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn managed_identity_get_private_contact(
    identity_handle: Handle,
    contact_id: *const u8,
    out_contact: *mut PrivateContactFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_contact);
    unsafe { *out_contact = PrivateContactFFI::empty() };
    let contact_id = unwrap_result_or_return!(unsafe { read_identifier(contact_id) });

    let option = MANAGED_IDENTITY_STORAGE.with_item(identity_handle, |identity| {
        identity
            .dashpay()
            .private_contact(&contact_id)
            .map(PrivateContactFFI::from)
    });
    let inner = unwrap_option_or_return!(option);
    let contact = unwrap_option_or_return!(inner);
    unsafe { *out_contact = contact };
    PlatformWalletFFIResult::ok()
}

/// Whether a contact request from `sender_id`, one of the wallet's
/// identities, to `recipient_id`, public or private, is held for a private
/// one, as a send decides it (`DashPayView::private_pair_held`), into
/// `out_held`: a private request to the recipient is being paid for, or was
/// paid for and is not yet read back from the store, or a private payment the
/// shielded store's journal keeps unresolved holds the pair, also after the
/// sender identity was removed and added again. While it is, a private send
/// or reply to the recipient is refused with `ErrorPrivateContactRequestPending`
/// (71) and a public one is held, so a host shows the pair as pending and
/// offers neither. The entry read back
/// (`platform_wallet_sync_private_contact_requests`) frees it, as does the
/// payment proven never to execute (its anchor pruned, which the shielded
/// sync judges); an endpoint's refusal of the payment does not. Not a pure
/// read: a journal record discharged since is applied first, briefly taking
/// the wallet's write lock, so a discharge reads as freed with no send or
/// sync in between.
///
/// `out_held` is reset to false first. An unknown `wallet_handle` is
/// `NotFound`; a sender the wallet does not hold is `ErrorIdentityNotFound`
/// (7).
///
/// # Safety
/// - `sender_id` and `recipient_id` must each point to 32 readable bytes.
/// - `out_held` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_private_pair_held(
    wallet_handle: Handle,
    sender_id: *const u8,
    recipient_id: *const u8,
    out_held: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_held);
    unsafe { *out_held = false };
    let sender_id = unwrap_result_or_return!(unsafe { read_identifier(sender_id) });
    let recipient_id = unwrap_result_or_return!(unsafe { read_identifier(recipient_id) });
    // Look the wallet up under the registry guard, but read outside it: the
    // read may wait on the wallet's write lock, and a registry guard held
    // across that wait would stall `platform_wallet_destroy`.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        identity
            .dashpay()
            .private_pair_held(&sender_id, &recipient_id)
            .await
    });
    let held = unwrap_result_or_return!(result);
    unsafe { *out_held = held };
    PlatformWalletFFIResult::ok()
}

/// Whether a public contact request from `sender_id`, one of the wallet's
/// identities, to `recipient_id` was broadcast and may have been delivered,
/// with no proof settling its outcome yet
/// (`DashPayView::public_contact_request_unconfirmed`), into
/// `out_unconfirmed`. The send or accept that broadcast it reported
/// `ErrorTransactionBroadcastUnconfirmed` (20); while it stands, every
/// contact request or acceptance to the recipient, public or private, is
/// refused with `ErrorPublicContactRequestUnconfirmed` (76), nothing signed,
/// so a host shows the pair as unconfirmed and offers neither. A public
/// contact request sync settles it: the request proved on Platform is
/// recorded as sent, and one proved unable to write it frees the pair. The
/// per-contact read beside `platform_wallet_get_private_pair_held`, so a
/// host tells a public request awaiting its outcome from a private one
/// pending.
///
/// `out_unconfirmed` is reset to false first. An unknown `wallet_handle` is
/// `NotFound`; a sender the wallet does not hold is `ErrorIdentityNotFound`
/// (7).
///
/// # Safety
/// - `sender_id` and `recipient_id` must each point to 32 readable bytes.
/// - `out_unconfirmed` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_public_contact_request_unconfirmed(
    wallet_handle: Handle,
    sender_id: *const u8,
    recipient_id: *const u8,
    out_unconfirmed: *mut bool,
) -> PlatformWalletFFIResult {
    check_ptr!(out_unconfirmed);
    unsafe { *out_unconfirmed = false };
    let sender_id = unwrap_result_or_return!(unsafe { read_identifier(sender_id) });
    let recipient_id = unwrap_result_or_return!(unsafe { read_identifier(recipient_id) });
    // Look the wallet up under the registry guard, but read outside it: the
    // read waits on the wallet's lock, and a registry guard held across that
    // wait would stall `platform_wallet_destroy`.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        identity
            .dashpay()
            .public_contact_request_unconfirmed(&sender_id, &recipient_id)
            .await
    });
    let unconfirmed = unwrap_result_or_return!(result);
    unsafe { *out_unconfirmed = unconfirmed };
    PlatformWalletFFIResult::ok()
}

/// Why the last restoration of the wallet's private payments failed, while
/// they are not restored for `identity_id` and none succeeded since
/// (`PrivateContactReadiness::private_payments_restore_failure`, which
/// `platform_wallet_get_private_contact_readiness` reports as
/// `private_payments_restore_failed`), into `out_reason`: a NUL-terminated
/// UTF-8 string the caller releases with `platform_wallet_string_free`, or
/// null when none failed (the payments are restored, or only not restored
/// yet). It reads the readiness, so a journal record discharged since is
/// applied first, as there.
///
/// `out_reason` is reset to null first. An unknown `wallet_handle` is
/// `NotFound`; an identity the wallet does not hold is
/// `ErrorIdentityNotFound` (7).
///
/// # Safety
/// - `identity_id` must point to 32 readable bytes.
/// - `out_reason` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_private_payments_restore_failure(
    wallet_handle: Handle,
    identity_id: *const u8,
    out_reason: *mut *mut c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(out_reason);
    unsafe { *out_reason = std::ptr::null_mut() };
    let identity_id = unwrap_result_or_return!(unsafe { read_identifier(identity_id) });
    // Look the wallet up under the registry guard, but read outside it: the
    // read may wait on the wallet's write lock, and a registry guard held
    // across that wait would stall `platform_wallet_destroy`.
    let option =
        PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| wallet.identity().clone());
    let identity = unwrap_option_or_return!(option);
    let result = block_on_worker(async move {
        identity
            .dashpay()
            .private_contact_readiness(&identity_id)
            .await
    });
    let readiness = unwrap_result_or_return!(result);
    if let Some(reason) = readiness.private_payments_restore_failure {
        let reason = unwrap_result_or_return!(CString::new(reason));
        unsafe { *out_reason = reason.into_raw() };
    }
    PlatformWalletFFIResult::ok()
}

/// Send a DashPay private contact request from `sender_identity_id` to
/// `recipient_identity_id`, paying for it from `account`'s shielded notes, so
/// nothing on chain names either identity. The request carries our receiving
/// xpub for the contact, the masked account reference, `account_label`,
/// `auto_accept_proof`, and an Orchard address the contact may pay us at
/// privately, which the wallet derives for this contact alone. It is sealed
/// to the recipient's DashPay key inside the resolver-backed signer.
///
/// `mnemonic_resolver_handle` supplies both the transient Orchard spend
/// authority and the DashPay keys. `signer_handle` is the identity signer
/// the public send takes (`platform_wallet_send_contact_request_with_signer`):
/// it signs the request with one of the sender's authentication keys, as it
/// signs a transition. Writes a handle into `CONTACT_REQUEST_STORAGE` to
/// `out_request_handle` (release with `contact_request_destroy`).
///
/// Errors follow the shielded spend contract: `ErrorShieldedSpendUnconfirmed`
/// means the payment may already be on chain; do not send again.
///
/// # Safety
/// - `wallet_id_bytes`, `sender_identity_id` and `recipient_identity_id` must
///   each point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must be a valid, non-destroyed handle; the
///   caller retains ownership and pins it for the call.
/// - `account_label`, when non-null, must be a valid NUL-terminated UTF-8 C
///   string.
/// - `auto_accept_proof`, when non-null, must point to
///   `auto_accept_proof_len` readable bytes; `(null, 0)` omits it.
/// - `signer_handle` must be a valid, non-destroyed `*mut SignerHandle`
///   produced by `dash_sdk_signer_create_with_ctx`; the caller retains
///   ownership and pins it for the call.
/// - `out_request_handle` must be a valid, writable pointer.
#[cfg(feature = "shielded")]
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_send_private_contact_request(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    sender_identity_id: *const u8,
    recipient_identity_id: *const u8,
    account_label: *const c_char,
    auto_accept_proof: *const u8,
    auto_accept_proof_len: usize,
    signer_handle: *mut SignerHandle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    // A panic in the proving task must not reach this `extern "C"` frame; the
    // notes may already be spent when it strikes, so the ambiguous
    // spend-unconfirmed contract applies (see `catch_spend_panic`).
    catch_spend_panic("private contact request", || {
        send_private_contact_request_inner(
            handle,
            wallet_id_bytes,
            mnemonic_resolver_handle,
            account,
            sender_identity_id,
            recipient_identity_id,
            account_label,
            auto_accept_proof,
            auto_accept_proof_len,
            signer_handle,
            out_request_handle,
        )
    })
}

/// Body of [`platform_wallet_manager_send_private_contact_request`], as an
/// ordinary Rust function so a panic unwinds into [`catch_spend_panic`].
///
/// # Safety
/// Identical contract to the export that calls it.
#[cfg(feature = "shielded")]
#[allow(clippy::too_many_arguments)]
unsafe fn send_private_contact_request_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    sender_identity_id: *const u8,
    recipient_identity_id: *const u8,
    account_label: *const c_char,
    auto_accept_proof: *const u8,
    auto_accept_proof_len: usize,
    signer_handle: *mut SignerHandle,
    out_request_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(signer_handle);
    check_ptr!(out_request_handle);
    unsafe { *out_request_handle = NULL_HANDLE };

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let sender = unwrap_result_or_return!(read_identifier(sender_identity_id));
    let recipient = unwrap_result_or_return!(read_identifier(recipient_identity_id));
    let label = if account_label.is_null() {
        None
    } else {
        Some(unwrap_result_or_return!(CStr::from_ptr(account_label).to_str()).to_string())
    };
    let proof = match read_auto_accept_proof(auto_accept_proof, auto_accept_proof_len) {
        Ok(proof) => proof,
        Err(result) => return result,
    };

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(pair) => pair,
        Err(result) => return result,
    };
    let seed = match resolve_seed_from_resolver(mnemonic_resolver_handle, &wallet_id) {
        Ok(seed) => seed,
        Err(result) => return result,
    };
    // SAFETY: the caller pins the resolver handle for the duration of this
    // call.
    let crypto = unsafe {
        resolver_contact_crypto_provider(mnemonic_resolver_handle, wallet_id, wallet.network())
    };
    // The signer pointer crosses into the worker as a plain address and is
    // re-materialized there, as the public send does.
    let signer_addr = signer_handle as usize;

    // The proof runs on a worker thread: Halo 2 synthesis recurses past the
    // iOS dispatch-thread stack.
    let result = block_on_worker(async move {
        // SAFETY: the caller pins the signer for the duration of this call,
        // and `block_on_worker` blocks this frame until the task completes.
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        let result = wallet
            .send_private_contact_request(
                &coordinator,
                seed.as_ref(),
                account,
                &sender,
                &recipient,
                label,
                AutoAcceptProofSource::from_option(proof),
                signer,
                &crypto,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&result, handle);
        result
    });
    match result {
        Ok(request) => {
            unsafe { *out_request_handle = CONTACT_REQUEST_STORAGE.insert(request) };
            PlatformWalletFFIResult::ok()
        }
        Err(error) => seed_mismatch_result(error)
            .unwrap_or_else(|error| map_spend_result(Err(error), "private contact request")),
    }
}

/// Accept a DashPay contact request that came privately, replying privately
/// and paying for the reply from `account`'s shielded notes, as
/// [`platform_wallet_manager_send_private_contact_request`] pays. The reply
/// carries an Orchard address the contact may pay us at privately, which the
/// wallet derives for this contact alone.
///
/// `request_handle` is a live `CONTACT_REQUEST_STORAGE` handle of the
/// incoming request. `signer_handle` is the identity signer the public accept
/// takes (`platform_wallet_accept_contact_request_with_signer`); it signs the
/// reply. Writes a handle into `ESTABLISHED_CONTACT_STORAGE` to
/// `out_established_handle` (release with `established_contact_destroy`).
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must be a valid, non-destroyed handle; the
///   caller retains ownership and pins it for the call.
/// - `signer_handle` must be a valid, non-destroyed `*mut SignerHandle`
///   produced by `dash_sdk_signer_create_with_ctx`; the caller retains
///   ownership and pins it for the call.
/// - `out_established_handle` must be a valid, writable pointer.
#[cfg(feature = "shielded")]
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_accept_private_contact_request(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    request_handle: Handle,
    signer_handle: *mut SignerHandle,
    out_established_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    // Same panic contract as the send: the reply's notes may be spent.
    catch_spend_panic("private contact request accept", || {
        accept_private_contact_request_inner(
            handle,
            wallet_id_bytes,
            mnemonic_resolver_handle,
            account,
            request_handle,
            signer_handle,
            out_established_handle,
        )
    })
}

/// Body of [`platform_wallet_manager_accept_private_contact_request`], as an
/// ordinary Rust function so a panic unwinds into [`catch_spend_panic`].
///
/// # Safety
/// Identical contract to the export that calls it.
#[cfg(feature = "shielded")]
#[allow(clippy::too_many_arguments)]
unsafe fn accept_private_contact_request_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    request_handle: Handle,
    signer_handle: *mut SignerHandle,
    out_established_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(signer_handle);
    check_ptr!(out_established_handle);
    unsafe { *out_established_handle = NULL_HANDLE };

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let request = CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.clone());
    let request = unwrap_option_or_return!(request);

    let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
        Ok(pair) => pair,
        Err(result) => return result,
    };
    let seed = match resolve_seed_from_resolver(mnemonic_resolver_handle, &wallet_id) {
        Ok(seed) => seed,
        Err(result) => return result,
    };
    // SAFETY: the caller pins the resolver handle for the duration of this
    // call.
    let crypto = unsafe {
        resolver_contact_crypto_provider(mnemonic_resolver_handle, wallet_id, wallet.network())
    };
    let signer_addr = signer_handle as usize;

    let result = block_on_worker(async move {
        // SAFETY: as in the send, the caller pins the signer for the call.
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        let result = wallet
            .accept_private_contact_request(
                &coordinator,
                seed.as_ref(),
                account,
                &request,
                signer,
                &crypto,
                &prover,
            )
            .await;
        poke_sync_on_unconfirmed(&result, handle);
        result
    });
    match result {
        Ok(contact) => {
            unsafe { *out_established_handle = ESTABLISHED_CONTACT_STORAGE.insert(contact) };
            PlatformWalletFFIResult::ok()
        }
        Err(error) => seed_mismatch_result(error)
            .unwrap_or_else(|error| map_spend_result(Err(error), "private contact request accept")),
    }
}

/// Accept a DashPay contact request replying the way it came
/// (`accept_contact_request_replying_in_kind`): privately, paid from
/// `account`'s shielded notes, when it came privately and no reply exists
/// yet; otherwise with a `contactRequest` document signed by `signer_handle`,
/// or by adopting the reply already sent. Which reply a request takes is the
/// wallet's decision: this is the one accept a host calls. A private reply
/// carries an Orchard address the contact may pay us at privately, which the
/// wallet derives for this contact alone. Exported in every build.
///
/// Only a private reply needs shielded support and the spend authority: the
/// shielded coordinator is looked up, and the mnemonic read through
/// `mnemonic_resolver_handle`, when the wallet asks for the payment. A public
/// or adopted reply works on a manager without
/// `platform_wallet_manager_configure_shielded` and reads no mnemonic for the
/// payment (the DashPay keys still come from the resolver). In a build
/// without the `shielded` feature a request that needs a private reply fails
/// with `ErrorShieldedBuildRequired`, and every other accept works as in a
/// shielded build.
///
/// Errors: a public or adopted reply fails with the codes the public accept
/// returns. Once a private reply's payment is asked for, errors follow the
/// shielded spend contract: `ErrorShieldedSpendUnconfirmed` means the reply
/// may already be on chain; do not accept again. A failed payment lookup
/// returns the code the lookup reported.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must be a valid, non-destroyed handle; the
///   caller retains ownership and pins it for the call.
/// - `signer_handle` must be a valid, non-destroyed `*mut SignerHandle`
///   produced by `dash_sdk_signer_create_with_ctx`; the caller retains
///   ownership and pins it for the call.
/// - `out_established_handle` must be a valid, writable pointer.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_accept_contact_request_replying_in_kind(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    request_handle: Handle,
    signer_handle: *mut SignerHandle,
    out_established_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    let accept = || {
        accept_contact_request_replying_in_kind_inner(
            handle,
            wallet_id_bytes,
            mnemonic_resolver_handle,
            account,
            request_handle,
            signer_handle,
            out_established_handle,
        )
    };
    // Same panic contract as the private accept: a private reply's notes may
    // be spent when it strikes. A build without shielded support spends none.
    #[cfg(feature = "shielded")]
    let result = catch_spend_panic("contact request accept", accept);
    #[cfg(not(feature = "shielded"))]
    let result = accept();
    result
}

/// Why a private reply could not be paid for, as the FFI reports it: the
/// export returns this code and message rather than the wallet error the
/// payment hands back to the wallet. Kept as owned parts, since a result
/// cannot cross threads.
type PaymentFailure = (PlatformWalletFFIResultCode, String);

/// The parts of a failed FFI `result`; its message is freed with it.
#[cfg(feature = "shielded")]
fn payment_failure(result: PlatformWalletFFIResult) -> PaymentFailure {
    let message = if result.message.is_null() {
        String::new()
    } else {
        // SAFETY: a non-null message is a NUL-terminated string the FFI made.
        unsafe { CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    };
    (result.code, message)
}

/// Record `failure` for the export to return, and the wallet error the
/// payment hands back to the wallet in its place.
fn record_payment_failure(
    slot: &Mutex<Option<PaymentFailure>>,
    failure: PaymentFailure,
) -> PlatformWalletError {
    let error = PlatformWalletError::InvalidParameter(failure.1.clone());
    if let Ok(mut recorded) = slot.lock() {
        *recorded = Some(failure);
    }
    error
}

/// The wallet of `wallet_id` under the manager `handle`, without the
/// shielded coordinator, which only a private reply needs. Wait-free (the
/// manager's wallet map is read without a lock or the runtime), with the
/// errors `resolve_wallet_and_coordinator` reports.
fn resolve_manager_wallet(
    handle: Handle,
    wallet_id: &[u8; 32],
) -> Result<Arc<platform_wallet::PlatformWallet>, PlatformWalletFFIResult> {
    match PLATFORM_WALLET_MANAGER_STORAGE
        .with_item(handle, |manager| manager.get_wallet_blocking(wallet_id))
    {
        None => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            format!("invalid manager handle: {handle}"),
        )),
        Some(None) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("wallet not found: {}", hex::encode(wallet_id)),
        )),
        Some(Some(wallet)) => Ok(wallet),
    }
}

/// What a private reply is paid with, looked up only when the wallet asks:
/// the shielded coordinator, then the seed read through the resolver at
/// `resolver_addr`. Runs on a blocking thread (the coordinator lookup blocks
/// on the runtime, and the resolver calls into the host).
///
/// # Safety
/// `resolver_addr` must be the address of a valid `MnemonicResolverHandle`
/// that the export's caller pins for the call.
#[cfg(feature = "shielded")]
unsafe fn private_reply_payment_parts(
    handle: Handle,
    wallet_id: [u8; 32],
    resolver_addr: usize,
) -> Result<(Arc<NetworkShieldedCoordinator>, Zeroizing<[u8; 64]>), PaymentFailure> {
    let coordinator = resolve_shielded_coordinator(handle).map_err(payment_failure)?;
    let seed = resolve_seed_from_resolver(resolver_addr as *mut MnemonicResolverHandle, &wallet_id)
        .map_err(payment_failure)?;
    Ok((coordinator, seed))
}

/// Body of [`platform_wallet_manager_accept_contact_request_replying_in_kind`],
/// as an ordinary Rust function so a panic unwinds into `catch_spend_panic`
/// in a shielded build.
///
/// # Safety
/// Identical contract to the export that calls it.
#[allow(clippy::too_many_arguments)]
unsafe fn accept_contact_request_replying_in_kind_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    account: u32,
    request_handle: Handle,
    signer_handle: *mut SignerHandle,
    out_established_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(signer_handle);
    check_ptr!(out_established_handle);
    unsafe { *out_established_handle = NULL_HANDLE };

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let request = CONTACT_REQUEST_STORAGE.with_item(request_handle, |request| request.clone());
    let request = unwrap_option_or_return!(request);

    let wallet = match resolve_manager_wallet(handle, &wallet_id) {
        Ok(wallet) => wallet,
        Err(result) => return result,
    };
    // SAFETY: the caller pins the resolver handle for the duration of this
    // call.
    let crypto = unsafe {
        resolver_contact_crypto_provider(mnemonic_resolver_handle, wallet_id, wallet.network())
    };
    let signer_addr = signer_handle as usize;
    // Why paying for a private reply failed, when it did: the export returns
    // that, not the wallet error standing in for it.
    let payment_failure_slot: Arc<Mutex<Option<PaymentFailure>>> = Arc::new(Mutex::new(None));
    let failure_slot = Arc::clone(&payment_failure_slot);

    #[cfg(feature = "shielded")]
    let payment_asked = Arc::new(AtomicBool::new(false));
    #[cfg(feature = "shielded")]
    let result = {
        let asked = Arc::clone(&payment_asked);
        let resolver_addr = mnemonic_resolver_handle as usize;
        block_on_worker(async move {
            // SAFETY: as in the private accept, the caller pins the signer for
            // the call.
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            let prover = CachedOrchardProver::new();
            let result = wallet
                .accept_contact_request_replying_in_kind(&request, signer, &crypto, || async {
                    asked.store(true, Ordering::SeqCst);
                    let parts = tokio::task::spawn_blocking(move || {
                        // SAFETY: the caller pins the resolver for the call,
                        // and this export blocks until the task completes.
                        unsafe { private_reply_payment_parts(handle, wallet_id, resolver_addr) }
                    })
                    .await
                    .unwrap_or_else(|_| {
                        Err((
                            PlatformWalletFFIResultCode::ErrorUnknown,
                            "the private reply's payment lookup panicked".to_string(),
                        ))
                    });
                    match parts {
                        Ok((coordinator, seed)) => Ok(PrivateReplyPayment {
                            coordinator,
                            seed,
                            account,
                            prover: &prover,
                        }),
                        Err(failure) => Err(record_payment_failure(&failure_slot, failure)),
                    }
                })
                .await;
            poke_sync_on_unconfirmed(&result, handle);
            result
        })
    };
    // Without shielded support a private reply cannot be paid for; every
    // other reply is the same as in a shielded build.
    #[cfg(not(feature = "shielded"))]
    let result = {
        let _ = account;
        let identity = wallet.identity().clone();
        block_on_worker(async move {
            // SAFETY: the caller pins the signer for the call.
            let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
            identity
                .dashpay()
                .accept_contact_request_replying_in_kind(
                    &request,
                    signer,
                    &crypto,
                    // Refused before any payment could reach the network,
                    // so the payment is never marked as started.
                    |_entry, _started| async move {
                        Err::<ResponseMetadata, _>(record_payment_failure(
                            &failure_slot,
                            (
                                PlatformWalletFFIResultCode::ErrorShieldedBuildRequired,
                                "this contact request needs a private reply, which is paid \
                                 from the shielded pool; this build has no shielded support"
                                    .to_string(),
                            ),
                        ))
                    },
                )
                .await
        })
    };

    match result {
        Ok(contact) => {
            unsafe { *out_established_handle = ESTABLISHED_CONTACT_STORAGE.insert(contact) };
            PlatformWalletFFIResult::ok()
        }
        Err(error) => {
            if let Some((code, message)) = payment_failure_slot
                .lock()
                .ok()
                .and_then(|mut slot| slot.take())
            {
                return PlatformWalletFFIResult::err(code, message);
            }
            let error = match seed_mismatch_result(error) {
                Ok(result) => return result,
                Err(error) => error,
            };
            #[cfg(feature = "shielded")]
            if payment_asked.load(Ordering::SeqCst) {
                return map_spend_result(Err(error), "contact request accept");
            }
            // No payment was asked for, so nothing was spent: report the
            // reply's failure as the public accept does.
            PlatformWalletFFIResult::from(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wrong seed refused by the private sync, or by the store read a send
    /// or accept makes first, takes the code every DashPay export gives it;
    /// any other error is left to the export's own mapping.
    #[test]
    fn should_report_a_seed_mismatch_from_the_private_store_by_its_dashpay_code() {
        let result = seed_mismatch_result(PlatformWalletError::SeedMismatch {
            wallet_id: "ab".to_string(),
        })
        .expect("a seed mismatch is mapped here");
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert!(matches!(
            seed_mismatch_result(PlatformWalletError::PrivateContactPaymentsNotRestored),
            Err(PlatformWalletError::PrivateContactPaymentsNotRestored)
        ));
    }

    #[test]
    fn should_clear_both_sync_outputs_on_an_unknown_wallet() {
        let mut requests = ContactRequestHandleArray {
            handles: std::ptr::NonNull::dangling().as_ptr(),
            count: 3,
        };
        let mut report = PrivateContactSyncReportFFI {
            next_position: 9,
            pages_read: 9,
            rejected_count: 9,
            unpersisted_identity_count: 9,
            complete: true,
        };
        // A resolver is never called for an unknown wallet; any non-null
        // pointer passes the null check.
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let result = unsafe {
            platform_wallet_sync_private_contact_requests(
                0xDEAD_BEEF,
                resolver,
                1,
                &mut requests,
                &mut report,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert!(requests.handles.is_null());
        assert_eq!(requests.count, 0);
        assert_eq!(report, PrivateContactSyncReportFFI::default());
    }

    #[test]
    fn should_reject_a_null_resolver_on_sync() {
        let mut requests = ContactRequestHandleArray::empty();
        let mut report = PrivateContactSyncReportFFI::default();
        let result = unsafe {
            platform_wallet_sync_private_contact_requests(
                0,
                std::ptr::null_mut(),
                1,
                &mut requests,
                &mut report,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// The request is signed by the identity signer, so the private send
    /// refuses to run without one, before it touches the wallet.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_reject_a_null_identity_signer_on_the_private_send() {
        let wallet_id = [1u8; 32];
        let sender = [2u8; 32];
        let recipient = [3u8; 32];
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let mut out = 0xDEAD_BEEF;
        let result = unsafe {
            platform_wallet_manager_send_private_contact_request(
                0xDEAD_BEEF,
                wallet_id.as_ptr(),
                resolver,
                0,
                sender.as_ptr(),
                recipient.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// As the private send: the reply is signed by the identity signer.
    #[cfg(feature = "shielded")]
    #[test]
    fn should_reject_a_null_identity_signer_on_the_private_accept() {
        let wallet_id = [1u8; 32];
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let mut out = 0xDEAD_BEEF;
        let result = unsafe {
            platform_wallet_manager_accept_private_contact_request(
                0xDEAD_BEEF,
                wallet_id.as_ptr(),
                resolver,
                0,
                0xDEAD_BEEF,
                std::ptr::null_mut(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// As the private accept: whichever reply the wallet picks is signed by
    /// the identity signer.
    #[test]
    fn should_reject_a_null_identity_signer_on_the_accept_replying_in_kind() {
        let wallet_id = [1u8; 32];
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let mut out = 0xDEAD_BEEF;
        let result = unsafe {
            platform_wallet_manager_accept_contact_request_replying_in_kind(
                0xDEAD_BEEF,
                wallet_id.as_ptr(),
                resolver,
                0,
                0xDEAD_BEEF,
                std::ptr::null_mut(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// A request that did not come privately is accepted with a document, which
    /// needs neither shielded support nor the spend authority: on a manager
    /// whose shielded support was never configured (and in a build without
    /// it), the accept gets past the lookups to the wallet's own refusal (the
    /// recipient is not one of its identities, reported as the public accept
    /// reports it), and the resolver is never asked for the mnemonic.
    #[test]
    fn should_accept_publicly_without_shielded_support_or_a_mnemonic_read() {
        use crate::contact_request::contact_request_destroy;
        use crate::event_handler::EventHandlerCallbacks;
        use crate::manager::{
            platform_wallet_manager_create, platform_wallet_manager_create_wallet_from_mnemonic,
            platform_wallet_manager_destroy,
        };
        use crate::persistence::PersistenceCallbacks;
        use platform_wallet::ContactRequest;
        use rs_sdk_ffi::{
            dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy,
            dash_sdk_signer_create, dash_sdk_signer_destroy, mnemonic_resolver_result,
            SignCompletionCallback,
        };
        use std::ffi::{c_void, CStr};
        use std::os::raw::c_char;
        use std::sync::atomic::{AtomicUsize, Ordering};

        unsafe extern "C" fn begin(_: *mut c_void, _: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end(_: *mut c_void, _: *const u8, _: bool) -> i32 {
            0
        }
        unsafe extern "C" fn counting_resolve(
            ctx: *const c_void,
            _wallet_id: *const u8,
            _out: *mut c_char,
            _capacity: usize,
            _out_len: *mut usize,
        ) -> i32 {
            (*(ctx as *const AtomicUsize)).fetch_add(1, Ordering::SeqCst);
            mnemonic_resolver_result::NOT_FOUND
        }
        unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}
        // A public reply to this request is refused before anything is signed;
        // a signature request would fail rather than hang.
        unsafe extern "C" fn refuse_to_sign(
            _signer: *const c_void,
            _pubkey: *const u8,
            _pubkey_len: usize,
            _key_type: u8,
            _data: *const u8,
            _data_len: usize,
            completion_ctx: *mut c_void,
            completion: SignCompletionCallback,
        ) {
            completion(
                completion_ctx,
                std::ptr::null(),
                0,
                1,
                c"no key in this test".as_ptr(),
            );
        }
        unsafe extern "C" fn cannot_sign(
            _signer: *const c_void,
            _pubkey: *const u8,
            _pubkey_len: usize,
            _key_type: u8,
        ) -> bool {
            false
        }

        let sdk = dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk");
        let persistence = PersistenceCallbacks {
            on_changeset_begin_fn: Some(begin),
            on_changeset_end_fn: Some(end),
            ..Default::default()
        };
        let events = EventHandlerCallbacks {
            context: std::ptr::null_mut(),
            on_wallet_event_fn: None,
            on_error_fn: None,
            on_platform_address_sync_completed_fn: None,
            on_shielded_sync_completed_fn: None,
            on_shielded_sync_progress_fn: None,
            on_shielded_tree_progress_fn: None,
            release_fn: None,
        };
        let mut manager = NULL_HANDLE;
        let created = unsafe {
            platform_wallet_manager_create(
                &sdk as *const dash_sdk::Sdk as *const c_void,
                &persistence,
                &events,
                &mut manager,
            )
        };
        assert_eq!(created.code, PlatformWalletFFIResultCode::Success);

        let mnemonic = c"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mut wallet_handle = NULL_HANDLE;
        let mut wallet_id = [0u8; 32];
        let added = unsafe {
            platform_wallet_manager_create_wallet_from_mnemonic(
                manager,
                mnemonic.as_ptr(),
                FFINetwork::Testnet,
                0,
                &mut wallet_handle,
                &mut wallet_id,
            )
        };
        assert_eq!(
            added.code,
            PlatformWalletFFIResultCode::Success,
            "the wallet is added"
        );

        // A request to an identity the wallet does not manage.
        let request = ContactRequest::new(
            [0xA0; 32].into(),
            [0xB0; 32].into(),
            1,
            2,
            7,
            vec![0x33; 96],
            100,
            1_700_000_000_000,
        );
        let request_handle = CONTACT_REQUEST_STORAGE.insert(request);
        let reads = AtomicUsize::new(0);
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(
                &reads as *const AtomicUsize as *mut c_void,
                counting_resolve,
                noop_destroy,
            )
        };
        let signer = unsafe { dash_sdk_signer_create(refuse_to_sign, cannot_sign, None) };

        let mut established = 0xDEAD_BEEF;
        let result = unsafe {
            platform_wallet_manager_accept_contact_request_replying_in_kind(
                manager,
                wallet_id.as_ptr(),
                resolver,
                0,
                request_handle,
                signer,
                &mut established,
            )
        };
        // Exactly what the public accept reports for this refusal (the
        // wallet's `IdentityNotFound`), not the missing-coordinator error an
        // up-front lookup gave.
        let expected =
            PlatformWalletFFIResult::from(PlatformWalletError::IdentityNotFound([0xB0; 32].into()));
        let message_of = |result: &PlatformWalletFFIResult| {
            if result.message.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(result.message) }
                    .to_string_lossy()
                    .into_owned()
            }
        };
        assert_eq!(result.code, expected.code, "{}", message_of(&result));
        assert_eq!(message_of(&result), message_of(&expected));
        assert_eq!(established, NULL_HANDLE);
        assert_eq!(reads.load(Ordering::SeqCst), 0, "no mnemonic was read");

        let _ = contact_request_destroy(request_handle);
        unsafe {
            dash_sdk_signer_destroy(signer);
            dash_sdk_mnemonic_resolver_destroy(resolver);
        }
        let _ = unsafe { platform_wallet_manager_destroy(manager) };
    }

    /// The accept replying in kind, for an identity the wallet manages,
    /// routes a request that came publicly without asking for shielded
    /// payment resources, on a manager whose shielded support was never
    /// configured (and in a build without it): a request already replied to
    /// is adopted, and a fresh one gets a public reply that reaches the
    /// identity signer. Payment resources (the shielded coordinator and the
    /// spend seed) would fail here with "not configured"; the resolver is
    /// still read, for the DashPay crypto (the seed check and the reply's
    /// encryption), which is what a public reply legitimately needs.
    #[test]
    fn should_adopt_or_reply_publicly_for_a_managed_identity_without_payment_resources() {
        use crate::contact_request::contact_request_destroy;
        use crate::event_handler::EventHandlerCallbacks;
        use crate::manager::{
            platform_wallet_manager_create, platform_wallet_manager_create_wallet_from_mnemonic,
            platform_wallet_manager_destroy,
        };
        use crate::persistence::PersistenceCallbacks;
        use crate::runtime::runtime;
        use dash_sdk::error::ContextProviderError;
        use dash_sdk::platform::dashpay::PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME;
        use dash_sdk::platform::private_document_entries::{
            PrivateDocumentEntries, PrivateDocumentEntriesQuery,
        };
        use dash_sdk::platform::ContextProvider;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::data_contract::{DataContract, TokenConfiguration};
        use dpp::identity::accessors::IdentityGettersV0;
        use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::{Identity, IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;
        use dpp::prelude::Identifier;
        use dpp::version::PlatformVersion;
        use key_wallet::bip32::KeyDerivationType;
        use key_wallet::mnemonic::Mnemonic;
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use platform_wallet::wallet::identity::network::identity_auth_derivation_path_for_type;
        use platform_wallet::ContactRequest;
        use rs_sdk_ffi::{
            dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy,
            dash_sdk_signer_create, dash_sdk_signer_destroy, mnemonic_resolver_result,
            SignCompletionCallback,
        };
        use std::ffi::{c_void, CString};
        use std::os::raw::c_char;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        const BOB_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon \
                                  abandon abandon abandon abandon about";
        const ALICE_PHRASE: &str =
            "legal winner thank year wave sausage worth useful legal winner thank yellow";
        const SIGNING_KEY_ID: KeyID = 0;
        const ENCRYPTION_KEY_ID: KeyID = 1;
        const DECRYPTION_KEY_ID: KeyID = 2;

        struct DashpayProvider(Arc<DataContract>);
        impl ContextProvider for DashpayProvider {
            fn get_data_contract(
                &self,
                id: &Identifier,
                _platform_version: &PlatformVersion,
            ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
                Ok((*id == self.0.id()).then(|| Arc::clone(&self.0)))
            }
            fn get_token_configuration(
                &self,
                _token_id: &Identifier,
            ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
                Ok(None)
            }
            fn get_quorum_public_key(
                &self,
                _quorum_type: u32,
                _quorum_hash: [u8; 32],
                _core_chain_locked_height: u32,
            ) -> Result<[u8; 48], ContextProviderError> {
                Err(ContextProviderError::Config(
                    "no quorum keys in this test".into(),
                ))
            }
            fn get_platform_activation_height(&self) -> Result<u32, ContextProviderError> {
                Ok(1)
            }
        }

        /// Identity index 0's ECDSA keys of `phrase`, as the wallet derives them.
        fn identity(phrase: &str, id: [u8; 32]) -> Identity {
            let seed = *Mnemonic::from_phrase(phrase)
                .expect("test mnemonic")
                .to_seed("");
            let wallet = key_wallet::wallet::Wallet::from_seed_bytes(
                seed,
                key_wallet::Network::Testnet,
                WalletAccountCreationOptions::None,
            )
            .expect("test wallet");
            let key = |key_id: KeyID, purpose: Purpose, security_level: SecurityLevel| {
                let path = identity_auth_derivation_path_for_type(
                    key_wallet::Network::Testnet,
                    KeyDerivationType::ECDSA,
                    0,
                    key_id,
                )
                .expect("identity key path");
                let xprv = wallet
                    .derive_extended_private_key(&path)
                    .expect("identity key");
                let secret =
                    dpp::secp256k1::SecretKey::from_secret_bytes(*xprv.private_key.to_bytes())
                        .expect("secret key");
                IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: key_id,
                    purpose,
                    security_level,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(
                        dpp::secp256k1::PublicKey::from_secret_key(&secret)
                            .serialize()
                            .to_vec(),
                    ),
                    disabled_at: None,
                })
            };
            let keys = [
                key(SIGNING_KEY_ID, Purpose::AUTHENTICATION, SecurityLevel::HIGH),
                key(
                    ENCRYPTION_KEY_ID,
                    Purpose::ENCRYPTION,
                    SecurityLevel::MEDIUM,
                ),
                key(
                    DECRYPTION_KEY_ID,
                    Purpose::DECRYPTION,
                    SecurityLevel::MEDIUM,
                ),
            ];
            Identity::V0(IdentityV0 {
                id: Identifier::from(id),
                public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
                balance: 0,
                revision: 0,
            })
        }
        fn public_request(sender: &Identity, recipient: &Identity, account: u32) -> ContactRequest {
            ContactRequest::new(
                sender.id(),
                recipient.id(),
                ENCRYPTION_KEY_ID,
                DECRYPTION_KEY_ID,
                account,
                vec![7; 96],
                100,
                1_700_000_000_000,
            )
        }

        unsafe extern "C" fn begin(_: *mut c_void, _: *const u8) -> i32 {
            0
        }
        unsafe extern "C" fn end(_: *mut c_void, _: *const u8, _: bool) -> i32 {
            0
        }
        /// Bob's mnemonic, counting the reads.
        unsafe extern "C" fn bob_resolve(
            ctx: *const c_void,
            _wallet_id: *const u8,
            out: *mut c_char,
            capacity: usize,
            out_len: *mut usize,
        ) -> i32 {
            (*(ctx as *const AtomicUsize)).fetch_add(1, Ordering::SeqCst);
            let phrase = BOB_PHRASE.as_bytes();
            if phrase.len() + 1 > capacity {
                return mnemonic_resolver_result::BUFFER_TOO_SMALL;
            }
            std::ptr::copy_nonoverlapping(phrase.as_ptr() as *const c_char, out, phrase.len());
            *out.add(phrase.len()) = 0;
            *out_len = phrase.len();
            mnemonic_resolver_result::SUCCESS
        }
        unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}
        static SIGNATURES_ASKED: AtomicUsize = AtomicUsize::new(0);
        /// Counts the signature requests and refuses them: the reply is not
        /// sent, so the mock needs no broadcast answer.
        unsafe extern "C" fn count_and_refuse(
            _signer: *const c_void,
            _pubkey: *const u8,
            _pubkey_len: usize,
            _key_type: u8,
            _data: *const u8,
            _data_len: usize,
            completion_ctx: *mut c_void,
            completion: SignCompletionCallback,
        ) {
            SIGNATURES_ASKED.fetch_add(1, Ordering::SeqCst);
            completion(
                completion_ctx,
                std::ptr::null(),
                0,
                1,
                c"no signature in this test".as_ptr(),
            );
        }
        unsafe extern "C" fn can_sign(
            _signer: *const c_void,
            _pubkey: *const u8,
            _pubkey_len: usize,
            _key_type: u8,
        ) -> bool {
            true
        }

        let alice = identity(ALICE_PHRASE, [0xA1; 32]);
        let bob = identity(BOB_PHRASE, [0xB1; 32]);
        let incoming = public_request(&alice, &bob, 3);

        let mut outcomes = Vec::new();
        for adopt in [true, false] {
            let platform_version = PlatformVersion::latest();
            let contract = Arc::new(
                dpp::system_data_contracts::load_system_data_contract(
                    dpp::data_contracts::SystemDataContract::Dashpay,
                    platform_version,
                )
                .expect("bundled DashPay"),
            );
            let mut sdk = dash_sdk::SdkBuilder::new_mock()
                .with_network(key_wallet::Network::Testnet)
                .with_version(platform_version)
                .with_context_provider(DashpayProvider(Arc::clone(&contract)))
                .build()
                .expect("mock sdk");
            runtime().block_on(async {
                sdk.mock()
                    .expect_fetch(contract.id(), Some((*contract).clone()))
                    .await
                    .expect("DashPay expectation");
                for known in [&alice, &bob] {
                    sdk.mock()
                        .expect_fetch(known.id(), Some((*known).clone()))
                        .await
                        .expect("identity expectation");
                }
                sdk.mock()
                    .expect_fetch(
                        PrivateDocumentEntriesQuery::new(
                            contract.id(),
                            PRIVATE_CONTACT_REQUEST_DOCUMENT_TYPE_NAME,
                            0,
                        ),
                        Some(PrivateDocumentEntries {
                            chunk_power: None,
                            entries: Vec::new(),
                            total_count: 0,
                        }),
                    )
                    .await
                    .expect("store page expectation");
                // A public reply takes Bob's DashPay contract nonce before it
                // is signed.
                sdk.mock()
                    .expect_fetch::<dash_sdk::query_types::IdentityContractNonceFetcher, _>(
                        (bob.id(), contract.id()),
                        None,
                    )
                    .await
                    .expect("contract nonce expectation");
            });

            let persistence = PersistenceCallbacks {
                on_changeset_begin_fn: Some(begin),
                on_changeset_end_fn: Some(end),
                ..Default::default()
            };
            let events = EventHandlerCallbacks {
                context: std::ptr::null_mut(),
                on_wallet_event_fn: None,
                on_error_fn: None,
                on_platform_address_sync_completed_fn: None,
                on_shielded_sync_completed_fn: None,
                on_shielded_sync_progress_fn: None,
                on_shielded_tree_progress_fn: None,
                release_fn: None,
            };
            let mut manager = NULL_HANDLE;
            let created = unsafe {
                platform_wallet_manager_create(
                    &sdk as *const dash_sdk::Sdk as *const c_void,
                    &persistence,
                    &events,
                    &mut manager,
                )
            };
            assert_eq!(created.code, PlatformWalletFFIResultCode::Success);
            let mnemonic = CString::new(BOB_PHRASE).expect("phrase");
            let mut wallet_handle = NULL_HANDLE;
            let mut wallet_id = [0u8; 32];
            // Default accounts: the seed check verifies the resolver against
            // BIP44 account 0.
            let added = unsafe {
                platform_wallet_manager_create_wallet_from_mnemonic(
                    manager,
                    mnemonic.as_ptr(),
                    FFINetwork::Testnet,
                    1,
                    &mut wallet_handle,
                    &mut wallet_id,
                )
            };
            assert_eq!(added.code, PlatformWalletFFIResultCode::Success);

            // Bob is the wallet's identity, holding Alice's public request,
            // and, to adopt, the reply already sent to her.
            let reply = public_request(&bob, &alice, 4);
            PLATFORM_WALLET_STORAGE
                .with_item(wallet_handle, |wallet| {
                    runtime().block_on(async {
                        let mut state = wallet.state_mut().await;
                        let manager = &mut state.identity_manager;
                        manager
                            .add_identity(bob.clone(), 0, wallet.wallet_id(), wallet.persister())
                            .expect("Bob");
                        let managed = manager.managed_identity_mut(&bob.id()).expect("Bob");
                        managed
                            .add_incoming_contact_request(incoming.clone(), wallet.persister())
                            .expect("Alice's request");
                        if adopt {
                            managed
                                .add_sent_contact_request(reply.clone(), wallet.persister())
                                .expect("the reply");
                        }
                    })
                })
                .expect("the wallet");

            let reads = AtomicUsize::new(0);
            let resolver = unsafe {
                dash_sdk_mnemonic_resolver_create(
                    &reads as *const AtomicUsize as *mut c_void,
                    bob_resolve,
                    noop_destroy,
                )
            };
            let signer = unsafe { dash_sdk_signer_create(count_and_refuse, can_sign, None) };
            let request_handle = CONTACT_REQUEST_STORAGE.insert(incoming.clone());
            let signatures_before = SIGNATURES_ASKED.load(Ordering::SeqCst);
            let mut established = NULL_HANDLE;
            let result = unsafe {
                platform_wallet_manager_accept_contact_request_replying_in_kind(
                    manager,
                    wallet_id.as_ptr(),
                    resolver,
                    0,
                    request_handle,
                    signer,
                    &mut established,
                )
            };
            let message = if result.message.is_null() {
                String::new()
            } else {
                unsafe { std::ffi::CStr::from_ptr(result.message) }
                    .to_string_lossy()
                    .into_owned()
            };
            outcomes.push((
                adopt,
                result.code,
                message,
                established,
                SIGNATURES_ASKED.load(Ordering::SeqCst) - signatures_before,
                reads.load(Ordering::SeqCst),
            ));

            if established != NULL_HANDLE {
                ESTABLISHED_CONTACT_STORAGE.remove(established);
            }
            let _ = contact_request_destroy(request_handle);
            unsafe {
                dash_sdk_signer_destroy(signer);
                dash_sdk_mnemonic_resolver_destroy(resolver);
            }
            let _ = unsafe { platform_wallet_manager_destroy(manager) };
        }

        let (_, adopted_code, adopted_message, adopted, adopted_signatures, _) = &outcomes[0];
        assert_eq!(
            *adopted_code,
            PlatformWalletFFIResultCode::Success,
            "the reply already sent is adopted: {adopted_message}"
        );
        assert_ne!(
            *adopted, NULL_HANDLE,
            "an established contact is handed out"
        );
        assert_eq!(*adopted_signatures, 0, "nothing is signed to adopt");

        let (_, replied_code, replied_message, replied, replied_signatures, replied_reads) =
            &outcomes[1];
        assert_ne!(*replied_code, PlatformWalletFFIResultCode::Success);
        assert_eq!(*replied, NULL_HANDLE);
        assert!(
            !replied_message.contains("not configured"),
            "no payment resource was asked for: {replied_message}"
        );
        assert!(
            *replied_signatures > 0,
            "the public reply reaches the identity signer: {replied_message}"
        );
        assert!(*replied_reads > 0, "the DashPay crypto reads the mnemonic");
    }

    /// A wallet whose identity is paying for nothing to a contact reads the
    /// pair as free, no public request to it as unconfirmed, and has no
    /// restoration failure to give; an unknown wallet
    /// is not found and an identity the wallet does not hold is code 7, each
    /// leaving the output reset; a null output is refused. Holding a pair is the wallet's own
    /// (its tests pin the reservation and the journal mirror), so this pins
    /// the reads.
    #[test]
    fn should_read_a_pair_s_hold_and_the_restore_failure_from_the_wallet() {
        use crate::runtime::runtime;
        use crate::test_fixtures::mock_manager;
        use dpp::identity::{Identity, IdentityV0};
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;

        let sender_id = [0xB0u8; 32];
        let recipient_id = [0xA0u8; 32];
        let manager = mock_manager();
        let wallet = runtime()
            .block_on(manager.create_wallet_from_seed_bytes(
                key_wallet::Network::Testnet,
                &[11; 64],
                WalletAccountCreationOptions::Default,
                Some(0),
            ))
            .expect("a wallet");
        let identity = Identity::V0(IdentityV0 {
            id: sender_id.into(),
            public_keys: Default::default(),
            balance: 0,
            revision: 0,
        });
        runtime().block_on(async {
            let mut state = wallet.state_mut().await;
            state
                .identity_manager
                .add_identity(identity, 0, wallet.wallet_id(), wallet.persister())
                .expect("the identity");
        });
        let handle = PLATFORM_WALLET_STORAGE.insert(wallet);

        let mut held = true;
        let free = unsafe {
            platform_wallet_get_private_pair_held(
                handle,
                sender_id.as_ptr(),
                recipient_id.as_ptr(),
                &mut held,
            )
        };
        assert_eq!(free.code, PlatformWalletFFIResultCode::Success);
        assert!(!held, "nothing holds the pair");

        let mut unconfirmed = true;
        let none = unsafe {
            platform_wallet_get_public_contact_request_unconfirmed(
                handle,
                sender_id.as_ptr(),
                recipient_id.as_ptr(),
                &mut unconfirmed,
            )
        };
        assert_eq!(none.code, PlatformWalletFFIResultCode::Success);
        assert!(
            !unconfirmed,
            "no public request to the pair awaits its outcome"
        );
        let mut unconfirmed = true;
        let unknown_wallet = unsafe {
            platform_wallet_get_public_contact_request_unconfirmed(
                Handle::MAX,
                sender_id.as_ptr(),
                recipient_id.as_ptr(),
                &mut unconfirmed,
            )
        };
        assert_eq!(unknown_wallet.code, PlatformWalletFFIResultCode::NotFound);
        assert!(!unconfirmed, "reset");
        let mut unconfirmed = true;
        let unknown_sender = unsafe {
            platform_wallet_get_public_contact_request_unconfirmed(
                handle,
                [0xC1u8; 32].as_ptr(),
                recipient_id.as_ptr(),
                &mut unconfirmed,
            )
        };
        assert_eq!(
            unknown_sender.code,
            PlatformWalletFFIResultCode::ErrorIdentityNotFound
        );
        assert!(!unconfirmed);
        assert_eq!(
            unsafe {
                platform_wallet_get_public_contact_request_unconfirmed(
                    handle,
                    sender_id.as_ptr(),
                    recipient_id.as_ptr(),
                    std::ptr::null_mut(),
                )
            }
            .code,
            PlatformWalletFFIResultCode::ErrorNullPointer
        );

        let mut reason: *mut c_char = std::ptr::dangling_mut();
        let none = unsafe {
            platform_wallet_get_private_payments_restore_failure(
                handle,
                sender_id.as_ptr(),
                &mut reason,
            )
        };
        assert_eq!(none.code, PlatformWalletFFIResultCode::Success);
        assert!(reason.is_null(), "no restoration failed");

        let mut held = true;
        let unknown_wallet = unsafe {
            platform_wallet_get_private_pair_held(
                Handle::MAX,
                sender_id.as_ptr(),
                recipient_id.as_ptr(),
                &mut held,
            )
        };
        assert_eq!(unknown_wallet.code, PlatformWalletFFIResultCode::NotFound);
        assert!(!held, "reset");
        let mut reason: *mut c_char = std::ptr::dangling_mut();
        let unknown_wallet = unsafe {
            platform_wallet_get_private_payments_restore_failure(
                Handle::MAX,
                sender_id.as_ptr(),
                &mut reason,
            )
        };
        assert_eq!(unknown_wallet.code, PlatformWalletFFIResultCode::NotFound);
        assert!(reason.is_null(), "reset");

        let stranger = [0xC0u8; 32];
        let mut held = true;
        let unknown_sender = unsafe {
            platform_wallet_get_private_pair_held(
                handle,
                stranger.as_ptr(),
                recipient_id.as_ptr(),
                &mut held,
            )
        };
        assert_eq!(
            unknown_sender.code,
            PlatformWalletFFIResultCode::ErrorIdentityNotFound
        );
        assert!(!held);
        let mut reason: *mut c_char = std::ptr::dangling_mut();
        let unknown_identity = unsafe {
            platform_wallet_get_private_payments_restore_failure(
                handle,
                stranger.as_ptr(),
                &mut reason,
            )
        };
        assert_eq!(
            unknown_identity.code,
            PlatformWalletFFIResultCode::ErrorIdentityNotFound
        );
        assert!(reason.is_null(), "reset");

        assert_eq!(
            unsafe {
                platform_wallet_get_private_pair_held(
                    handle,
                    sender_id.as_ptr(),
                    recipient_id.as_ptr(),
                    std::ptr::null_mut(),
                )
            }
            .code,
            PlatformWalletFFIResultCode::ErrorNullPointer
        );
        assert_eq!(
            unsafe {
                platform_wallet_get_private_payments_restore_failure(
                    handle,
                    sender_id.as_ptr(),
                    std::ptr::null_mut(),
                )
            }
            .code,
            PlatformWalletFFIResultCode::ErrorNullPointer
        );

        let _ = PLATFORM_WALLET_STORAGE.remove(handle);
    }

    #[test]
    fn should_carry_every_field_of_a_private_contact() {
        let address = [0x55; SHIELDED_ADDRESS_LEN];
        let with_address = PrivateContactFFI::from(PrivateContact {
            received_privately: true,
            sent_privately: false,
            shielded_address: Some(address),
        });
        assert!(with_address.received_privately);
        assert!(!with_address.sent_privately);
        assert!(with_address.has_shielded_address);
        assert_eq!(with_address.shielded_address, address);

        let without_address = PrivateContactFFI::from(PrivateContact {
            received_privately: false,
            sent_privately: true,
            shielded_address: None,
        });
        assert!(without_address.sent_privately);
        assert!(!without_address.has_shielded_address);
        assert_eq!(without_address.shielded_address, [0; SHIELDED_ADDRESS_LEN]);
    }

    #[test]
    fn should_report_no_private_contact_for_an_unknown_identity_handle() {
        let mut contact = PrivateContactFFI {
            received_privately: true,
            sent_privately: true,
            has_shielded_address: true,
            shielded_address: [0x55; SHIELDED_ADDRESS_LEN],
        };
        let contact_id = [7u8; 32];
        let result = unsafe {
            managed_identity_get_private_contact(0xDEAD_BEEF, contact_id.as_ptr(), &mut contact)
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::NotFound);
        assert_eq!(contact, PrivateContactFFI::empty());
    }
}
