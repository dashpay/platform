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

use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::Identifier;
use platform_wallet::AssetLockFunding;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
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
     and may still execute: do NOT submit it again; the next sync settles the outcome.";

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
/// a failed read is `ErrorIdentityLookupFailed` (80) and an identity
/// Platform does not know `ErrorIdentityNotFound` (7), nothing sent.
///
/// `ErrorTransactionBroadcastUnconfirmed` (20) means the top-up may have
/// executed or may still execute: its result was not proved, or the call
/// panicked, possibly after signing or broadcast. A result proof counts only
/// when it shows every input address at or past the nonce this top-up was
/// signed with AND the identity balance above the balance the top-up started
/// from, a fresh read from Platform taken before signing; any other proof is
/// 20 and nothing is reconciled from it. The host must not submit it again;
/// the next sync settles the outcome.
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `identity_id` must point at a 32-byte identity id buffer for
///   the duration of the call.
/// - `inputs` must point at a valid `[IdentityFundingInputFFI;
///   inputs_count]` array; each row's `hash` is a 20-byte address
///   hash.
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
        check_ptr!(inputs);
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
/// address and refuses what it cannot act on.
///
/// # Safety
/// `inputs` must point at a valid `[IdentityFundingInputFFI; inputs_count]`
/// array.
unsafe fn funding_rows(
    inputs: *const IdentityFundingInputFFI,
    inputs_count: usize,
) -> Result<Vec<(PlatformAddress, Credits)>, PlatformWalletFFIResult> {
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
/// Platform does not know returns `ErrorIdentityNotFound` before anything
/// is signed or broadcast.
///
/// `ErrorTransactionBroadcastUnconfirmed` (20) means the top-up may have
/// executed or may still execute: its result was not proved, or the call
/// panicked, possibly after signing or broadcast. A result proof counts only
/// when it shows every input address at or past the nonce this top-up was
/// signed with AND the identity balance above the balance the top-up started
/// from, a fresh read from Platform taken before signing; any other proof is
/// 20 and nothing is reconciled from it. The host must not submit it again;
/// the next sync settles the outcome.
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
            check_ptr!(inputs);
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

/// Minimum asset-lock funding for a Core-funded identity top-up, in duffs.
///
/// Platform rejects an `IdentityTopUp` whose asset-lock output value (in
/// credits) is below `IdentityTopUpTransition::calculate_min_required_fee`.
/// Under the active fee version (v1) that minimum is
/// `identity_topup_base_cost` (500_000 credits = 500 duffs) **plus**
/// `required_asset_lock_duff_balance_for_processing_start_for_identity_top_up`
/// (50_000 duffs) — i.e. 50_500 duffs. Below that, a lock built and broadcast
/// here is accepted by Core (spending real UTXOs) but rejected by Platform,
/// stranding the funds in a lock that can never complete the top-up. Reject
/// sub-floor amounts up front so no such lock is ever broadcast.
///
/// (The bare 50_000 asset-lock floor alone was the fee-v0 minimum; the active
/// v1 calc — `STATE_TRANSITION_VERSIONS_V3`, protocol v11+ — adds the base
/// cost, so this constant must include it.)
const MIN_TOP_UP_DUFFS: u64 = 50_500;

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
/// `amount_duffs` must be at least [`MIN_TOP_UP_DUFFS`]; a smaller amount is
/// rejected with `ErrorInvalidParameter` before any lock is broadcast.
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
    if amount_duffs < MIN_TOP_UP_DUFFS {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`amount_duffs` is below the minimum top-up asset-lock balance",
        );
    }

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

#[cfg(test)]
mod top_up_with_funding_guard_tests {
    use super::*;
    use crate::error::PlatformWalletFFIResultCode;

    /// A non-null but never-dereferenced core-signer pointer. Every guard
    /// under test returns before the handle is used, so a dangling pointer
    /// is sufficient (and never unsound here).
    fn dangling_core_signer() -> *mut MnemonicResolverHandle {
        std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr()
    }

    #[test]
    fn rejects_null_identity_id() {
        let mut balance = 0u64;
        let res = unsafe {
            platform_wallet_top_up_identity_with_funding_signer(
                0,
                std::ptr::null(),
                MIN_TOP_UP_DUFFS,
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
                MIN_TOP_UP_DUFFS,
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
                MIN_TOP_UP_DUFFS,
                0,
                dangling_core_signer(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(res.code, PlatformWalletFFIResultCode::ErrorNullPointer);
    }

    #[test]
    fn rejects_sub_floor_amount() {
        let id = [0u8; 32];
        let mut balance = 0u64;
        for amount in [0u64, MIN_TOP_UP_DUFFS - 1] {
            let res = unsafe {
                platform_wallet_top_up_identity_with_funding_signer(
                    0,
                    &id,
                    amount,
                    0,
                    dangling_core_signer(),
                    &mut balance,
                )
            };
            assert_eq!(
                res.code,
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "amount {amount} below MIN_TOP_UP_DUFFS should be rejected"
            );
        }
    }
}
