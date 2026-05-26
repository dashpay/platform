//! `account_registrations` + `account_address_pools` writers and the
//! keyless account-manifest reader.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::{AccountAddressPoolEntry, AccountRegistrationEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

pub fn apply_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountRegistrationEntry],
) -> Result<(), WalletStorageError> {
    if entries.is_empty() {
        return Ok(());
    }
    // `account_xpub_bytes` carries the bincode-serde encoded
    // `AccountRegistrationEntry` (xpub + account_type). The
    // separate `account_type` / `account_index` columns mirror
    // the entry for direct SQL lookups.
    let mut stmt = tx.prepare_cached(
        "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id, account_type, account_index) DO UPDATE SET \
                account_xpub_bytes = excluded.account_xpub_bytes",
    )?;
    for entry in entries {
        let account_type = account_type_db_label(&entry.account_type);
        let account_index = account_index(&entry.account_type);
        let payload = blob::encode(entry)?;
        stmt.execute(params![
            wallet_id.as_slice(),
            account_type,
            i64::from(account_index),
            payload,
        ])?;
    }
    Ok(())
}

pub fn apply_pools(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountAddressPoolEntry],
) -> Result<(), WalletStorageError> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut stmt = tx.prepare_cached(
        "INSERT INTO account_address_pools \
                (wallet_id, account_type, account_index, pool_type, snapshot_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(wallet_id, account_type, account_index, pool_type) DO UPDATE SET \
                snapshot_blob = excluded.snapshot_blob",
    )?;
    for entry in entries {
        let account_type = account_type_db_label(&entry.account_type);
        let account_index = account_index(&entry.account_type);
        let pool_type = pool_type_db_label(&entry.pool_type);
        let payload = blob::encode(entry)?;
        stmt.execute(params![
            wallet_id.as_slice(),
            account_type,
            i64::from(account_index),
            pool_type,
            payload,
        ])?;
    }
    Ok(())
}

/// Read every `account_registrations` row for `wallet_id` back into a
/// keyless [`AccountRegistrationEntry`] manifest.
///
/// This is the account-set oracle for rehydration: it dictates which
/// accounts must be re-derived and supplies the per-account xpubs the
/// wrong-account gate cross-checks against. It mints no `Wallet` — the
/// `account_xpub_bytes` blob carries only the public xpub plus the
/// account type (PUBLIC material only).
///
/// Rows are returned ordered by `(account_type, account_index)` so the
/// manifest is deterministic across reopens. Any row whose blob fails
/// to decode is a hard, typed [`WalletStorageError`] — corruption is
/// never silently dropped.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<AccountRegistrationEntry>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT account_xpub_bytes FROM account_registrations \
         WHERE wallet_id = ?1 ORDER BY account_type, account_index",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        row.get::<_, Vec<u8>>(0)
    })?;
    let mut out = Vec::new();
    for r in rows {
        let payload = r?;
        out.push(blob::decode::<AccountRegistrationEntry>(&payload)?);
    }
    Ok(out)
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

/// Numeric account index embedded in an `AccountType`.
///
/// Persisted in the `account_index` column of `account_registrations`,
/// `account_address_pools`, and `core_derived_addresses` (the last of
/// which is the script→account lookup the UTXO writer joins against).
pub(crate) fn account_index(at: &key_wallet::account::AccountType) -> u32 {
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
