//! FFI bindings for claiming (withdrawing) a masternode identity's Platform
//! credits — `platform_wallet::wallet::masternode_withdrawal`.
//!
//! Two entry points, both keyed by `(manager, wallet_id, pro_tx_hash)`:
//!
//! - [`platform_wallet_manager_masternode_withdrawal_keys`]: which signing
//!   keys this wallet holds (owner / transfer) and the registered payout
//!   address — the UI gates the Withdraw button and the destination field
//!   on this, and it is the same resolution the withdraw path signs with.
//! - [`platform_wallet_manager_masternode_withdraw`]: the claim itself,
//!   signed through the host's mnemonic resolver like every other
//!   wallet-key signing path (`core_wallet_sign_message`,
//!   `core_wallet_tx_builder_finalize`).
//!
//! The masternode itself is resolved from the same provider-transaction
//! aggregation `platform_wallet_manager_list_masternodes` renders, so the
//! owner key hash / payout script the claim uses are exactly the ones the
//! list shows.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;
use std::sync::Arc;

use dashcore::Address as DashAddress;
use platform_wallet::{
    MasternodeWithdrawalKey, MasternodeWithdrawalKeys, MasternodeWithdrawalRequest, PlatformWallet,
};
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};

use crate::core_wallet_types::{aggregate_masternodes, ListMembership, MasternodeAggregate};
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::{check_ptr, unwrap_result_or_return};

/// Preflight result of [`platform_wallet_manager_masternode_withdrawal_keys`].
///
/// `payout_address` is a heap C string (or null when the node has no
/// encodable payout script) — free it with `platform_wallet_string_free`.
#[repr(C)]
pub struct MasternodeWithdrawalKeysFFI {
    /// This wallet holds the masternode's owner key (`ProviderOwnerKeys`).
    pub owner_key_in_wallet: bool,
    /// `ProviderOwnerKeys` index of the owner key; valid only when
    /// `owner_key_in_wallet`.
    pub owner_key_index: u32,
    /// This wallet holds the payout-script (identity `TRANSFER`) key, so a
    /// destination other than the payout address may be chosen.
    pub transfer_key_in_wallet: bool,
    /// Registered payout address (base58), or null.
    pub payout_address: *mut c_char,
}

impl MasternodeWithdrawalKeysFFI {
    fn empty() -> Self {
        Self {
            owner_key_in_wallet: false,
            owner_key_index: 0,
            transfer_key_in_wallet: false,
            payout_address: std::ptr::null_mut(),
        }
    }
}

/// Resolve `(wallet, masternode aggregate)` for a `pro_tx_hash` (wire
/// order) from the manager — the same aggregation the masternode list
/// renders. Clones the `Arc<PlatformWallet>` out so callers can do network
/// work after the handle-storage guard is released.
unsafe fn resolve_masternode(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
) -> Result<(Arc<PlatformWallet>, MasternodeAggregate), PlatformWalletFFIResult> {
    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);
    let target: [u8; 32] = std::ptr::read(pro_tx_hash as *const [u8; 32]);

    let resolved = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        let wallet = manager.get_wallet_blocking(&wid)?;
        let (_network, txs, dml, _operator_index, _platform_index) =
            manager.provider_masternode_txs_blocking(&wid)?;
        let membership = |pro_tx_hash: &[u8; 32]| -> ListMembership {
            match &dml {
                None => ListMembership::ListUnavailable,
                Some(map) => match map.get(pro_tx_hash) {
                    Some(true) => ListMembership::ValidEntry,
                    Some(false) => ListMembership::InvalidEntry,
                    None => ListMembership::Absent,
                },
            }
        };
        let aggregate =
            aggregate_masternodes(txs.iter().map(|(h, p, tx)| (*h, *p, tx)), membership)
                .into_iter()
                .find(|mn| mn.pro_tx_hash == target);
        Some((wallet, aggregate))
    });

    match resolved {
        None => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            "invalid platform wallet manager handle",
        )),
        Some(None) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "wallet not found in the manager",
        )),
        Some(Some((_, None))) => Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::NotFound,
            "no masternode with this proTxHash in the wallet's provider transactions",
        )),
        Some(Some((wallet, Some(mn)))) => Ok((wallet, mn)),
    }
}

/// Which masternode-withdrawal signing keys this wallet holds for the
/// masternode `pro_tx_hash` (32 bytes, wire order), plus its registered
/// payout address. Seedless — no resolver needed.
///
/// # Safety
/// `wallet_id` / `pro_tx_hash` must point at 32 readable bytes; `out` must
/// be writable. Free `out.payout_address` with `platform_wallet_string_free`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_masternode_withdrawal_keys(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    out: *mut MasternodeWithdrawalKeysFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(out);
    *out = MasternodeWithdrawalKeysFFI::empty();

    let (wallet, mn) =
        unwrap_result_or_return!(resolve_masternode(manager_handle, wallet_id, pro_tx_hash));
    let keys =
        unwrap_result_or_return!(wallet
            .masternode_withdrawal_keys(mn.owner_key_hash.as_ref(), mn.payout_script.as_deref()));

    let payout_address = match keys.payout_address {
        Some(address) => unwrap_result_or_return!(CString::new(address)).into_raw(),
        None => std::ptr::null_mut(),
    };
    *out = MasternodeWithdrawalKeysFFI {
        owner_key_in_wallet: keys.owner_key.is_some(),
        owner_key_index: keys
            .owner_key
            .as_ref()
            .map(|(index, _)| *index)
            .unwrap_or(0),
        transfer_key_in_wallet: keys.transfer_key.is_some(),
        payout_address,
    };
    PlatformWalletFFIResult::ok()
}

