//! Identity top-up for an *existing* identity — two funding sources:
//!
//! - [`platform_wallet_top_up_from_addresses_with_signer`] wraps the
//!   composite
//!   [`PlatformWallet::top_up_from_addresses`](platform_wallet::PlatformWallet::top_up_from_addresses),
//!   spending already-funded Platform-payment addresses (driven by an
//!   external `Signer<PlatformAddress>` handle) and reusing the same
//!   address-input shape (`IdentityFundingInputFFI`) the registration FFI
//!   exposes.
//! - [`platform_wallet_top_up_identity_with_funding_signer`] wraps
//!   `IdentityWallet::top_up_identity_with_funding`, building and
//!   broadcasting a **new Core asset lock** (same mechanism as identity
//!   registration), driven by a Core-side `MnemonicResolverHandle`.
//!
//! The address path's top-up state-transitions are signed entirely with
//! the Platform address inputs' private keys (the SDK uses
//! `BalanceTransfer` to credit the existing identity), so that FFI takes a
//! single `SignerHandle` — `signer_address_handle` — used as
//! `Signer<PlatformAddress>`. Neither path needs an identity-key signer
//! (existing identity, no IdentityCreate to sign); the asset-lock path is
//! signed by the lock's Core-side key via the `MnemonicResolver`.
//!
//! On success the function writes the post-transition credit balance
//! back through `out_new_balance`. The local `ManagedIdentity`
//! manager is updated and the spent platform-address balances are
//! reconciled synchronously inside the composite library call;
//! callers can re-read the balance via `ManagedIdentity` once this
//! returns.

use std::slice;
use std::sync::{Arc, Mutex};

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use platform_wallet::AssetLockFunding;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::identity_registration::IdentityFundingInputFFI;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

#[cfg(test)]
mod external_identity_tests;
#[cfg(test)]
mod registry_release_tests;
#[cfg(test)]
mod submission_result_tests;

/// What a host should know after an address top-up export panicked.
const ADDRESS_TOP_UP_PANIC_GUIDANCE: &str = "The top-up may have been signed and broadcast \
     and may still execute: do NOT submit it again. A later sync refreshes balances but \
     does not confirm this top-up: the wallet keeps no record of it.";

/// [`catch_panic_to_code`] for the address top-up exports. A panic can strike
/// in the identity lookup, the signing, after the broadcast or in the
/// reconciliation, so it is reported as
/// [`PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed`],
/// the code of a top-up whose execution is not proved, never as a definitive
/// unsent failure.
fn catch_address_top_up_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        ADDRESS_TOP_UP_PANIC_GUIDANCE,
        body,
    )
}

