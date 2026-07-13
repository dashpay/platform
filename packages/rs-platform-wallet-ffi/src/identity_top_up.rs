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
//! Top-up state-transitions are signed entirely with the Platform
//! address inputs' private keys (the SDK uses `BalanceTransfer` to
//! credit the existing identity), so this FFI takes a single
//! `SignerHandle` — `signer_address_handle` — used as
//! `Signer<PlatformAddress>`. No identity-key signer is needed
//! (existing identity, no IdentityCreate to sign).
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
    let identity_id = Identifier::from(identity_id_bytes);

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
    if amount_duffs == 0 {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "`amount_duffs` is zero",
        );
    }

    let identity_id_bytes: [u8; 32] = *identity_id;
    let identity_id = Identifier::from(identity_id_bytes);

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
