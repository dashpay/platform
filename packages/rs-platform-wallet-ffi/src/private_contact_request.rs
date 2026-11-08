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
//!
//! The explicitly public send and accept refuse a contact whose requests came
//! or went privately with `ErrorPrivateContactRequestNeedsPrivateReply`.

use std::sync::{Arc, Mutex};

use platform_wallet::{PlatformWalletError, PrivateContact};
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::contact_request::CONTACT_REQUEST_STORAGE;
use crate::dashpay::{resolver_contact_crypto_provider, ContactRequestHandleArray};
use crate::error::*;
use crate::established_contact::ESTABLISHED_CONTACT_STORAGE;
use crate::handle::*;
use crate::runtime::{block_on_worker, run_on_big_stack_thread, runtime};
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

#[cfg(not(feature = "shielded"))]
use dash_sdk::dapi_grpc::platform::v0::ResponseMetadata;

#[cfg(feature = "shielded")]
use std::ffi::CStr;
#[cfg(feature = "shielded")]
use std::os::raw::c_char;
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
/// its sender's keys, which needs no secret. The cursors live in memory only,
/// so the first pass of a process reads the store from its start.
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
    // `run_on_big_stack_thread` rather than `block_on_worker`: the pass's
    // future trips rustc's implied-lifetime-bound limitation (rust-lang/rust
    // issue #100013) against `block_on_worker`'s `Send + 'static` bounds,
    // through the SDK's identity fetch stream.
    let spawned = run_on_big_stack_thread(|| {
        runtime().block_on(
            identity
                .dashpay()
                .sync_private_contact_requests(&provider, max_pages as usize),
        )
    });
    let result = match spawned {
        Ok(result) => result,
        Err(error) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorWalletOperation,
                format!("failed to spawn big-stack thread for the private contact sync: {error}"),
            );
        }
    };
    let report = unwrap_result_or_return!(result);
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

/// Copy an optional Orchard address: null means none.
///
/// # Safety
/// A non-null `ptr` must point to [`SHIELDED_ADDRESS_LEN`] readable bytes.
unsafe fn read_shielded_address(ptr: *const u8) -> Option<[u8; SHIELDED_ADDRESS_LEN]> {
    if ptr.is_null() {
        return None;
    }
    let mut address = [0u8; SHIELDED_ADDRESS_LEN];
    std::ptr::copy_nonoverlapping(ptr, address.as_mut_ptr(), SHIELDED_ADDRESS_LEN);
    Some(address)
}

/// Send a DashPay private contact request from `sender_identity_id` to
/// `recipient_identity_id`, paying for it from `account`'s shielded notes, so
/// nothing on chain names either identity. The request carries our receiving
/// xpub for the contact, the masked account reference, `account_label`,
/// `auto_accept_proof`, and `shielded_address`, the Orchard address the
/// contact may pay us at privately. It is sealed to the recipient's DashPay
/// key inside the resolver-backed signer.
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
/// - `shielded_address`, when non-null, must point to 43 readable bytes;
///   null omits it.
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
    shielded_address: *const u8,
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
            shielded_address,
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
    shielded_address: *const u8,
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
    let shielded_address = read_shielded_address(shielded_address);

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
                shielded_address,
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
        Err(error) => map_spend_result(Err(error), "private contact request"),
    }
}

/// Accept a DashPay contact request that came privately, replying privately
/// and paying for the reply from `account`'s shielded notes, as
/// [`platform_wallet_manager_send_private_contact_request`] pays.
/// `shielded_address` is the Orchard address the contact may pay us at
/// privately; null omits it.
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
/// - `shielded_address`, when non-null, must point to 43 readable bytes.
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
    shielded_address: *const u8,
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
            shielded_address,
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
    shielded_address: *const u8,
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
    let shielded_address = read_shielded_address(shielded_address);

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
                shielded_address,
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
        Err(error) => map_spend_result(Err(error), "private contact request accept"),
    }
}

/// Accept a DashPay contact request replying the way it came
/// (`accept_contact_request_replying_in_kind`): privately, paid from
/// `account`'s shielded notes, when it came privately and no reply exists
/// yet; otherwise with a `contactRequest` document signed by `signer_handle`,
/// or by adopting the reply already sent. Which reply a request takes is the
/// wallet's decision: this is the one accept a host calls. `shielded_address`
/// is the Orchard address the contact may pay us at privately; null omits it,
/// and a public reply ignores it. Exported in every build.
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
/// - `shielded_address`, when non-null, must point to 43 readable bytes.
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
    shielded_address: *const u8,
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
            shielded_address,
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
    shielded_address: *const u8,
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
    let shielded_address = read_shielded_address(shielded_address);

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
                .accept_contact_request_replying_in_kind(
                    &request,
                    shielded_address,
                    signer,
                    &crypto,
                    || async {
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
                    },
                )
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
                    shielded_address,
                    signer,
                    &crypto,
                    |_entry| async move {
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
                std::ptr::null(),
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
                std::ptr::null(),
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
                std::ptr::null(),
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
                std::ptr::null(),
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
