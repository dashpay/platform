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
/// `account_index` addresses the *standard* Core families: the asset
/// lock POOLS the BIP44 and BIP32 accounts at that index together with
/// every DashPay receiving account (change returns to BIP44); the index
/// does not restrict which DashPay receiving accounts contribute.
/// `platform_account_index` selects which platform-payment account the
/// recipient addresses belong to.
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
                        consume_invitation_voucher: false,
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

/// Fund a THIRD PARTY's platform address from a Core L1 asset lock,
/// with the caller's own address absorbing the remainder (change) and
/// the fee.
///
/// Sister to [`platform_address_wallet_fund_from_asset_lock_signer`]:
/// identical parameters, identical marshalling, identical orchestration.
/// The only delta is the recipient pre-flight on the Rust side —
/// explicit-amount (`has_balance == true`) entries may be ANY valid
/// P2PKH address, while the single remainder (`has_balance == false`)
/// entry must still belong to `platform_account_index`.
///
/// Callers that mean to fund only their own addresses should keep using
/// the non-`external` entry point: there, a mistyped recipient is
/// rejected before anything is broadcast, whereas here it is a valid
/// (and irreversible) payment to a stranger.
///
/// `fee_strategy` is positional over the outputs map's LEXICOGRAPHIC
/// order (`PlatformAddress`'s derived `Ord`: P2PKH before P2SH, then
/// hash bytes), NOT over the order entries appear in `addresses` —
/// `decode_funding_addresses` collects into a `BTreeMap`. Callers must
/// compute `ReduceOutput(index)` against that ordering; the Swift SDK
/// does so by sorting its recipient array into the same canonical order
/// before marshalling.
///
/// # Safety
/// - `signer_address_handle` / `core_signer_handle` — see
///   [`platform_address_wallet_fund_from_asset_lock_signer`]. Same
///   ownership and validity contract.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_fund_from_asset_lock_external_signer(
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
    // Sentinel first — see `platform_address_wallet_fund_from_asset_lock_signer`.
    *out_changeset = PlatformAddressChangeSetFFI::empty();
    check_ptr!(addresses);
    check_ptr!(signer_address_handle);
    check_ptr!(core_signer_handle);

    let address_map = match decode_funding_addresses(addresses, addresses_count) {
        Ok(m) => m,
        Err(e) => return e,
    };

    let fee = parse_fee_strategy(fee_strategy, fee_strategy_count);

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
                .fund_from_asset_lock_external(
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

/// Resume an external-recipient platform-address funding flow from an
/// already-tracked asset lock by outpoint.
///
/// Sister to
/// [`platform_address_wallet_fund_from_asset_lock_external_signer`]
/// (same relaxed recipient rules) and to
/// [`platform_address_wallet_resume_fund_from_asset_lock_signer`] (same
/// resume-by-outpoint shape).
///
/// ## Why the recipients are a parameter and not recovered from state
///
/// The tracked asset lock records the L1 outpoint, its status and its
/// proof — it does NOT record who the credits were destined for. Nothing
/// on the Rust side ever learns the recipient set: it is chosen by the
/// host at ST-submit time and, for a third-party payment, is not even
/// derivable from the wallet's own key material. So a resume must be
/// told the recipients again, exactly as the shielded resume entry point
/// is (`platform_wallet_manager_shielded_resume_fund_from_asset_lock`).
///
/// The practical consequence is worth stating plainly: a resume with a
/// DIFFERENT recipient set than the original attempt is accepted, and
/// pays the new set. That is the correct behaviour for a flow whose
/// first attempt never landed on Platform — the asset lock is a bearer
/// input, not a commitment to a destination — but it does mean the host
/// is responsible for round-tripping the intended recipient (which is
/// what `PersistentAssetLock.recipientPlatformAddressHash` /
/// `recipientIsExternal` exist for on the Swift side).
///
/// # Safety
/// - `out_point` must be a valid, non-null pointer to an `OutPointFFI`
///   (32-byte raw txid + u32 vout). The caller retains ownership.
/// - `signer_address_handle` / `core_signer_handle` — see
///   [`platform_address_wallet_fund_from_asset_lock_signer`].
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_address_wallet_resume_fund_from_asset_lock_external_signer(
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
    // Sentinel first — see `platform_address_wallet_fund_from_asset_lock_signer`.
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
                .fund_from_asset_lock_external(
                    AssetLockFunding::FromExistingAssetLock {
                        out_point: resume_outpoint,
                        consume_invitation_voucher: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform_address_types::PlatformAddressFFI;

    fn entry(address_type: u8, tag: u8, balance: Option<u64>) -> FundingAddressEntryFFI {
        FundingAddressEntryFFI {
            address: PlatformAddressFFI {
                address_type,
                hash: [tag; 20],
            },
            has_balance: balance.is_some(),
            balance: balance.unwrap_or(0),
        }
    }

    /// Pins the contract the SDKs' fee-strategy computation depends on:
    /// `ReduceOutput(index)` is resolved by consensus against the
    /// outputs `BTreeMap`'s key order, and `decode_funding_addresses`
    /// builds that map. So when the caller hands us a canonically
    /// sorted array (P2PKH before P2SH, then hash bytes ascending — the
    /// derived `Ord` on `PlatformAddress`), array position and
    /// consensus output index are the SAME number.
    ///
    /// The hazard this guards: a caller that computes the remainder
    /// index from its own arbitrary array order silently re-targets the
    /// fee onto a different output whenever the remainder is not first
    /// lexicographically. With a third-party recipient in the set, that
    /// means the payee's explicit amount pays the fee instead of the
    /// sender's change.
    #[test]
    fn decoded_map_order_matches_canonically_sorted_input() {
        // Alice (0x0A) is the remainder; Bob (0xBB) and Carol (0xCC)
        // take explicit amounts. Alice sorts FIRST, so the remainder
        // index is 0 even though a caller listing "payees first" would
        // have naively computed 2.
        let canonical = [
            entry(0, 0x0A, None),
            entry(0, 0xBB, Some(500)),
            entry(0, 0xCC, Some(700)),
        ];

        let map = unsafe { decode_funding_addresses(canonical.as_ptr(), canonical.len()) }
            .expect("valid entries decode");

        let decoded_order: Vec<PlatformAddress> = map.keys().copied().collect();
        let input_order: Vec<PlatformAddress> = canonical
            .iter()
            .map(|e| PlatformAddress::try_from(e.address).expect("valid address"))
            .collect();
        assert_eq!(
            decoded_order, input_order,
            "a canonically sorted input array must survive the BTreeMap round trip in order"
        );

        let remainder_index = canonical
            .iter()
            .position(|e| !e.has_balance)
            .expect("exactly one remainder");
        assert_eq!(remainder_index, 0);
        assert_eq!(
            map.get(&decoded_order[remainder_index]),
            Some(&None),
            "ReduceOutput(remainder_index) must land on the None (remainder) output"
        );
    }

    /// The inverse arrangement: the remainder sorts LAST. Pins that the
    /// index tracks lexicographic position rather than being pinned to
    /// 0 or to the caller's listing order.
    #[test]
    fn remainder_index_tracks_lexicographic_position() {
        let canonical = [
            entry(0, 0x0A, Some(500)),
            entry(0, 0xBB, Some(700)),
            entry(0, 0xCC, None),
        ];

        let map = unsafe { decode_funding_addresses(canonical.as_ptr(), canonical.len()) }
            .expect("valid entries decode");
        let decoded_order: Vec<PlatformAddress> = map.keys().copied().collect();

        let remainder_index = canonical
            .iter()
            .position(|e| !e.has_balance)
            .expect("exactly one remainder");
        assert_eq!(remainder_index, 2);
        assert_eq!(map.get(&decoded_order[remainder_index]), Some(&None));
    }

    #[test]
    fn rejects_duplicate_recipients() {
        let entries = [entry(0, 0x0A, Some(500)), entry(0, 0x0A, None)];
        let err = unsafe { decode_funding_addresses(entries.as_ptr(), entries.len()) }
            .expect_err("duplicates must be rejected");
        assert_eq!(err.code, PlatformWalletFFIResultCode::ErrorInvalidParameter);
    }
}
