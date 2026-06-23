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
                .send_contact_request_with_external_signer(
                    &sender, &recipient, label, proof, signer, &provider,
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
        block_on_worker(async move { identity.ignore_contact_sender(&our_id, &contact_id).await })
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
        block_on_worker(async move { identity.unignore_contact_sender(&our_id, &contact_id).await })
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
        block_on_worker(async move {
            identity
                .send_payment(&from_id, &to_id, amount_duffs, memo_str, &signer)
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
        use key_wallet::signer::Signer;
        self.signer
            .extended_public_key(path)
            .await
            .map_err(|e| platform_wallet::PlatformWalletError::InvalidIdentityData(e.to_string()))
    }

    async fn ecdh_shared_secret(
        &self,
        path: &key_wallet::bip32::DerivationPath,
        peer: &dashcore::secp256k1::PublicKey,
    ) -> Result<[u8; 32], platform_wallet::PlatformWalletError> {
        self.signer
            .ecdh_shared_secret(path, peer)
            .map(|z| *z)
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
            .contact_info_open(root_path, derivation_index, enc_to_user_id, private_data_blob)
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
/// signer-present DashPay action). Writes the number of completed entries to
/// `out_drained`.
///
/// # Safety
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle`; ownership is retained by the caller.
/// - `out_drained` must be a valid `*mut u32`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_drain_pending_contact_crypto(
    wallet_handle: Handle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_drained: *mut u32,
) -> PlatformWalletFFIResult {
    check_ptr!(core_signer_handle);
    check_ptr!(out_drained);

    let signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        // SAFETY: same lifetime contract as platform_wallet_send_dashpay_payment —
        // the caller pins the resolver handle for the duration of this call.
        let provider = unsafe {
            resolver_contact_crypto_provider(
                signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        block_on_worker(async move { identity.drain_pending_contact_crypto(&provider).await })
    });
    let drained = unwrap_option_or_return!(option);
    unsafe {
        *out_drained = drained as u32;
    }
    PlatformWalletFFIResult::ok()
}
