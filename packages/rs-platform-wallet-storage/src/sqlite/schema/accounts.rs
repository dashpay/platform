//! `account_registrations` + `account_address_pools` writers.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::{AccountAddressPoolEntry, AccountRegistrationEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;
use crate::sqlite::schema::blob::BlobWriter;

pub fn apply_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountRegistrationEntry],
) -> Result<(), SqlitePersisterError> {
    for entry in entries {
        let account_type = format!("{:?}", entry.account_type);
        let account_index = account_index(&entry.account_type);
        // Use BIP-32 / DIP-14 binary encoding for the xpub — 78 or 107 bytes,
        // round-trippable via `ExtendedPubKey::decode`.
        let xpub_bytes = entry.account_xpub.encode();
        tx.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, account_type, account_index) DO UPDATE SET \
                account_xpub_bytes = excluded.account_xpub_bytes",
            params![
                wallet_id.as_slice(),
                account_type,
                account_index as i64,
                xpub_bytes,
            ],
        )?;
    }
    Ok(())
}

pub fn apply_pools(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountAddressPoolEntry],
) -> Result<(), SqlitePersisterError> {
    for entry in entries {
        let account_type = format!("{:?}", entry.account_type);
        let account_index = account_index(&entry.account_type);
        let pool_type = format!("{:?}", entry.pool_type);
        // `AddressInfo` is `Debug + Clone` only upstream — capture the
        // raw count so consumers can detect a non-empty pool. Full
        // round-trips are deferred until upstream gains serde.
        let mut w = BlobWriter::new();
        w.u64(entry.addresses.len() as u64);
        tx.execute(
            "INSERT INTO account_address_pools \
                (wallet_id, account_type, account_index, pool_type, snapshot_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, account_type, account_index, pool_type) DO UPDATE SET \
                snapshot_blob = excluded.snapshot_blob",
            params![
                wallet_id.as_slice(),
                account_type,
                account_index as i64,
                pool_type,
                w.finish(),
            ],
        )?;
    }
    Ok(())
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