/// Top up an existing identity's credit balance from one or more
/// Platform addresses, using an external `SignerHandle` for the
/// per-address funding signatures.
///
/// On success `out_new_balance` is populated with the post-transition
/// credit balance returned by Platform. The Rust-side
/// `ManagedIdentity` for `identity_id` has its balance updated and
/// queued for persistence inside the library call, so a subsequent
/// `ManagedIdentity::balance` read on the same handle reflects the
/// new value without an extra round trip.
///
/// The identity's balance is read from Platform before anything is signed:
/// a failed read is `ErrorIdentityLookupFailed` (80) and no balance from
/// Platform `ErrorIdentityBalanceUnavailable` (58), nothing sent.
///
/// `ErrorTransactionBroadcastUnconfirmed` (20) means the top-up may have
/// executed or may still execute: its result was not proved, or the call
/// panicked, possibly after signing or broadcast. A result proof counts only
/// when it shows every input address at or past the nonce this top-up was
/// signed with AND the identity balance above the balance the top-up started
/// from, a fresh read from Platform taken before signing; any other proof is
/// 20 and nothing is reconciled from it. So is a broadcast an endpoint
/// refused once a dispatch may have reached it: the refusal is no proof that
/// the signed top-up was left out of every mempool, so an address-nonce
/// refusal is 20 too, and this export never returns
/// `ErrorAddressNonceMismatch` (21). The host must not submit it again and
/// keeps it pending. Nothing in the wallet holds its input addresses yet: a
/// top-up submitted again reads a fresh nonce and is signed and broadcast,
/// so once this one executed it is a second payment (a durable,
/// execution-bound record of the pending top-up is dashpay/platform#4270).
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must point at a 32-byte identity id buffer for
///   the duration of the call.
/// - `inputs` must point at a valid `[IdentityFundingInputFFI;
///   inputs_count]` array; each row's `hash` is a 20-byte address
///   hash. It may be null when `inputs_count` is 0: no rows is
///   `ErrorInvalidParameter`, refused by the library.
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*mut SignerHandle` produced by `dash_sdk_signer_create_with_ctx`.
///   The caller retains ownership; this function does NOT destroy it.
/// - `out_new_balance` must be writable for the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_top_up_from_addresses_with_signer(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    inputs: *const IdentityFundingInputFFI,
    inputs_count: usize,
    signer_address_handle: *mut SignerHandle,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    catch_address_top_up_panic("platform_wallet_top_up_from_addresses_with_signer", || {
        check_ptr!(identity_id);
        check_ptr!(signer_address_handle);
        check_ptr!(out_new_balance);
        let identity_id_bytes: [u8; 32] = *identity_id;
        let identity_id = Identifier::from_bytes(&identity_id_bytes).unwrap_or_default();
        let rows = match funding_rows(inputs, inputs_count) {
            Ok(rows) => rows,
            Err(error) => return error,
        };

        // Round-trip the signer pointer through `usize` so the spawned
        // future's capture is `Send + 'static` (the raw pointer is `!Send`,
        // but `usize` is). The underlying `Inner::Callback { ctx, vtable }`
        // is `Send + Sync` — see the unsafe impls in `rs-sdk-ffi/src/signer.rs`.
        let signer_addr = signer_address_handle as usize;

        // Take the wallet out of the registry in a short read, so the registry
        // is not locked while the host signs.
        let wallet = unwrap_option_or_return!(
            PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |w| w.clone())
        );
        let result = block_on_worker(async move {
            let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            // The composite tops up the identity AND reconciles the spent
            // platform-address balances from the proof, so the wallet's
            // displayed balance and next input selection reflect the spend
            // (covering addresses restored from disk that are no longer in
            // a live derived pool).
            wallet
                .top_up_from_addresses(&identity_id, rows, address_signer, None)
                .await
        });
        let new_balance = unwrap_result_or_return!(result);
        *out_new_balance = new_balance;
        PlatformWalletFFIResult::ok()
    })
}

/// Marshal the address-input rows of an address-funded top-up into the
/// funding rows the library takes; the library sums rows naming the same
/// address and refuses what it cannot act on, no rows at all included.
/// An empty list may be passed as `(NULL, 0)`; a null `inputs` with a
/// nonzero count is `ErrorNullPointer`.
///
/// # Safety
/// `inputs` must point at a valid `[IdentityFundingInputFFI; inputs_count]`
/// array, or be null when `inputs_count` is 0.
unsafe fn funding_rows(
    inputs: *const IdentityFundingInputFFI,
    inputs_count: usize,
) -> Result<Vec<(PlatformAddress, Credits)>, PlatformWalletFFIResult> {
    if inputs_count == 0 {
        return Ok(Vec::new());
    }
    if inputs.is_null() {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorNullPointer,
            "inputs ptr is null",
        ));
    }
    slice::from_raw_parts(inputs, inputs_count)
        .iter()
        .map(|entry| match entry.address_type {
            0 => Ok((PlatformAddress::P2pkh(entry.hash), entry.credits)),
            1 => Ok((PlatformAddress::P2sh(entry.hash), entry.credits)),
            _ => Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "invalid address_type (expected 0 or 1)",
            )),
        })
        .collect()
}

