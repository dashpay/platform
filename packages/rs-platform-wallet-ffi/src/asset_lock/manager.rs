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
    /// An unbound identity top-up that records no identity and that the
    /// wallet knows is not consumed (a live status: Built, Broadcast,
    /// InstantSendLocked or ChainLocked): no resume pays it until the user
    /// assigns it a recipient with
    /// `platform_wallet_assign_identity_top_up_recipient`. Hosts list it for
    /// the user rather than resuming it into one of their identities.
    ///
    /// False for a recipient-less lock with status `RecoveredFromChain`
    /// (status 5), which every past top-up rebuilt from the chain after a
    /// restore from the seed carries: Platform may have consumed it long ago,
    /// and no query tells the wallet whether it did, so hosts must not
    /// present it as recoverable. Assigning one is refused with
    /// `ErrorAssetLockConsumptionUnknown` (82) and nothing is assigned; its key
    /// stays with the wallet until a consumption check exists.
    pub identity_recipient_unassigned: bool,
    /// Whether the lock is an external Platform-address funding (funding type
    /// 4) that recorded the addresses it pays (read them with
    /// `platform_wallet_get_asset_lock_platform_address_recipients`). A
    /// resume pays exactly those and refuses anything else, an owned funding
    /// included, with `ErrorAssetLockRecipientMismatch`; resume it with
    /// `platform_address_wallet_resume_fund_from_asset_lock_to_recorded_recipients_signer`.
    /// False for other funding types and for an owned Platform-address
    /// funding.
    pub has_platform_address_recipients: bool,
}

/// One recipient of an external Platform-address funding.
#[repr(C)]
pub struct AssetLockPlatformAddressRecipientFFI {
    /// The Platform address in its 21-byte storage form (type byte, 0 = P2PKH
    /// or 1 = P2SH, then the 20-byte hash).
    pub address: [u8; 21],
    /// False for the single remainder recipient (the sender's own change),
    /// which receives what is left after the explicit amounts and the fee.
    pub has_amount: bool,
    /// Explicit amount in credits, valid only when `has_amount` is true.
    pub amount_credits: u64,
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
                    has_platform_address_recipients: lock.platform_address_recipients.is_some(),
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

/// Read the Platform addresses a tracked asset lock of the platform wallet
/// `wallet_handle` recorded when it was built by an external Platform-address
/// funding.
///
/// On success `out_found` says whether the lock records them. When it does,
/// `out_recipients` / `out_count` are a heap-allocated recipient array in the
/// order the wallet stores it (the order consensus resolves the outputs in);
/// free it with `platform_wallet_free_asset_lock_platform_address_recipients`.
/// A lock that is not tracked is reported as not found. The lock resumes only
/// to exactly these recipients.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_asset_lock_platform_address_recipients(
    wallet_handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    out_found: *mut bool,
    out_recipients: *mut *mut AssetLockPlatformAddressRecipientFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(txid);
    check_ptr!(out_found);
    check_ptr!(out_recipients);
    check_ptr!(out_count);

    let out_point = dashcore::OutPoint {
        txid: dashcore::Txid::from_byte_array(*txid),
        vout,
    };
    let option = PLATFORM_WALLET_STORAGE.with_item(wallet_handle, |wallet| {
        std::sync::Arc::clone(wallet.asset_locks())
    });
    let asset_locks = unwrap_option_or_return!(option);
    let recorded = runtime().block_on(asset_locks.tracked_platform_address_recipients(&out_point));

    *out_found = recorded.is_some();
    *out_recipients = std::ptr::null_mut();
    *out_count = 0;
    if let Some(recorded) = recorded {
        let recipients: Vec<AssetLockPlatformAddressRecipientFFI> = recorded
            .as_map()
            .iter()
            .map(|(address, amount)| AssetLockPlatformAddressRecipientFFI {
                address: address.to_bytes().try_into().unwrap_or([0; 21]),
                has_amount: amount.is_some(),
                amount_credits: amount.unwrap_or(0),
            })
            .collect();
        *out_count = recipients.len();
        if !recipients.is_empty() {
            *out_recipients = Box::into_raw(recipients.into_boxed_slice()).cast();
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Free a recipient array returned by
/// `platform_wallet_get_asset_lock_platform_address_recipients`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_free_asset_lock_platform_address_recipients(
    recipients: *mut AssetLockPlatformAddressRecipientFFI,
    count: usize,
) {
    if !recipients.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(recipients, count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The recipients getter checks its pointers first and reports an
    /// unknown wallet handle as not found.
    #[test]
    fn should_refuse_a_recipients_read_with_a_null_pointer_or_an_unknown_wallet() {
        let txid = [0u8; 32];
        let mut found = true;
        let mut recipients: *mut AssetLockPlatformAddressRecipientFFI = std::ptr::null_mut();
        let mut count = 0usize;
        let call = |txid: *const [u8; 32],
                    found: *mut bool,
                    recipients: *mut *mut AssetLockPlatformAddressRecipientFFI,
                    count: *mut usize| unsafe {
            platform_wallet_get_asset_lock_platform_address_recipients(
                u64::MAX,
                txid,
                0,
                found,
                recipients,
                count,
            )
        };
        for code in [
            call(std::ptr::null(), &mut found, &mut recipients, &mut count).code,
            call(&txid, std::ptr::null_mut(), &mut recipients, &mut count).code,
            call(&txid, &mut found, std::ptr::null_mut(), &mut count).code,
            call(&txid, &mut found, &mut recipients, std::ptr::null_mut()).code,
        ] {
            assert_eq!(code, PlatformWalletFFIResultCode::ErrorNullPointer);
        }
        assert_eq!(
            call(&txid, &mut found, &mut recipients, &mut count).code,
            PlatformWalletFFIResultCode::NotFound
        );
    }
}
