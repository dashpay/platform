//! `account_registrations` + `account_address_pools` writers.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::{AccountAddressPoolEntry, AccountRegistrationEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

pub fn apply_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountRegistrationEntry],
) -> Result<(), WalletStorageError> {
    for entry in entries {
        let account_type = account_type_db_label(&entry.account_type);
        let account_index = account_index(&entry.account_type);
        // `account_xpub_bytes` carries the bincode-serde encoded
        // `AccountRegistrationEntry` (xpub + account_type). The
        // separate `account_type` / `account_index` columns mirror
        // the entry for direct SQL lookups.
        let payload = blob::encode(entry)?;
        tx.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, account_type, account_index) DO UPDATE SET \
                account_xpub_bytes = excluded.account_xpub_bytes",
            params![
                wallet_id.as_slice(),
                account_type,
                i64::from(account_index),
                payload,
            ],
        )?;
    }
    Ok(())
}

pub fn apply_pools(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountAddressPoolEntry],
) -> Result<(), WalletStorageError> {
    for entry in entries {
        let account_type = account_type_db_label(&entry.account_type);
        let account_index = account_index(&entry.account_type);
        let pool_type = pool_type_db_label(&entry.pool_type);
        let payload = blob::encode(entry)?;
        tx.execute(
            "INSERT INTO account_address_pools \
                (wallet_id, account_type, account_index, pool_type, snapshot_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, account_type, account_index, pool_type) DO UPDATE SET \
                snapshot_blob = excluded.snapshot_blob",
            params![
                wallet_id.as_slice(),
                account_type,
                i64::from(account_index),
                pool_type,
                payload,
            ],
        )?;
    }
    Ok(())
}

/// Stable database label for an `AccountType` variant.
///
/// Used for the `account_type` text column on `account_registrations`,
/// `account_address_pools`, and `core_derived_addresses`. The
/// `Debug` impl on `AccountType` is NOT a stable serialisation
/// format; this match is the contract. Variants identical in
/// label are distinguished by the companion `account_index` column.
///
/// Adding a variant to upstream `AccountType` makes this match
/// exhaustive-check fail at compile time, forcing an explicit label
/// decision rather than silent garbage.
pub(crate) fn account_type_db_label(at: &key_wallet::account::AccountType) -> &'static str {
    use key_wallet::account::AccountType;
    match at {
        AccountType::Standard { .. } => "standard",
        AccountType::CoinJoin { .. } => "coinjoin",
        AccountType::IdentityRegistration => "identity_registration",
        AccountType::IdentityTopUp { .. } => "identity_topup",
        AccountType::IdentityTopUpNotBoundToIdentity => "identity_topup_unbound",
        AccountType::IdentityInvitation => "identity_invitation",
        AccountType::AssetLockAddressTopUp => "asset_lock_address_topup",
        AccountType::AssetLockShieldedAddressTopUp => "asset_lock_shielded_topup",
        AccountType::ProviderVotingKeys => "provider_voting",
        AccountType::ProviderOwnerKeys => "provider_owner",
        AccountType::ProviderOperatorKeys => "provider_operator",
        AccountType::ProviderPlatformKeys => "provider_platform",
        AccountType::DashpayReceivingFunds { .. } => "dashpay_receiving",
        AccountType::DashpayExternalAccount { .. } => "dashpay_external",
        AccountType::PlatformPayment { .. } => "platform_payment",
    }
}

/// Stable database label for an `AddressPoolType` variant.
pub(crate) fn pool_type_db_label(
    pool: &key_wallet::managed_account::address_pool::AddressPoolType,
) -> &'static str {
    use key_wallet::managed_account::address_pool::AddressPoolType;
    match pool {
        AddressPoolType::External => "external",
        AddressPoolType::Internal => "internal",
        AddressPoolType::Absent => "absent",
        AddressPoolType::AbsentHardened => "absent_hardened",
    }
}

fn account_index(at: &key_wallet::account::AccountType) -> u32 {
    use key_wallet::account::AccountType;
    match at {
        AccountType::Standard { index, .. } => *index,
        AccountType::CoinJoin { index } => *index,
        AccountType::IdentityRegistration => 0,
        AccountType::IdentityTopUp { registration_index } => *registration_index,
        AccountType::IdentityTopUpNotBoundToIdentity => 0,
        AccountType::IdentityInvitation => 0,
        AccountType::AssetLockAddressTopUp => 0,
        AccountType::AssetLockShieldedAddressTopUp => 0,
        AccountType::ProviderVotingKeys => 0,
        AccountType::ProviderOwnerKeys => 0,
        AccountType::ProviderOperatorKeys => 0,
        AccountType::ProviderPlatformKeys => 0,
        AccountType::DashpayReceivingFunds { index, .. } => *index,
        AccountType::DashpayExternalAccount { index, .. } => *index,
        AccountType::PlatformPayment { account, .. } => *account,
    }
}