/// Top up an identity this wallet does **not** manage (someone else's)
/// from one or more Platform addresses, using an external `SignerHandle`
/// for the per-address funding signatures.
///
/// Wraps the composite
/// [`PlatformWallet::top_up_external_identity_from_addresses`](platform_wallet::PlatformWallet::top_up_external_identity_from_addresses):
/// the identity is fetched from Platform rather than read from the wallet,
/// is never added to the wallet's identity manager, and the spent
/// platform-address balances are reconciled as for
/// [`platform_wallet_top_up_from_addresses_with_signer`].
///
/// On success `out_new_balance` is populated with the identity's
/// post-transition credit balance returned by Platform. An identity
/// Platform does not know returns `ErrorIdentityNotFound` (7) before anything
/// is signed or broadcast, and a lookup that fails (a transport or proof
/// failure, not Platform's answer) `ErrorIdentityLookupFailed` (80), likewise
/// with nothing signed or broadcast, so it may be retried as it is.
///
/// `ErrorTransactionBroadcastUnconfirmed` (20) means the top-up may have
/// executed or may still execute: its result was not proved, or the call
/// panicked, possibly after signing or broadcast. A result proof counts only
/// when it shows every input address at or past the nonce this top-up was
/// signed with AND the identity balance above the balance the top-up started
/// from, a fresh read from Platform taken before signing; any other proof is
/// 20 and nothing is reconciled from it, and a refused broadcast reads as
/// for [`platform_wallet_top_up_from_addresses_with_signer`] (20, never
/// `ErrorAddressNonceMismatch` (21)). The host must not submit it again and
/// keeps it pending; as on that export, nothing in the wallet holds its
/// input addresses yet (dashpay/platform#4270).
///
/// # Safety
/// Same contract as [`platform_wallet_top_up_from_addresses_with_signer`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_top_up_external_identity_from_addresses_with_signer(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    inputs: *const IdentityFundingInputFFI,
    inputs_count: usize,
    signer_address_handle: *mut SignerHandle,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    catch_address_top_up_panic(
        "platform_wallet_top_up_external_identity_from_addresses_with_signer",
        || {
            check_ptr!(identity_id);
            check_ptr!(signer_address_handle);
            check_ptr!(out_new_balance);
            let identity_id_bytes: [u8; 32] = *identity_id;
            let identity_id = Identifier::from_bytes(&identity_id_bytes).unwrap_or_default();
            let rows = match funding_rows(inputs, inputs_count) {
                Ok(rows) => rows,
                Err(error) => return error,
            };

            // `usize` round-trip keeps the spawned future `Send + 'static`; see
            // `platform_wallet_top_up_from_addresses_with_signer`.
            let signer_addr = signer_address_handle as usize;

            let wallet = unwrap_option_or_return!(
                PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |w| w.clone())
            );
            let result = block_on_worker(async move {
                let address_signer: &VTableSigner =
                    unsafe { &*(signer_addr as *const VTableSigner) };
                wallet
                    .top_up_external_identity_from_addresses(
                        &identity_id,
                        rows,
                        address_signer,
                        None,
                    )
                    .await
            });
            let new_balance = unwrap_result_or_return!(result);
            *out_new_balance = new_balance;
            PlatformWalletFFIResult::ok()
        },
    )
}

