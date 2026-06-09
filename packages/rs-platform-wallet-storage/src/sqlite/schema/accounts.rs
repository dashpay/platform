//! `account_registrations` + `account_address_pools` writers + readers
//! (platform-payment registrations and the keyless account-manifest
//! reader).

use std::collections::BTreeMap;

use key_wallet::bip32::ExtendedPubKey;
use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::{AccountAddressPoolEntry, AccountRegistrationEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: account-manifest types (account xpubs / pool
// snapshots) reaching `_blob` columns.
impl_persistable_blob!(AccountRegistrationEntry, AccountAddressPoolEntry);

/// Decoded `platform_payment` account registration: the DIP-17 account
/// index and its extended public key, recovered from the bincode-serde
/// `AccountRegistrationEntry` stored in `account_xpub_bytes`.
pub(crate) type PlatformPaymentRegistration = (u32, ExtendedPubKey);

/// One `platform_payment` registration row decoded into
/// `(account_index, xpub)`.
fn decode_platform_payment_row(
    account_index: i64,
    xpub_bytes: &[u8],
) -> Result<PlatformPaymentRegistration, WalletStorageError> {
    let account_index = crate::sqlite::util::safe_cast::i64_to_u32(
        "account_registrations.account_index",
        account_index,
    )?;
    let entry: AccountRegistrationEntry = blob::decode(xpub_bytes)?;
    Ok((account_index, entry.account_xpub))
}

/// Every `platform_payment` registration for one wallet, decoded into
/// `(account_index, xpub)`.
#[cfg(any(test, feature = "__test-helpers"))]
pub(crate) fn list_platform_payment_registrations(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<PlatformPaymentRegistration>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT account_index, account_xpub_bytes FROM account_registrations \
         WHERE wallet_id = ?1 AND account_type = 'platform_payment' \
         ORDER BY account_index",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (idx, bytes) = r?;
        out.push(decode_platform_payment_row(idx, &bytes)?);
    }
    Ok(out)
}

/// Bulk variant of [`list_platform_payment_registrations`]: every
/// wallet's `platform_payment` registrations in one scan, grouped by
/// `wallet_id`. Used by `load()` to avoid a per-wallet registrations
/// query.
pub(crate) fn all_platform_payment_registrations(
    conn: &Connection,
) -> Result<BTreeMap<WalletId, Vec<PlatformPaymentRegistration>>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT wallet_id, account_index, account_xpub_bytes FROM account_registrations \
         WHERE account_type = 'platform_payment' \
         ORDER BY wallet_id, account_index",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Vec<u8>>(2)?,
        ))
    })?;
    let mut out: BTreeMap<WalletId, Vec<PlatformPaymentRegistration>> = BTreeMap::new();
    for r in rows {
        let (wid_bytes, idx, bytes) = r?;
        let wallet_id = <[u8; 32]>::try_from(wid_bytes.as_slice()).map_err(|_| {
            WalletStorageError::InvalidWalletIdLength {
                actual: wid_bytes.len(),
            }
        })?;
        out.entry(wallet_id)
            .or_default()
            .push(decode_platform_payment_row(idx, &bytes)?);
    }
    Ok(out)
}

pub fn apply_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountRegistrationEntry],
) -> Result<(), WalletStorageError> {
    if entries.is_empty() {
        return Ok(());
    }
    // `account_xpub_bytes` holds the encoded `AccountRegistrationEntry`; the
    // separate `account_type` / `account_index` columns mirror it for SQL.
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
        // Mirror every snapshot address into `core_derived_addresses` (same
        // tx) so a UTXO landing on a pool address resolves its account even
        // when no live derive event preceded it (genesis-rescan). The shared
        // helper keeps these rows identical to the live derive path.
        for info in &entry.addresses {
            let address = info.address.to_string();
            let path =
                crate::sqlite::schema::core_state::derivation_path_label(pool_type, info.index);
            crate::sqlite::schema::core_state::upsert_derived_address_row(
                tx,
                wallet_id,
                account_type,
                i64::from(account_index),
                &address,
                &path,
                info.used,
            )?;
        }
    }
    Ok(())
}

/// Read every `account_registrations` row for `wallet_id` into a keyless
/// [`AccountRegistrationEntry`] manifest — the rehydration account-set oracle
/// (which accounts to re-derive + the per-account xpubs the wrong-account gate
/// checks). PUBLIC material only (xpub + account type), no `Wallet` minted.
/// Ordered by `(account_type, account_index)` for determinism; a row that
/// fails to decode is a hard [`WalletStorageError`].
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

/// Source of truth for the `account_type` TEXT domain across
/// `account_registrations`, `account_address_pools`, and
/// `core_derived_addresses`, mirroring [`key_wallet::account::AccountType`].
/// `migrations/V001__initial.rs` interpolates it into each table's
/// `CHECK (account_type IN (...))`; `account_type_labels_match_enum` keeps it
/// in sync with [`account_type_db_label`].
pub(crate) const ACCOUNT_TYPE_LABELS: &[&str] = &[
    "standard",
    "coinjoin",
    "identity_registration",
    "identity_topup",
    "identity_topup_unbound",
    "identity_invitation",
    "asset_lock_address_topup",
    "asset_lock_shielded_topup",
    "provider_voting",
    "provider_owner",
    "provider_operator",
    "provider_platform",
    "dashpay_receiving",
    "dashpay_external",
    "platform_payment",
];

