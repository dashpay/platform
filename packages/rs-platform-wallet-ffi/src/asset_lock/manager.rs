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
    /// Whether the lock was built to pay someone other than the wallet and
    /// recorded what it pays (its payment intent; read it with
    /// `platform_wallet_get_asset_lock_payment_intent`). Such a lock resumes only
    /// through the flow it was built for, with the recorded recipients; any
    /// other resume is refused with `ErrorAssetLockRecipientMismatch`. False
    /// for owned fundings and for locks tracked before intents were recorded.
    pub has_payment_intent: bool,
}

/// Kind of a payment intent: the recipients are third-party-capable Platform
/// addresses (`fund_from_asset_lock_external`).
pub const ASSET_LOCK_PAYMENT_INTENT_KIND_PLATFORM_ADDRESSES: u8 = 1;

/// One recipient of a payment intent.
#[repr(C)]
pub struct AssetLockPaymentRecipientFFI {
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
                    has_payment_intent: lock.payment_intent.is_some(),
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

/// Read the payment intent a tracked asset lock of the platform wallet
/// `wallet_handle` recorded when it was built to pay someone other than the
/// wallet.
///
/// On success `out_found` says whether the lock carries one. When it does,
/// `out_kind` is its kind (`ASSET_LOCK_PAYMENT_INTENT_KIND_*`) and
/// `out_recipients` / `out_count` a heap-allocated recipient array in the
/// order the wallet stores it; free it with
/// `platform_wallet_free_asset_lock_payment_recipients`. A lock that is not tracked
/// is reported as not found. To resume the lock, pass exactly these
/// recipients back to the flow the kind names.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_asset_lock_payment_intent(
    wallet_handle: Handle,
    txid: *const [u8; 32],
    vout: u32,
    out_found: *mut bool,
    out_kind: *mut u8,
    out_recipients: *mut *mut AssetLockPaymentRecipientFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    use platform_wallet::changeset::AssetLockPaymentIntent;

    check_ptr!(txid);
    check_ptr!(out_found);
    check_ptr!(out_kind);
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
    let intent = runtime().block_on(asset_locks.tracked_payment_intent(&out_point));

    *out_found = intent.is_some();
    *out_kind = 0;
    *out_recipients = std::ptr::null_mut();
    *out_count = 0;
    if let Some(intent) = intent {
        let recipients: Vec<AssetLockPaymentRecipientFFI> = match intent {
            AssetLockPaymentIntent::PlatformAddresses(recipients) => {
                *out_kind = ASSET_LOCK_PAYMENT_INTENT_KIND_PLATFORM_ADDRESSES;
                recipients
                    .into_iter()
                    .map(|(address, amount)| {
                        let mut bytes = [0u8; 21];
                        bytes.copy_from_slice(&address.to_bytes());
                        AssetLockPaymentRecipientFFI {
                            address: bytes,
                            has_amount: amount.is_some(),
                            amount_credits: amount.unwrap_or(0),
                        }
                    })
                    .collect()
            }
        };
        *out_count = recipients.len();
        if !recipients.is_empty() {
            *out_recipients = Box::into_raw(recipients.into_boxed_slice()).cast();
        }
    }
    PlatformWalletFFIResult::ok()
}

/// Free a recipient array returned by `platform_wallet_get_asset_lock_payment_intent`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_free_asset_lock_payment_recipients(
    recipients: *mut AssetLockPaymentRecipientFFI,
    count: usize,
) {
    if !recipients.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(recipients, count));
    }
}
