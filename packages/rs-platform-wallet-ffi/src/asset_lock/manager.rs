//! Handle management and queries for AssetLockManager.

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return};

/// C-compatible tracked asset lock entry.
#[repr(C)]
pub struct TrackedAssetLockFFI {
    /// Outpoint txid (32 bytes).
    pub txid: [u8; 32],
    /// Outpoint vout.
    pub vout: u32,
    /// BIP44 account index.
    pub account_index: u32,
    /// Funding type (0=IdentityRegistration, 1=IdentityTopUp, 2=IdentityTopUpNotBound,
    /// 3=IdentityInvitation, 4=AssetLockAddressTopUp, 5=AssetLockShieldedAddressTopUp).
    pub funding_type: u32,
    /// Identity index.
    pub identity_index: u32,
    /// Amount in duffs.
    pub amount: u64,
    /// Status (0=Built, 1=Broadcast, 2=InstantSendLocked, 3=ChainLocked,
    /// 4=Consumed, 5=RecoveredFromChain — finality proven by the restore
    /// scan, Platform-side consumption unknown).
    pub status: u32,
    /// Whether a proof is attached.
    pub has_proof: bool,
}

/// Destroy an AssetLockManager handle.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_destroy(handle: Handle) -> PlatformWalletFFIResult {
    ASSET_LOCK_MANAGER_STORAGE.remove(handle);
    PlatformWalletFFIResult::ok()
}

/// List all tracked asset locks.
///
/// On success, `out_locks` and `out_count` are set to a heap-allocated array.
/// Free with `asset_lock_manager_free_tracked_locks`.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_list_tracked_locks(
    handle: Handle,
    out_locks: *mut *mut TrackedAssetLockFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(out_locks);
    check_ptr!(out_count);

    let option = ASSET_LOCK_MANAGER_STORAGE.with_item(handle, |manager| {
        use platform_wallet::AssetLockStatus;

        let locks = runtime().block_on(manager.list_tracked_locks());
        let entries: Vec<TrackedAssetLockFFI> = locks
            .iter()
            .map(|lock| {
                let mut txid = [0u8; 32];
                txid.copy_from_slice(&lock.out_point.txid[..]);
                TrackedAssetLockFFI {
                    txid,
                    vout: lock.out_point.vout,
                    account_index: lock.account_index,
                    funding_type: match lock.funding_type {
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityRegistration => 0,
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp => 1,
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUpNotBound => 2,
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityInvitation => 3,
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::AssetLockAddressTopUp => 4,
                        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::AssetLockShieldedAddressTopUp => 5,
                    },
                    identity_index: lock.identity_index,
                    amount: lock.amount,
                    status: match lock.status {
                        AssetLockStatus::Built => 0,
                        AssetLockStatus::Broadcast => 1,
                        AssetLockStatus::InstantSendLocked => 2,
                        AssetLockStatus::ChainLocked => 3,
                        AssetLockStatus::Consumed => 4,
                        AssetLockStatus::RecoveredFromChain => 5,
                    },
                    has_proof: lock.proof.is_some(),
                }
            })
            .collect();
        entries
    });
    let entries = unwrap_option_or_return!(option);

    *out_count = entries.len();
    if entries.is_empty() {
        *out_locks = std::ptr::null_mut();
    } else {
        *out_locks = Box::into_raw(entries.into_boxed_slice()) as *mut TrackedAssetLockFFI;
    }
    PlatformWalletFFIResult::ok()
}

/// Free tracked locks array.
#[no_mangle]
pub unsafe extern "C" fn asset_lock_manager_free_tracked_locks(
    locks: *mut TrackedAssetLockFFI,
    count: usize,
) {
    if !locks.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(locks, count));
    }
}