/// Claim (withdraw) `amount_credits` from the masternode identity of
/// `pro_tx_hash` (32 bytes, wire order) to L1 via an Identity Credit
/// Withdrawal, signed with a wallet-held key derived through the host's
/// mnemonic resolver. Writes the identity's remaining balance to
/// `out_new_balance`.
///
/// - `use_owner_key == true`: sign with the `ProviderOwnerKeys` key.
///   Platform pays the registered payout address; `dest_address` MUST be
///   null.
/// - `use_owner_key == false`: sign with the payout-script (`TRANSFER`)
///   key. `dest_address` (a base58 address for the wallet's network) is the
///   destination; null ⇒ the registered payout address.
///
/// Fails WITHOUT broadcasting when the wallet doesn't hold the requested
/// key, the identity doesn't carry the matching key, or the destination is
/// invalid. The resolver is invoked exactly once, at signing time; the
/// handle-storage guard is not held across it.
///
/// # Safety
/// `wallet_id` / `pro_tx_hash` must point at 32 readable bytes;
/// `dest_address` is null or a NUL-terminated UTF-8 string;
/// `mnemonic_resolver_handle` must come from
/// `dash_sdk_mnemonic_resolver_create` and outlive this call;
/// `out_new_balance` must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_masternode_withdraw(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    amount_credits: u64,
    use_owner_key: bool,
    dest_address: *const c_char,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(out_new_balance);
    *out_new_balance = 0;

    if use_owner_key && !dest_address.is_null() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "an owner-key withdrawal pays the registered payout address; dest_address must be null",
        );
    }

    let (wallet, mn) =
        unwrap_result_or_return!(resolve_masternode(manager_handle, wallet_id, pro_tx_hash));
    let network = wallet.network();

    let destination = if dest_address.is_null() {
        None
    } else {
        let text = unwrap_result_or_return!(CStr::from_ptr(dest_address).to_str()).to_string();
        let unchecked = unwrap_result_or_return!(DashAddress::from_str(&text));
        Some(unwrap_result_or_return!(unchecked.require_network(network)))
    };

    // Key resolution is blocking (wallet-manager read lock + seedless
    // derive-and-compare) — runs here on the caller thread, before the
    // async claim below.
    let keys: MasternodeWithdrawalKeys =
        unwrap_result_or_return!(wallet
            .masternode_withdrawal_keys(mn.owner_key_hash.as_ref(), mn.payout_script.as_deref()));

    let request = MasternodeWithdrawalRequest {
        pro_tx_hash: mn.pro_tx_hash,
        owner_key_hash: mn.owner_key_hash,
        amount_credits,
        signing_key: if use_owner_key {
            MasternodeWithdrawalKey::Owner
        } else {
            MasternodeWithdrawalKey::Transfer
        },
        destination,
    };

    // SAFETY: `signer_addr` came from `mnemonic_resolver_handle`, which the
    // caller keeps alive for this call; the calling thread blocks until the
    // future completes, so the signer is dropped before this frame returns.
    let signer_addr = mnemonic_resolver_handle as usize;
    let wallet_id_bytes = wallet.wallet_id();
    let new_balance = unwrap_result_or_return!(block_on_worker(async move {
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id_bytes,
            network,
        );
        wallet.masternode_withdraw(request, &keys, &signer).await
    }));

    *out_new_balance = new_balance;
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_sdk_ffi::{dash_sdk_mnemonic_resolver_create, dash_sdk_mnemonic_resolver_destroy};
    use std::os::raw::c_void;

    unsafe extern "C" fn never_resolve(
        _ctx: *const c_void,
        _wallet_id_bytes: *const u8,
        _out_buf: *mut c_char,
        _out_capacity: usize,
        _out_len: *mut usize,
    ) -> i32 {
        unreachable!("rejected before any mnemonic is resolved");
    }

    unsafe extern "C" fn noop_destroy(_ctx: *mut c_void) {}

    #[test]
    fn owner_key_withdrawal_rejects_a_destination_before_touching_handles() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), never_resolve, noop_destroy)
        };
        let wallet_id = [0u8; 32];
        let pro_tx_hash = [1u8; 32];
        let dest = CString::new("yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt").unwrap();
        let mut out_balance = u64::MAX;
        let result = unsafe {
            platform_wallet_manager_masternode_withdraw(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                1_000_000,
                true,
                dest.as_ptr(),
                resolver,
                &mut out_balance,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out_balance, 0, "out parameter is initialised on every path");
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }

    #[test]
    fn unknown_manager_handle_is_an_invalid_handle() {
        let resolver = unsafe {
            dash_sdk_mnemonic_resolver_create(std::ptr::null_mut(), never_resolve, noop_destroy)
        };
        let wallet_id = [0u8; 32];
        let pro_tx_hash = [1u8; 32];
        let mut out_balance = 0u64;
        let result = unsafe {
            platform_wallet_manager_masternode_withdraw(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                1_000_000,
                false,
                std::ptr::null(),
                resolver,
                &mut out_balance,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);

        let mut out = MasternodeWithdrawalKeysFFI::empty();
        let result = unsafe {
            platform_wallet_manager_masternode_withdrawal_keys(
                Handle::MAX,
                wallet_id.as_ptr(),
                pro_tx_hash.as_ptr(),
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert!(out.payout_address.is_null());
        unsafe { dash_sdk_mnemonic_resolver_destroy(resolver) };
    }
}