/// Top up an existing identity's credit balance by building and
/// broadcasting a **new Core asset lock** (the same funding mechanism as
/// identity registration), distinct from
/// [`platform_wallet_top_up_from_addresses_with_signer`] which spends
/// already-funded Platform-payment addresses.
///
/// Wraps
/// [`IdentityWallet::top_up_identity_with_funding`](platform_wallet::wallet::identity::network::registration)
/// with [`AssetLockFunding::FromWalletBalance`] — the same L2 orchestrator
/// (funding resolution, IS→CL fallback, asset-lock cleanup) that
/// [`platform_wallet_register_identity_with_funding_signer`] drives for
/// registration. `account_index` addresses the *standard* families: the
/// asset lock POOLS the BIP44 and BIP32 accounts at that index together
/// with every DashPay receiving account (change returns to BIP44). The
/// index does not restrict which DashPay receiving accounts contribute —
/// callers must not present this as an account-scoped funding or privacy
/// choice (matching registration).
///
/// Unlike registration this takes NO identity-key signer: the
/// `IdentityTopUp` state-transition is signed entirely by the asset lock's
/// Core-side key, so only `core_signer_handle` (a
/// `*mut MnemonicResolverHandle`, reusing the Keychain-resolver vtable) is
/// required. On success `out_new_balance` receives the post-transition
/// credit balance Platform returns; the local `ManagedIdentity` balance is
/// updated + queued for persistence inside the library call.
///
/// `amount_duffs` must be at least Platform's top-up minimum, the largest
/// across the protocol versions this build supports
/// ([`min_top_up_duffs`](platform_wallet::wallet::identity::network::min_top_up_duffs),
/// 50_500 duffs today); the wallet rejects a smaller amount with
/// `ErrorInvalidParameter` before any lock is built.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must point at a 32-byte identity id buffer for the
///   duration of the call.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   [`crate::dash_sdk_mnemonic_resolver_create`]. The caller retains
///   ownership; this function does NOT destroy it.
/// - `out_new_balance` must be writable for the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_top_up_identity_with_funding_signer(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    amount_duffs: u64,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id);
    check_ptr!(core_signer_handle);
    check_ptr!(out_new_balance);
    // FFI-safe sentinel before any fallible work, matching the sibling
    // `platform_wallet_topup_identity_with_existing_asset_lock_signer`.
    *out_new_balance = 0;

    let identity_id_bytes: [u8; 32] = *identity_id;
    let identity_id = Identifier::from_bytes(&identity_id_bytes).unwrap_or_default();

    // Round-trip the handle through `usize` so the spawned future's
    // capture is `Send + 'static` — same pattern as the registration
    // FFI (raw pointers are `!Send`, `usize` isn't).
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let identity_wallet = wallet.identity().clone();
        let wallet_id = wallet.wallet_id();
        // Capture the network the asset-lock signer should derive under,
        // pulled from the wallet (mirrors the registration FFI).
        let network = wallet.sdk().network;
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — the handle is pinned
            // alive for the duration of this FFI call.
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            identity_wallet
                .top_up_identity_with_funding(
                    &identity_id,
                    AssetLockFunding::FromWalletBalance {
                        amount_duffs,
                        account_index,
                    },
                    &asset_lock_signer,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let new_balance = unwrap_result_or_return!(result);
    *out_new_balance = new_balance;
    PlatformWalletFFIResult::ok()
}