/// Single source of truth for the `account_address_pools.pool_type`
/// TEXT-column domain.
///
/// Mirrors every variant of
/// [`key_wallet::managed_account::address_pool::AddressPoolType`]
/// (writer side: [`pool_type_db_label`]). See [`ACCOUNT_TYPE_LABELS`]
/// for the broader rationale and the parity-test contract.
pub(crate) const POOL_TYPE_LABELS: &[&str] = &["external", "internal", "absent", "absent_hardened"];

/// Stable database label for an `AccountType` variant (the `Debug` impl is not
/// a stable format; this match is the contract). Variants sharing a label are
/// distinguished by the companion `account_index` column. An added upstream
/// variant fails this match's exhaustiveness check at compile time.
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

/// Numeric account index embedded in an `AccountType`, persisted in the
/// `account_index` column of `account_registrations`, `account_address_pools`,
/// and `core_derived_addresses`.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every [`key_wallet::account::AccountType`] variant; the wildcard-free
    /// match below fails to compile if upstream adds one.
    fn all_account_type_variants() -> Vec<key_wallet::account::AccountType> {
        use key_wallet::account::{AccountType, StandardAccountType};
        let variants = vec![
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AccountType::CoinJoin { index: 0 },
            AccountType::IdentityRegistration,
            AccountType::IdentityTopUp {
                registration_index: 0,
            },
            AccountType::IdentityTopUpNotBoundToIdentity,
            AccountType::IdentityInvitation,
            AccountType::AssetLockAddressTopUp,
            AccountType::AssetLockShieldedAddressTopUp,
            AccountType::ProviderVotingKeys,
            AccountType::ProviderOwnerKeys,
            AccountType::ProviderOperatorKeys,
            AccountType::ProviderPlatformKeys,
            AccountType::DashpayReceivingFunds {
                index: 0,
                user_identity_id: [0u8; 32],
                friend_identity_id: [0u8; 32],
            },
            AccountType::DashpayExternalAccount {
                index: 0,
                user_identity_id: [0u8; 32],
                friend_identity_id: [0u8; 32],
            },
            AccountType::PlatformPayment {
                account: 0,
                key_class: 0,
            },
        ];
        for v in &variants {
            match v {
                AccountType::Standard { .. }
                | AccountType::CoinJoin { .. }
                | AccountType::IdentityRegistration
                | AccountType::IdentityTopUp { .. }
                | AccountType::IdentityTopUpNotBoundToIdentity
                | AccountType::IdentityInvitation
                | AccountType::AssetLockAddressTopUp
                | AccountType::AssetLockShieldedAddressTopUp
                | AccountType::ProviderVotingKeys
                | AccountType::ProviderOwnerKeys
                | AccountType::ProviderOperatorKeys
                | AccountType::ProviderPlatformKeys
                | AccountType::DashpayReceivingFunds { .. }
                | AccountType::DashpayExternalAccount { .. }
                | AccountType::PlatformPayment { .. } => {}
            }
        }
        variants
    }

    fn all_pool_type_variants() -> Vec<key_wallet::managed_account::address_pool::AddressPoolType> {
        use key_wallet::managed_account::address_pool::AddressPoolType;
        let variants = vec![
            AddressPoolType::External,
            AddressPoolType::Internal,
            AddressPoolType::Absent,
            AddressPoolType::AbsentHardened,
        ];
        for v in &variants {
            match v {
                AddressPoolType::External
                | AddressPoolType::Internal
                | AddressPoolType::Absent
                | AddressPoolType::AbsentHardened => {}
            }
        }
        variants
    }

    #[test]
    fn account_type_labels_match_enum() {
        let from_writer: HashSet<&'static str> = all_account_type_variants()
            .iter()
            .map(account_type_db_label)
            .collect();
        let from_const: HashSet<&'static str> = ACCOUNT_TYPE_LABELS.iter().copied().collect();
        assert_eq!(
            from_writer, from_const,
            "ACCOUNT_TYPE_LABELS ({:?}) drifted from account_type_db_label codomain ({:?})",
            from_const, from_writer
        );
    }

    #[test]
    fn pool_type_labels_match_enum() {
        let from_writer: HashSet<&'static str> = all_pool_type_variants()
            .iter()
            .map(pool_type_db_label)
            .collect();
        let from_const: HashSet<&'static str> = POOL_TYPE_LABELS.iter().copied().collect();
        assert_eq!(
            from_writer, from_const,
            "POOL_TYPE_LABELS ({:?}) drifted from pool_type_db_label codomain ({:?})",
            from_const, from_writer
        );
    }
}
