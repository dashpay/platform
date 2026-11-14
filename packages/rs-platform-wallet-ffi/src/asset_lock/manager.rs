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
    /// Family-independent source index of the pooled funding (BIP44 and
    /// BIP32 at this index plus DashPay receiving accounts). Not a
    /// BIP44-only selector.
    pub account_index: u32,
    /// Funding type (0=IdentityRegistration, 1=IdentityTopUp, 2=IdentityTopUpNotBound,
    /// 3=IdentityInvitation, 4=AssetLockAddressTopUp, 5=AssetLockShieldedAddressTopUp).
    pub funding_type: u32,
    /// Identity index.
    pub identity_index: u32,
    /// Amount in duffs.
    pub amount: u64,
    /// Status (0=Built, 1=Broadcast, 2=InstantSendLocked, 3=ChainLocked,
    /// 4=Consumed, 5=RecoveredFromChain — Core finality proven by restore
    /// reconstruction or live reconciliation, Platform-side consumption unknown;
    /// 6=DrainCompleted — proven drain consumption and owned output recorded durably).
    pub status: u32,
    /// Whether a proof is attached.
    pub has_proof: bool,
    /// Whether `shielded_recipient_raw_43` carries the Orchard recipient a
    /// shielded funding (funding type 5) was built to pay. False for other
    /// funding types and for shielded locks tracked before the recipient was
    /// recorded (a resume of those pays the recipient the caller names).
    pub has_shielded_recipient: bool,
    /// The raw 43-byte Orchard recipient a resume of this lock pays, valid
    /// only when `has_shielded_recipient` is true (all zero otherwise).
    pub shielded_recipient_raw_43: [u8; 43],
    /// Whether `identity_recipient_32` carries the identity an unbound
    /// identity top-up (funding type 2) pays. A resume of the lock pays that
    /// identity and refuses any other.
    pub has_identity_recipient: bool,
    /// The identity a resume of this lock pays, valid only when
    /// `has_identity_recipient` is true (all zero otherwise).
    pub identity_recipient_32: [u8; 32],
    /// An unbound identity top-up, not consumed, that records no identity
    /// (rebuilt from the chain, for example after a restore from the seed):
    /// no resume pays it until the user assigns it a recipient with
    /// `platform_wallet_assign_identity_top_up_recipient`. Hosts list it
    /// for the user rather than resuming it into one of their identities.
    pub identity_recipient_unassigned: bool,
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
        AssetLockStatus::DrainCompleted => 6,
                    },
                    has_proof: lock.proof.is_some(),
                    has_shielded_recipient: lock.shielded_recipient.is_some(),
                    shielded_recipient_raw_43: lock.shielded_recipient.unwrap_or([0; 43]),
                    has_identity_recipient: lock.identity_recipient.is_some(),
                    identity_recipient_32: lock
                        .identity_recipient
                        .map(|identity| identity.to_buffer())
                        .unwrap_or([0; 32]),
                    identity_recipient_unassigned: lock.is_unassigned_identity_top_up(),
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