/// Top up someone else's identity — one this wallet does not manage — by
/// building and broadcasting a new Core asset lock from the wallet's
/// balance. The external counterpart of
/// [`platform_wallet_top_up_identity_with_funding_signer`].
///
/// Wraps
/// `IdentityWallet::top_up_external_identity_with_funding` with
/// `AssetLockFunding::FromWalletBalance`: the identity is fetched from
/// Platform, the lock is funded from the wallet's unbound top-up account and
/// its row records `identity_id` before the broadcast, so an interrupted
/// top-up resumes to this identity only
/// ([`crate::platform_wallet_resume_identity_top_up_to_recorded_recipient_signer`]).
/// No identity is added to the wallet (one the wallet already holds gets its
/// local balance updated). On success
/// `out_new_balance` receives the identity's proved post-transition balance.
///
/// Errors before anything is built: `ErrorInvalidParameter` for an amount
/// below [`min_top_up_duffs`](platform_wallet::wallet::identity::network::min_top_up_duffs)
/// (the largest top-up minimum across the supported protocol versions, 50_500
/// duffs today), `ErrorIdentityNotFound` (7) for an identity
/// Platform does not know, and a persistence error when the host's persister
/// does not declare `ASSET_LOCK_IDENTITY_RECIPIENTS`. A failed lookup is
/// reported as the SDK error for now. In every one of these
/// `out_asset_lock_outpoint` is all zero: nothing was built, nothing was
/// paid, and a new call is safe.
///
/// `out_asset_lock_outpoint` receives the outpoint of the lock this call
/// built, on success and on failure alike (a panic included), and is all zero
/// when nothing was built. The error code alone does not tell a failure
/// before the build from one after it (a failed lookup and a failed Platform
/// submit can share a code); the outpoint does. Once it is non-zero:
///
/// - Do not build a new lock to retry, or the identity may be paid twice.
///   Resume this one with
///   [`crate::platform_wallet_resume_identity_top_up_to_recorded_recipient_signer`],
///   which pays the identity its row records.
/// - `ErrorTransactionBroadcastRejected` (26) means Core definitively
///   rejected the lock and it was removed, with its inputs released: a new
///   call is safe.
/// - `ErrorTransactionBroadcastUnconfirmed` (20) means the lock may be on the
///   network, or its row may come back after a restart while its inputs stay
///   reserved: do not retry; resume this outpoint (after the next launch if
///   it is not tracked now).
/// - Any other failure: resume this outpoint. A resume answering
///   `ErrorAssetLockNotTracked` (23) means the lock was removed before it was
///   sent (its row could not be made durable, and its removal was), so
///   nothing was paid and a new call is safe.
///
/// # Safety
/// Same contract as [`platform_wallet_top_up_identity_with_funding_signer`],
/// and `out_asset_lock_outpoint` must be writable for the duration of the
/// call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_top_up_external_identity_with_funding_signer(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    amount_duffs: u64,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_new_balance: *mut u64,
    out_asset_lock_outpoint: *mut OutPointFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_asset_lock_outpoint);
    // The whole body runs under `catch_unwind`: a panic (most concretely
    // `block_on_worker`'s `.expect` on a panicking top-up task) must not reach
    // this `extern "C"` frame. It can strike after the lock was broadcast, so
    // the ambiguous broadcast-unconfirmed contract applies.
    report_built_outpoint(out_asset_lock_outpoint, |built| {
        catch_top_up_panic("external identity top-up", || {
            top_up_external_identity_with_funding_signer_inner(
                wallet_handle,
                identity_id,
                amount_duffs,
                account_index,
                core_signer_handle,
                out_new_balance,
                built,
            )
        })
    })
}

/// The outpoint a funding call's `on_built` observer reported, shared with
/// the export that writes it out once the call returns.
type BuiltOutPoint = Arc<Mutex<Option<dashcore::OutPoint>>>;

/// Zero `out`, run `body` with a slot its `on_built` observer fills, then
/// write the reported outpoint to `out` whatever `body` returned. `body`
/// catches its own panics, so the outpoint of a lock built before one is
/// still written.
///
/// # Safety
/// `out` must be non-null and writable for the duration of the call.
unsafe fn report_built_outpoint(
    out: *mut OutPointFFI,
    body: impl FnOnce(BuiltOutPoint) -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    *out = OutPointFFI {
        txid: [0; 32],
        vout: 0,
    };
    let built = BuiltOutPoint::default();
    let result = body(Arc::clone(&built));
    if let Some(out_point) = *built.lock().unwrap_or_else(|e| e.into_inner()) {
        *out = OutPointFFI::from(&out_point);
    }
    result
}

