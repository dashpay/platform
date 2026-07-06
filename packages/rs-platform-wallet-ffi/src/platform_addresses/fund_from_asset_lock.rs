//! Asset-lock-funded platform-address top-up driven by external signers.
//!
//! Two signer surfaces are deliberately distinct (mirrors the
//! identity-side `platform_wallet_register_identity_with_funding_signer`):
//!
//! - `signer_address_handle` (a `*mut rs_sdk_ffi::SignerHandle`) is
//!   the platform-address per-input-witness signer (ECDSA over each
//!   `AddressWitness`).
//! - `core_signer_handle` (a `*mut MnemonicResolverHandle`) is the
//!   Core-side ECDSA signer used for the asset-lock's outer
//!   state-transition signature, atomically deriving + signing +
//!   zeroising inside the Keychain-resolver trust boundary.
//!
//! Two entry points: one for fresh wallet-balance funding, one for
//! resuming a tracked asset lock by outpoint (crash-recovery shape).

use std::collections::BTreeMap;

use dashcore::hashes::Hash;
use dpp::address_funds::PlatformAddress;
use platform_wallet::wallet::asset_lock::AssetLockFunding;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle, SignerHandle, VTableSigner};

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::platform_address_types::*;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// Fund platform addresses from a Core L1 asset lock, orchestrated
/// through the wallet's `AssetLockManager` (build → IS-or-CL → submit
/// → consume), with the asset-lock signature produced by an external
/// `MnemonicResolverHandle`.
///
/// `account_index` selects the BIP44 *standard* Core account whose
/// UTXOs fund the asset lock (only BIP44 standard accounts supported
/// today). `platform_account_index` selects which platform-payment
/// account the recipient addresses belong to.
///
/// # Safety
/// - `signer_address_handle` must be a valid, non-destroyed
///   `*mut SignerHandle` produced by `dash_sdk_signer_create_with_ctx`.
///   The caller retains ownership.
/// - `core_signer_handle` must be a valid, non-destroyed
///   `*mut MnemonicResolverHandle` produced by
///   [`crate::dash_sdk_mnemonic_resolver_create`]. The caller retains
///   ownership.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_fund_from_asset_lock_signer(
    handle: Handle,
    amount_duffs: u64,
    account_index: u32,
    platform_account_index: u32,
    addresses: *const FundingAddressEntryFFI,
    addresses_count: usize,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    signer_address_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_changeset: *mut PlatformAddressChangeSetFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_changeset);
    // Sentinel first: address decoding, the wallet lookup, and the async
    // fund below are all fallible. See `PlatformAddressChangeSetFFI::empty`
    // for the double-free rationale.
    *out_changeset = PlatformAddressChangeSetFFI::empty();
    check_ptr!(addresses);
    check_ptr!(signer_address_handle);
    check_ptr!(core_signer_handle);

    let address_map = match decode_funding_addresses(addresses, addresses_count) {
        Ok(m) => m,
        Err(e) => return e,
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    // Round-trip both handles through `usize` so the spawned future's
    // capture is `Send + 'static` (raw pointers are `!Send`).
    let signer_addr = signer_address_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet_clone = wallet.clone();
        let wallet_id = wallet.wallet_id();
        // Pull the network from the wallet rather than threading it
        // as an extra FFI parameter — it would be ambiguous if the
        // two disagreed.
        let network = wallet.network();
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — both handles are
            // pinned alive for the duration of this FFI call.
            let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            wallet_clone
                .fund_from_asset_lock(
                    AssetLockFunding::FromWalletBalance {
                        amount_duffs,
                        account_index,
                    },
                    platform_account_index,
                    address_map,
                    fee,
                    address_signer,
                    &asset_lock_signer,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
}

/// Resume a platform-address funding flow from an already-tracked
/// asset lock by outpoint.
///
/// Sister to [`platform_address_wallet_fund_from_asset_lock_signer`]:
/// instead of building a fresh asset-lock transaction, pick up an
/// existing tracked lock and drive whatever stages remain
/// (broadcast, IS/CL wait, Platform submission). Use case mirrors
/// the identity-side resume path — a prior attempt left the lock
/// in storage at `Broadcast` / `InstantSendLocked` / `ChainLocked`
/// but the address-funding ST never completed, and the user wants
/// to consume the lock from the "Unused Asset Locks" picker.
///
/// # Safety
/// - `out_point` must be a valid, non-null pointer to an
///   `OutPointFFI` (32-byte raw txid + u32 vout). The caller retains
///   ownership; the FFI does not free it.
/// - `signer_address_handle` / `core_signer_handle` — see
///   [`platform_address_wallet_fund_from_asset_lock_signer`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_resume_fund_from_asset_lock_signer(
    handle: Handle,
    out_point: *const OutPointFFI,
    platform_account_index: u32,
    addresses: *const FundingAddressEntryFFI,
    addresses_count: usize,
    fee_strategy: *const FeeStrategyStepFFI,
    fee_strategy_count: usize,
    signer_address_handle: *mut SignerHandle,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_changeset: *mut PlatformAddressChangeSetFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_changeset);
    // Sentinel first: address decoding, the wallet lookup, and the async
    // resume-fund below are all fallible. See
    // `PlatformAddressChangeSetFFI::empty` for the double-free rationale.
    *out_changeset = PlatformAddressChangeSetFFI::empty();
    check_ptr!(addresses);
    check_ptr!(out_point);
    check_ptr!(signer_address_handle);
    check_ptr!(core_signer_handle);

    let address_map = match decode_funding_addresses(addresses, addresses_count) {
        Ok(m) => m,
        Err(e) => return e,
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

    let out_point_ffi = *out_point;
    let resume_outpoint = dashcore::OutPoint {
        txid: dashcore::Txid::from_byte_array(out_point_ffi.txid),
        vout: out_point_ffi.vout,
    };

    let signer_addr = signer_address_handle as usize;
    let core_signer_addr = core_signer_handle as usize;

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet_clone = wallet.clone();
        let wallet_id = wallet.wallet_id();
        let network = wallet.network();
        block_on_worker(async move {
            // SAFETY: see the fn-level safety doc — both handles are
            // pinned alive for the duration of this FFI call.
            let address_signer: &VTableSigner = unsafe { &*(signer_addr as *const VTableSigner) };
            let asset_lock_signer = unsafe {
                MnemonicResolverCoreSigner::new(
                    core_signer_addr as *mut MnemonicResolverHandle,
                    wallet_id,
                    network,
                )
            };
            wallet_clone
                .fund_from_asset_lock(
                    AssetLockFunding::FromExistingAssetLock {
                        out_point: resume_outpoint,
                    },
                    platform_account_index,
                    address_map,
                    fee,
                    address_signer,
                    &asset_lock_signer,
                    None,
                )
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let changeset = unwrap_result_or_return!(result);
    *out_changeset = PlatformAddressChangeSetFFI::from(&changeset);
    PlatformWalletFFIResult::ok()
}

/// Decode an FFI array of `FundingAddressEntryFFI` into the
/// `BTreeMap<PlatformAddress, Option<Credits>>` shape that
/// `fund_from_asset_lock` consumes.
///
/// # Safety
/// - `addresses` must be a valid, non-null pointer to an array of
///   at least `addresses_count` `FundingAddressEntryFFI` entries
///   WHEN `addresses_count > 0`. A `0`-count call is handled
///   short-circuit and does not dereference `addresses`, so a
///   dangling non-null sentinel pointer in that case is sound.
pub(super) unsafe fn decode_funding_addresses(
    addresses: *const FundingAddressEntryFFI,
    addresses_count: usize,
) -> Result<BTreeMap<PlatformAddress, Option<dpp::fee::Credits>>, PlatformWalletFFIResult> {
    // Short-circuit the empty case to dodge the
    // `slice::from_raw_parts` safety contract entirely when no
    // dereference is needed. Downstream `validate_recipient_addresses`
    // rejects empty with a typed error.
    if addresses_count == 0 {
        return Ok(BTreeMap::new());
    }
    let mut address_map = BTreeMap::new();
    for entry in std::slice::from_raw_parts(addresses, addresses_count) {
        let addr = PlatformAddress::try_from(entry.address).map_err(|e| {
            PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("invalid platform address: {e}"),
            )
        })?;
        let balance = if entry.has_balance {
            Some(entry.balance)
        } else {
            None
        };
        // Reject duplicates rather than silently collapsing them.
        // The Swift wrapper's `fundFromAssetLockPreflight` already
        // dedupes client-side, but this is the FFI boundary —
        // callers other than our Swift code (or a future Swift
        // bug) could pass duplicates and we'd silently lose the
        // earlier entry's amount.
        if address_map.insert(addr, balance).is_some() {
            return Err(PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                "duplicate platform address in funding request".to_string(),
            ));
        }
    }
    Ok(address_map)
}
