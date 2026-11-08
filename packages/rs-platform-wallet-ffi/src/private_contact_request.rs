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
//!   that came privately, and with a document otherwise. Built with the
//!   `shielded` feature only.
//! - [`managed_identity_get_private_contact`]: what a contact's private
//!   requests carry beyond the `ContactRequest`: whether they came or went
//!   privately, and the contact's shielded address.
//!
//! The explicitly public send and accept refuse a contact whose requests came
//! or went privately with `ErrorPrivateContactRequestNeedsPrivateReply`.

use platform_wallet::PrivateContact;
use rs_sdk_ffi::MnemonicResolverHandle;
#[cfg(feature = "shielded")]
use rs_sdk_ffi::{SignerHandle, VTableSigner};

use crate::dashpay::{resolver_contact_crypto_provider, ContactRequestHandleArray};
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::types::*;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

#[cfg(feature = "shielded")]
use std::ffi::CStr;
#[cfg(feature = "shielded")]
use std::os::raw::c_char;

#[cfg(feature = "shielded")]
use platform_wallet::wallet::shielded::CachedOrchardProver;
#[cfg(feature = "shielded")]
use platform_wallet::AutoAcceptProofSource;

#[cfg(feature = "shielded")]
use crate::contact_request::CONTACT_REQUEST_STORAGE;
#[cfg(feature = "shielded")]
use crate::dashpay::read_auto_accept_proof;
#[cfg(feature = "shielded")]
use crate::established_contact::ESTABLISHED_CONTACT_STORAGE;
#[cfg(feature = "shielded")]
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
#[cfg(feature = "shielded")]
use crate::shielded_send::{
    catch_spend_panic, map_spend_result, poke_sync_on_unconfirmed, resolve_wallet_and_coordinator,
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
    let result = block_on_worker(async move {
        identity
            .dashpay()
            .sync_private_contact_requests(&provider, max_pages as usize)
            .await
    });
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
#[cfg(feature = "shielded")]
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
/// (`PlatformWallet::accept_contact_request_replying_in_kind`): privately,
/// paid from `account`'s shielded notes, when it came privately and no reply
/// exists yet; otherwise with a `contactRequest` document signed by
/// `signer_handle`, or by adopting the reply already sent. Which reply a
/// request takes is the wallet's decision: this is the one accept a host
/// calls. `shielded_address` is the Orchard address the contact may pay us
/// at privately; null omits it, and a public reply ignores it.
///
/// Arguments and outputs are as for
/// [`platform_wallet_manager_accept_private_contact_request`]. The spend
/// authority is derived only for a private reply. Errors follow the shielded
/// spend contract: `ErrorShieldedSpendUnconfirmed` means the reply may
/// already be on chain; do not accept again.
///
/// # Safety
/// Identical contract to
/// [`platform_wallet_manager_accept_private_contact_request`].
#[cfg(feature = "shielded")]
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
    // Same panic contract as the private accept: a private reply's notes may
    // be spent when it strikes.
    catch_spend_panic("contact request accept", || {
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
    })
}

/// Body of [`platform_wallet_manager_accept_contact_request_replying_in_kind`],
/// as an ordinary Rust function so a panic unwinds into [`catch_spend_panic`].
///
/// # Safety
/// Identical contract to the export that calls it.
#[cfg(feature = "shielded")]
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
        // SAFETY: as in the private accept, the caller pins the signer for the
        // call.
        let signer: &VTableSigner = &*(signer_addr as *const VTableSigner);
        let prover = CachedOrchardProver::new();
        let result = wallet
            .accept_contact_request_replying_in_kind(
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
        Err(error) => map_spend_result(Err(error), "contact request accept"),
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
    #[cfg(feature = "shielded")]
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