/// Body of [`platform_wallet_top_up_external_identity_with_funding_signer`],
/// as an ordinary Rust function so a panic unwinds into
/// [`catch_top_up_panic`] instead of across the C ABI.
///
/// # Safety
/// Identical contract to the export that calls it.
unsafe fn top_up_external_identity_with_funding_signer_inner(
    wallet_handle: Handle,
    identity_id: *const [u8; 32],
    amount_duffs: u64,
    account_index: u32,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_new_balance: *mut u64,
    built: BuiltOutPoint,
) -> PlatformWalletFFIResult {
    check_ptr!(identity_id);
    check_ptr!(core_signer_handle);
    check_ptr!(out_new_balance);
    *out_new_balance = 0;
    let identity_id = Identifier::from(*identity_id);
    let core_signer_addr = core_signer_handle as usize;

    // Read what the call needs out of the registry, then release it before
    // the top-up awaits the network and the host's signer and persister.
    let (identity_wallet, wallet_id, network) = unwrap_option_or_return!(PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            (
                wallet.identity().clone(),
                wallet.wallet_id(),
                wallet.sdk().network,
            )
        }));
    let result = block_on_worker(async move {
        // SAFETY: see the fn-level safety doc — the handle is pinned alive
        // for the duration of this FFI call.
        let asset_lock_signer = unsafe {
            MnemonicResolverCoreSigner::new(
                core_signer_addr as *mut MnemonicResolverHandle,
                wallet_id,
                network,
            )
        };
        // Runs before the lock's row is stored or the lock is broadcast, so
        // the outpoint is recorded for every failure after the build.
        let on_built = move |out_point: dashcore::OutPoint| {
            *built.lock().unwrap_or_else(|e| e.into_inner()) = Some(out_point);
        };
        identity_wallet
            .top_up_external_identity_with_funding(
                &identity_id,
                AssetLockFunding::FromWalletBalance {
                    amount_duffs,
                    account_index,
                },
                &asset_lock_signer,
                None,
                Some(&on_built),
            )
            .await
    });
    *out_new_balance = unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Post-panic guidance for the exports that may have paid an identity
/// top-up. Paired with `ErrorTransactionBroadcastUnconfirmed` in
/// [`catch_top_up_panic`].
const TOP_UP_PANIC_GUIDANCE: &str = "The asset lock may or may not have been broadcast and \
     the top-up may or may not have been submitted — do NOT build a new lock to retry; resume \
     the tracked lock with platform_wallet_resume_identity_top_up_to_recorded_recipient_signer, \
     or the identity may be paid twice.";

/// [`catch_panic_to_code`] for the exports that fund or resume an identity
/// top-up from a Core asset lock.
///
/// The panic is mapped to
/// [`PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed`], not
/// to a definitive failure: it can strike after the lock reached the wire or
/// after Platform accepted the top-up, so the outcome is ambiguous, and that
/// code's contract (do not retry, the lock stays tracked and resumable) is
/// the conservative one.
pub(crate) fn catch_top_up_panic(
    operation: &str,
    body: impl FnOnce() -> PlatformWalletFFIResult,
) -> PlatformWalletFFIResult {
    catch_panic_to_code(
        operation,
        PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
        TOP_UP_PANIC_GUIDANCE,
        body,
    )
}

#[cfg(test)]
mod top_up_with_funding_guard_tests {
    use super::*;
    use crate::error::PlatformWalletFFIResultCode;

    /// Any amount: every guard under test answers before the minimum check.
    const AMOUNT_DUFFS: u64 = 50_500;

    /// A non-null but never-dereferenced core-signer pointer. Every guard
    /// under test returns before the handle is used, so a dangling pointer
    /// is sufficient (and never unsound here).
    fn dangling_core_signer() -> *mut MnemonicResolverHandle {
        std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr()
    }

