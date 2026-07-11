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

use std::collections::BTreeMap;
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
    check_ptr!(identity_id);
    check_ptr!(inputs);
    check_ptr!(signer_address_handle);
    check_ptr!(out_new_balance);
    if inputs_count == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`inputs_count` is zero",
        );
    }

    let identity_id_bytes: [u8; 32] = *identity_id;
    let identity_id = Identifier::from_bytes(&identity_id_bytes).unwrap_or_default();

    let entries = slice::from_raw_parts(inputs, inputs_count);
    let mut input_map: BTreeMap<PlatformAddress, Credits> = BTreeMap::new();
    for entry in entries {
        let address = match entry.address_type {
            0 => PlatformAddress::P2pkh(entry.hash),
            1 => PlatformAddress::P2sh(entry.hash),
            _ => {
                return PlatformWalletFFIResult::err(
                    PlatformWalletFFIResultCode::ErrorInvalidParameter,
                    "invalid address_type (expected 0 or 1)",
                );
            }
        };
        // Caller may pass the same address twice (the Swift inputs
        // builder is greedy and could legitimately split a single
        // address across multiple rows in a future shape). Sum the
        // contributions rather than overwriting — `BTreeMap::insert`
        // would silently keep only the last credit value, causing
        // the top-up to under-fund the identity by the difference.
        // Saturating add keeps a pathological caller from looping
        // overflow back to a small value.
        input_map
            .entry(address)
            .and_modify(|existing| *existing = existing.saturating_add(entry.credits))
            .or_insert(entry.credits);
    }

    // Round-trip the signer pointer through `usize` so the spawned
    // future's capture is `Send + 'static` (the raw pointer is `!Send`,
    // but `usize` is). The underlying `Inner::Callback { ctx, vtable }`
    // is `Send + Sync` — see the unsafe impls in `rs-sdk-ffi/src/signer.rs`.
    let signer_addr = signer_address_handle as usize;

    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        let wallet = wallet.clone();
        block_on_worker(async move {
            let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            // The composite tops up the identity AND reconciles the spent
            // platform-address balances from the proof, so the wallet's
            // displayed balance and next input selection reflect the spend
            // (covering addresses restored from disk that are no longer in
            // a live derived pool).
            wallet
                .top_up_from_addresses(&identity_id, input_map, address_signer, None)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let new_balance = unwrap_result_or_return!(result);
    *out_new_balance = new_balance;
    PlatformWalletFFIResult::ok()
}

/// Minimum asset-lock funding for a Core-funded identity top-up, in duffs.
///
/// Platform will not start processing an `IdentityTopUp` whose asset lock
/// funds less than
/// `required_asset_lock_duff_balance_for_processing_start_for_identity_top_up`
/// (currently 50_000 duffs). Below that, a lock built and broadcast here is
/// accepted by Core (spending real UTXOs) but rejected by Platform, stranding
/// the funds in a lock that can never complete the top-up. Reject sub-floor
/// amounts up front so no such lock is ever broadcast.
const MIN_TOP_UP_DUFFS: u64 = 50_000;

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
/// registration. `account_index` selects which BIP44 *standard* account
/// the asset-lock UTXOs are drawn from (only BIP44 standard accounts are
/// supported today, matching registration).
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