    /// A panic inside a top-up export must not unwind into the `extern "C"`
    /// frame. It becomes `ErrorTransactionBroadcastUnconfirmed`, the
    /// ambiguous do-not-retry contract, because it can strike after the lock
    /// was broadcast.
    #[test]
    fn should_map_a_top_up_panic_to_the_broadcast_unconfirmed_contract() {
        let mut result = catch_top_up_panic("external identity top-up", || {
            panic!("tokio worker panicked");
        });

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed
        );
        let message = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert!(
            message.contains("external identity top-up panicked")
                && message.contains("tokio worker panicked")
                && message.contains("do NOT build a new lock"),
            "{message}"
        );
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }

    #[test]
    fn rejects_null_identity_id() {
        let mut balance = 0u64;
        let res = unsafe {
            platform_wallet_top_up_identity_with_funding_signer(
                0,
                std::ptr::null(),
                AMOUNT_DUFFS,
                0,
                dangling_core_signer(),
                &mut balance,
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn rejects_null_core_signer() {
        let id = [0u8; 32];
        let mut balance = 0u64;
        let res = unsafe {
            platform_wallet_top_up_identity_with_funding_signer(
                0,
                &id,
                AMOUNT_DUFFS,
                0,
                std::ptr::null_mut(),
                &mut balance,
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn rejects_null_out_balance() {
        let id = [0u8; 32];
        let res = unsafe {
            platform_wallet_top_up_identity_with_funding_signer(
                0,
                &id,
                AMOUNT_DUFFS,
                0,
                dangling_core_signer(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    /// The external export guards its pointers like the managed one; the
    /// minimum amount is the wallet's check, so an unknown wallet answers
    /// first here.
    #[test]
    fn should_refuse_an_external_top_up_with_a_null_pointer_or_an_unknown_wallet() {
        let id = [0u8; 32];
        let mut balance = 7u64;
        let mut outpoint = OutPointFFI {
            txid: [0xab; 32],
            vout: 9,
        };
        let outpoint_ptr: *mut OutPointFFI = &mut outpoint;
        let call = |identity: *const [u8; 32],
                    signer: *mut MnemonicResolverHandle,
                    out: *mut u64,
                    out_outpoint: *mut OutPointFFI| unsafe {
            platform_wallet_top_up_external_identity_with_funding_signer(
                u64::MAX,
                identity,
                AMOUNT_DUFFS,
                0,
                signer,
                out,
                out_outpoint,
            )
        };
        for code in [
            call(
                std::ptr::null(),
                dangling_core_signer(),
                &mut balance,
                outpoint_ptr,
            )
            .code,
            call(&id, std::ptr::null_mut(), &mut balance, outpoint_ptr).code,
            call(
                &id,
                dangling_core_signer(),
                std::ptr::null_mut(),
                outpoint_ptr,
            )
            .code,
            call(
                &id,
                dangling_core_signer(),
                &mut balance,
                std::ptr::null_mut(),
            )
            .code,
        ] {
            assert_eq!(code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
        assert_eq!(
            call(&id, dangling_core_signer(), &mut balance, outpoint_ptr).code,
            PlatformWalletFFIResultCode::NotFound
        );
        assert_eq!(
            balance, 0,
            "the out-param is reset before any fallible work"
        );
        assert_eq!(
            (outpoint.txid, outpoint.vout),
            ([0; 32], 0),
            "nothing was built, so the outpoint is all zero"
        );
    }

    /// The built lock's outpoint reaches the host whatever the call returns:
    /// on a failure after the build, and after a panic the body caught. It
    /// stays all zero when nothing was built.
    #[test]
    fn should_write_the_built_outpoint_on_failure_and_panic_and_zero_otherwise() {
        let built_at = dashcore::OutPoint::new(dashcore::Txid::from_byte_array([0x42; 32]), 3);
        let mut out = OutPointFFI {
            txid: [0xab; 32],
            vout: 9,
        };

        let result = unsafe {
            report_built_outpoint(&mut out, |_| {
                PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorIdentityNotFound,
                    "nothing built",
                )
            })
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorIdentityNotFound
        );
        assert_eq!((out.txid, out.vout), ([0; 32], 0));

        let result = unsafe {
            report_built_outpoint(&mut out, |built| {
                *built.lock().unwrap() = Some(built_at);
                PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorUnknown,
                    "the Platform submit failed after the broadcast",
                )
            })
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorUnknown);
        assert_eq!(dashcore::OutPoint::from(&out), built_at);

        out = OutPointFFI {
            txid: [0; 32],
            vout: 0,
        };
        let mut result = unsafe {
            report_built_outpoint(&mut out, |built| {
                catch_top_up_panic("external identity top-up", || {
                    *built.lock().unwrap() = Some(built_at);
                    panic!("tokio worker panicked");
                })
            })
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed
        );
        assert_eq!(dashcore::OutPoint::from(&out), built_at);
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }
}
