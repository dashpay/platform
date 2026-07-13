//! `account_registrations` writer + keyless reader (platform-payment
//! registrations and the rehydration account-manifest oracle), plus the
//! provider key-material accounts and their `provider_platform_node_keys`
//! child rows.

use std::collections::BTreeMap;

use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::{
    AccountRegistrationEntry, ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
    ProviderKeyRegistrationBlob, ProviderPlatformNodePubKey,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

// PUBLIC material only: the account-registration xpub manifest reaching
// the `account_xpub_bytes` blob column.
impl_persistable_blob!(AccountRegistrationEntry);

// PUBLIC material only: the provider account's own-curve extended PUBLIC key
// (BLS operator / EdDSA platform node) reaching the same blob column. The type
// has no field that could carry signing material.
impl_persistable_blob!(ProviderKeyRegistrationBlob);

/// The persisted account manifest of one wallet: the secp256k1 accounts and
/// the provider key-material accounts, which carry a non-secp256k1 extended
/// public key and so cannot share one entry type.
#[derive(Debug, Clone, Default)]
pub struct AccountManifest {
    /// secp256k1 accounts, ordered by `(account_type, account_index)`.
    pub ecdsa: Vec<AccountRegistrationEntry>,
    /// BLS operator-key / EdDSA platform-node-key accounts, ordered by
    /// `account_type`.
    pub provider: Vec<ProviderKeyAccountEntry>,
}

impl AccountManifest {
    /// True when the wallet registered no account of either kind.
    pub fn is_empty(&self) -> bool {
        self.ecdsa.is_empty() && self.provider.is_empty()
    }
}

/// Decoded `platform_payment` account registration: the DIP-17 account
/// index and its extended public key, recovered from the bincode-serde
/// `AccountRegistrationEntry` stored in `account_xpub_bytes`.
pub(crate) type PlatformPaymentRegistration = (u32, ExtendedPubKey);

/// One `platform_payment` registration row decoded into
/// `(account_index, xpub)`.
fn decode_platform_payment_row(
    typed_index: i64,
    xpub_bytes: &[u8],
) -> Result<PlatformPaymentRegistration, WalletStorageError> {
    let typed_index = crate::sqlite::util::safe_cast::i64_to_u32(
        "account_registrations.account_index",
        typed_index,
    )?;
    let entry: AccountRegistrationEntry = blob::decode(xpub_bytes)?;
    // Callers select `WHERE account_type = 'platform_payment'`, so the decoded
    // blob must agree: a PlatformPayment account at the same index. A row whose
    // blob disagrees is corrupt / mis-bucketed, never fed to the oracle.
    if account_type_db_label(&entry.account_type) != "platform_payment"
        || account_index(&entry.account_type) != typed_index
    {
        return Err(WalletStorageError::AccountRegistrationEntryMismatch);
    }
    Ok((typed_index, entry.account_xpub))
}

/// Every `platform_payment` registration for one wallet, decoded into
/// `(account_index, xpub)`.
#[cfg(any(test, feature = "__test-helpers"))]
pub(crate) fn list_platform_payment_registrations(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<PlatformPaymentRegistration>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT account_index, length(account_xpub_bytes), account_xpub_bytes \
         FROM account_registrations \
         WHERE wallet_id = ?1 AND account_type = 'platform_payment' \
         ORDER BY account_index",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let idx: i64 = row.get(0)?;
        blob::check_size(row.get::<_, i64>(1)?)?;
        let bytes: Vec<u8> = row.get(2)?;
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
        "SELECT wallet_id, account_index, length(account_xpub_bytes), account_xpub_bytes \
         FROM account_registrations \
         WHERE account_type = 'platform_payment' \
         ORDER BY wallet_id, account_index",
    )?;
    let mut rows = stmt.query([])?;
    let mut out: BTreeMap<WalletId, Vec<PlatformPaymentRegistration>> = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let wid_bytes: Vec<u8> = row.get(0)?;
        let idx: i64 = row.get(1)?;
        let len = usize::try_from(row.get::<_, i64>(2)?).unwrap_or(usize::MAX);
        if len > blob::BLOB_SIZE_LIMIT_BYTES {
            return Err(WalletStorageError::BlobTooLarge {
                len_bytes: len,
                limit_bytes: blob::BLOB_SIZE_LIMIT_BYTES,
            });
        }
        let bytes: Vec<u8> = row.get(3)?;
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
    let mut stmt = tx.prepare_cached(UPSERT_ACCOUNT_SQL)?;
    for entry in entries {
        upsert_account_row(
            &mut stmt,
            wallet_id,
            &entry.account_type,
            blob::encode(entry)?,
        )?;
    }
    Ok(())
}

/// Upsert for one `account_registrations` row, shared by both writers — they
/// key on the same six-column PK and differ only in what they encode into
/// `account_xpub_bytes`. `key_class` and the DashPay `(user, friend)` identity
/// pair widen the PK so distinct accounts sharing `(account_type,
/// account_index)` don't overwrite each other; re-persisting one updates it.
const UPSERT_ACCOUNT_SQL: &str = "INSERT INTO account_registrations \
        (wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id, account_xpub_bytes) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
     ON CONFLICT(wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id) DO UPDATE SET \
        account_xpub_bytes = excluded.account_xpub_bytes";

/// Bind and execute [`UPSERT_ACCOUNT_SQL`]: the typed PK columns are derived
/// from `account_type` so they always mirror the blob the reader cross-checks
/// them against.
fn upsert_account_row(
    stmt: &mut rusqlite::CachedStatement<'_>,
    wallet_id: &WalletId,
    account_type: &AccountType,
    payload: Vec<u8>,
) -> Result<(), WalletStorageError> {
    let (user_identity_id, friend_identity_id) = account_dashpay_ids(account_type);
    stmt.execute(params![
        wallet_id.as_slice(),
        account_type_db_label(account_type),
        i64::from(account_index(account_type)),
        i64::from(account_key_class(account_type)),
        &user_identity_id[..],
        &friend_identity_id[..],
        payload,
    ])?;
    Ok(())
}

/// Persist the provider key-material accounts (BLS operator / EdDSA
/// platform node) into `account_registrations`, and each account's
/// pre-derived platform-node keys into `provider_platform_node_keys`.
///
/// The account row upserts on the same PK as [`apply_registrations`] — a
/// provider account is index-less, so `(wallet_id, account_type)` plus the
/// sentinel columns is its natural key and a re-persist must update, not
/// duplicate. The node-key batch is replaced wholesale per account
/// (delete-then-insert in the caller's tx): the changeset carries a
/// whole-batch snapshot captured at registration, never a delta.
pub fn apply_provider_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[ProviderKeyAccountEntry],
) -> Result<(), WalletStorageError> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut account_stmt = tx.prepare_cached(UPSERT_ACCOUNT_SQL)?;
    let mut clear_node_keys_stmt = tx.prepare_cached(
        "DELETE FROM provider_platform_node_keys WHERE wallet_id = ?1 AND account_type = ?2",
    )?;
    let mut node_key_stmt = tx.prepare_cached(
        "INSERT INTO provider_platform_node_keys \
                (wallet_id, account_type, key_index, public_key, node_id) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for entry in entries {
        let account_type = account_type_db_label(&entry.account_type);
        let payload = blob::encode(&ProviderKeyRegistrationBlob {
            account_type: entry.account_type,
            extended_public_key: entry.extended_public_key.clone(),
        })?;
        upsert_account_row(&mut account_stmt, wallet_id, &entry.account_type, payload)?;
        clear_node_keys_stmt.execute(params![wallet_id.as_slice(), account_type])?;
        for key in &entry.derived_platform_node_keys {
            node_key_stmt.execute(params![
                wallet_id.as_slice(),
                account_type,
                i64::from(key.index),
                &key.public_key[..],
                &key.node_id[..],
            ])?;
        }
    }
    Ok(())
}

/// Every `provider_platform_node_keys` row of one provider account, ordered
/// by `key_index` — the order the reader contracts to return (the pool is a
/// pre-derived batch, so index order is display order).
fn load_platform_node_keys(
    conn: &Connection,
    wallet_id: &WalletId,
    account_type: &str,
) -> Result<Vec<ProviderPlatformNodePubKey>, WalletStorageError> {
    // Both blob columns are fixed-width, so `length(<col>)` is read first
    // (O(1) from the row header) and gated before the Vec is materialized.
    let mut stmt = conn.prepare(
        "SELECT key_index, length(public_key), public_key, length(node_id), node_id \
         FROM provider_platform_node_keys \
         WHERE wallet_id = ?1 AND account_type = ?2 \
         ORDER BY key_index",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), account_type])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let index = crate::sqlite::util::safe_cast::i64_to_u32(
            "provider_platform_node_keys.key_index",
            row.get(0)?,
        )?;
        blob::check_fixed_width(
            row.get::<_, i64>(1)?,
            32,
            "provider_platform_node_keys.public_key",
        )?;
        let public_key: Vec<u8> = row.get(2)?;
        blob::check_fixed_width(
            row.get::<_, i64>(3)?,
            20,
            "provider_platform_node_keys.node_id",
        )?;
        let node_id: Vec<u8> = row.get(4)?;
        out.push(ProviderPlatformNodePubKey {
            index,
            public_key: public_key.as_slice().try_into().map_err(|_| {
                WalletStorageError::blob_decode("provider_platform_node_keys.public_key")
            })?,
            node_id: node_id.as_slice().try_into().map_err(|_| {
                WalletStorageError::blob_decode("provider_platform_node_keys.node_id")
            })?,
        });
    }
    Ok(out)
}

/// True when the extended public key's curve is the one its account type
/// mandates. The `account_type` column is the decode discriminator (same
/// convention as the FFI backend), so a row whose blob carries the other
/// curve is cross-curve confusion, not a decodable account.
fn provider_curve_matches_type(at: &AccountType, key: &ProviderKeyExtendedPubKey) -> bool {
    matches!(
        (at, key),
        (
            AccountType::ProviderOperatorKeys,
            ProviderKeyExtendedPubKey::Bls(_)
        ) | (
            AccountType::ProviderPlatformKeys,
            ProviderKeyExtendedPubKey::EdDSA(_)
        )
    )
}

/// Read the provider key-material accounts of one wallet, each with its
/// pre-derived platform-node keys. Ordered by `account_type`; a row that
/// fails to decode, contradicts its typed columns, or carries the wrong
/// curve for its account type is a hard [`WalletStorageError`].
fn load_provider_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<ProviderKeyAccountEntry>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT account_type, account_index, key_class, user_identity_id, friend_identity_id, \
                length(account_xpub_bytes), account_xpub_bytes FROM account_registrations \
         WHERE wallet_id = ?1 AND account_type IN ('provider_operator', 'provider_platform') \
         ORDER BY account_type",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let typed_type: String = row.get(0)?;
        let typed_index: i64 = row.get(1)?;
        let typed_key_class: i64 = row.get(2)?;
        let typed_user: Vec<u8> = row.get(3)?;
        let typed_friend: Vec<u8> = row.get(4)?;
        blob::check_size(row.get::<_, i64>(5)?)?;
        let payload: Vec<u8> = row.get(6)?;
        let entry = blob::decode::<ProviderKeyRegistrationBlob>(&payload)?;

        // Same typed-column cross-check the ECDSA reader applies, plus the
        // curve↔account-type agreement the provider rows add.
        let (blob_user, blob_friend) = account_dashpay_ids(&entry.account_type);
        let typed_index = crate::sqlite::util::safe_cast::i64_to_u32(
            "account_registrations.account_index",
            typed_index,
        )?;
        let typed_key_class = crate::sqlite::util::safe_cast::i64_to_u32(
            "account_registrations.key_class",
            typed_key_class,
        )?;
        if account_type_db_label(&entry.account_type) != typed_type.as_str()
            || account_index(&entry.account_type) != typed_index
            || account_key_class(&entry.account_type) != typed_key_class
            || blob_user.as_slice() != typed_user.as_slice()
            || blob_friend.as_slice() != typed_friend.as_slice()
            || !provider_curve_matches_type(&entry.account_type, &entry.extended_public_key)
        {
            return Err(WalletStorageError::ProviderKeyAccountEntryMismatch);
        }

        out.push(ProviderKeyAccountEntry {
            account_type: entry.account_type,
            extended_public_key: entry.extended_public_key,
            derived_platform_node_keys: load_platform_node_keys(conn, wallet_id, &typed_type)?,
        });
    }
    Ok(out)
}

/// Read every `account_registrations` row for `wallet_id` into a keyless
/// [`AccountManifest`] — the rehydration account-set oracle (which accounts to
/// re-derive + the per-account xpubs the wrong-account gate checks). PUBLIC
/// material only (xpub + account type), no `Wallet` minted. Each list is
/// ordered by its typed columns for determinism; a row that fails to decode is
/// a hard [`WalletStorageError`].
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<AccountManifest, WalletStorageError> {
    Ok(AccountManifest {
        ecdsa: load_ecdsa_state(conn, wallet_id)?,
        provider: load_provider_state(conn, wallet_id)?,
    })
}

/// The secp256k1 half of [`load_state`]: every row whose blob is an
/// [`AccountRegistrationEntry`].
fn load_ecdsa_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<AccountRegistrationEntry>, WalletStorageError> {
    // Select typed columns alongside the blob so we can cross-check them
    // against the decoded entry — a row whose blob disagrees with its indexed
    // columns is a sign of corruption or a schema bug and must be rejected
    // rather than silently mis-bucketed.
    // `length(account_xpub_bytes)` is read first (O(1) from the row header) so
    // an oversize blob is caught before the Vec is allocated.
    // The provider key-material rows are excluded by `account_type`: their
    // blob is a `ProviderKeyRegistrationBlob` over a non-secp256k1 curve and
    // would hard-error this decode.
    let mut stmt = conn.prepare(
        "SELECT account_type, account_index, key_class, user_identity_id, friend_identity_id, \
                length(account_xpub_bytes), account_xpub_bytes FROM account_registrations \
         WHERE wallet_id = ?1 \
           AND account_type NOT IN ('provider_operator', 'provider_platform') \
         ORDER BY account_type, account_index, key_class, user_identity_id, friend_identity_id",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let typed_type: String = row.get(0)?; // account_type TEXT
        let typed_index: i64 = row.get(1)?; // account_index INTEGER
        let typed_key_class: i64 = row.get(2)?; // key_class INTEGER
        let typed_user: Vec<u8> = row.get(3)?; // user_identity_id BLOB
        let typed_friend: Vec<u8> = row.get(4)?; // friend_identity_id BLOB
        blob::check_size(row.get::<_, i64>(5)?)?;
        let payload: Vec<u8> = row.get(6)?; // account_xpub_bytes BLOB
        let entry = blob::decode::<AccountRegistrationEntry>(&payload)?;
        // Cross-check every typed PK column vs the decoded blob so a
        // corruption that passes `PRAGMA integrity_check` is still caught
        // here rather than feeding a wrong account to the oracle.
        let blob_index = account_index(&entry.account_type);
        let blob_key_class = account_key_class(&entry.account_type);
        let (blob_user, blob_friend) = account_dashpay_ids(&entry.account_type);
        let typed_index = crate::sqlite::util::safe_cast::i64_to_u32(
            "account_registrations.account_index",
            typed_index,
        )?;
        let typed_key_class = crate::sqlite::util::safe_cast::i64_to_u32(
            "account_registrations.key_class",
            typed_key_class,
        )?;
        if account_type_db_label(&entry.account_type) != typed_type.as_str()
            || blob_index != typed_index
            || blob_key_class != typed_key_class
            || blob_user.as_slice() != typed_user.as_slice()
            || blob_friend.as_slice() != typed_friend.as_slice()
        {
            return Err(WalletStorageError::AccountRegistrationEntryMismatch);
        }
        out.push(entry);
    }
    Ok(out)
}

/// Source of truth for the `account_registrations.account_type` TEXT domain,
/// mirroring [`key_wallet::account::AccountType`].
/// `migrations/V001__initial.rs` interpolates it into the table's
/// `CHECK (account_type IN (...))`; `account_type_labels_match_enum` keeps it
/// in sync with [`account_type_db_label`].
///
/// `Standard` maps to two distinct labels by `StandardAccountType` variant
/// (`"standard_bip44"` / `"standard_bip32"`) so BIP44 and BIP32 standard
/// accounts with the same index never collide on their shared PK columns.
pub(crate) const ACCOUNT_TYPE_LABELS: &[&str] = &[
    "standard_bip44",
    "standard_bip32",
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

/// Stable database label for an `AccountType` variant (the `Debug` impl is not
/// a stable format; this match is the contract). An added upstream variant
/// fails this match's exhaustiveness check at compile time.
///
/// `Standard` maps to two distinct labels by `StandardAccountType` so BIP44
/// and BIP32 accounts with the same `index` never collapse onto the same PK.
pub(crate) fn account_type_db_label(at: &key_wallet::account::AccountType) -> &'static str {
    use key_wallet::account::{AccountType, StandardAccountType};
    match at {
        AccountType::Standard {
            standard_account_type: StandardAccountType::BIP44Account,
            ..
        } => "standard_bip44",
        AccountType::Standard {
            standard_account_type: StandardAccountType::BIP32Account,
            ..
        } => "standard_bip32",
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

/// Numeric account index embedded in an `AccountType`, persisted in the
/// `account_registrations.account_index` column.
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

/// Hardened `key_class` discriminator for `PlatformPayment`, persisted in the
/// `account_registrations.key_class` PK column. `0` for every other variant —
/// the sentinel "no key-class axis" value, matching the column default.
pub(crate) fn account_key_class(at: &key_wallet::account::AccountType) -> u32 {
    use key_wallet::account::AccountType;
    match at {
        AccountType::PlatformPayment { key_class, .. } => *key_class,
        _ => 0,
    }
}

/// DashPay `(user_identity_id, friend_identity_id)` discriminator pair — the
/// real account key for `DashpayReceivingFunds` / `DashpayExternalAccount`,
/// persisted in the matching PK columns. All-zero for every non-DashPay
/// variant (no identity axis), matching the column default.
pub(crate) fn account_dashpay_ids(at: &key_wallet::account::AccountType) -> ([u8; 32], [u8; 32]) {
    use key_wallet::account::AccountType;
    match at {
        AccountType::DashpayReceivingFunds {
            user_identity_id,
            friend_identity_id,
            ..
        }
        | AccountType::DashpayExternalAccount {
            user_identity_id,
            friend_identity_id,
            ..
        } => (*user_identity_id, *friend_identity_id),
        _ => ([0u8; 32], [0u8; 32]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Open an in-memory SQLite connection and run the full schema migration
    /// so tests can insert rows through the production table DDL.
    fn migrated_conn() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn
    }

    /// A fixed serialised extended public key for use in tests. Taken from the
    /// BIP-32 mainnet test vector so it is stable and round-trips correctly.
    fn test_xpub() -> key_wallet::bip32::ExtendedPubKey {
        key_wallet::bip32::ExtendedPubKey::decode(
            &hex::decode(
                "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D\
                 314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC\
                 2DF1050E2E8FF49C85C2",
            )
            .unwrap(),
        )
        .unwrap()
    }

    /// `load_state` must return `AccountRegistrationEntryMismatch` when the
    /// typed `account_type` column disagrees with the decoded blob.  The test
    /// inserts a row whose blob encodes a `PlatformPayment` entry but whose
    /// column is set to `identity_registration`, then verifies the mismatch
    /// is caught on the read path.
    #[test]
    fn load_state_rejects_account_type_column_mismatch() {
        let conn = migrated_conn();
        let w = [0x11u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();

        // Build a valid blob for PlatformPayment (account_index = 0).
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 0,
                key_class: 0,
            },
            account_xpub: test_xpub(),
        };
        let blob = blob::encode(&entry).unwrap();

        // Insert with a deliberately wrong `account_type` column label so
        // the typed column and the blob disagree.
        conn.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, 'identity_registration', 0, ?2)",
            rusqlite::params![&w[..], blob],
        )
        .unwrap();

        let err = load_state(&conn, &w).expect_err("load_state must fail on type mismatch");
        assert!(
            matches!(err, WalletStorageError::AccountRegistrationEntryMismatch),
            "expected AccountRegistrationEntryMismatch, got {err:?}"
        );
    }

    /// `load_state` must return `AccountRegistrationEntryMismatch` when the
    /// typed `account_index` column disagrees with the decoded blob, even when
    /// `account_type` matches.
    #[test]
    fn load_state_rejects_account_index_column_mismatch() {
        let conn = migrated_conn();
        let w = [0x22u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();

        // Blob encodes PlatformPayment at account index 0.
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 0,
                key_class: 0,
            },
            account_xpub: test_xpub(),
        };
        let blob = blob::encode(&entry).unwrap();

        // Column says account_index = 1 but blob says 0 — deliberate mismatch.
        conn.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, 'platform_payment', 1, ?2)",
            rusqlite::params![&w[..], blob],
        )
        .unwrap();

        let err = load_state(&conn, &w).expect_err("load_state must fail on index mismatch");
        assert!(
            matches!(err, WalletStorageError::AccountRegistrationEntryMismatch),
            "expected AccountRegistrationEntryMismatch, got {err:?}"
        );
    }

    /// Baseline: a consistent row (column and blob agree) round-trips cleanly.
    #[test]
    fn load_state_accepts_consistent_row() {
        let conn = migrated_conn();
        let w = [0x33u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 3,
                key_class: 0,
            },
            account_xpub: test_xpub(),
        };
        let blob = blob::encode(&entry).unwrap();
        conn.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, account_xpub_bytes) \
             VALUES (?1, 'platform_payment', 3, ?2)",
            rusqlite::params![&w[..], blob],
        )
        .unwrap();

        let loaded = load_state(&conn, &w)
            .expect("consistent row must load cleanly")
            .ecdsa;
        assert_eq!(loaded.len(), 1);
        assert!(matches!(
            loaded[0].account_type,
            key_wallet::account::AccountType::PlatformPayment { account: 3, .. }
        ));
    }

    /// Two `PlatformPayment` accounts sharing `(account_type, account_index)`
    /// but differing in `key_class` must both survive a persist — the widened
    /// PK keeps distinct key classes from collapsing onto one row (the
    /// data-loss bug this fix addresses).
    #[test]
    fn distinct_key_class_accounts_do_not_collide() {
        let mut conn = migrated_conn();
        let w = [0x44u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        let entry = |key_class: u32| AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 0,
                key_class,
            },
            account_xpub: test_xpub(),
        };
        {
            let tx = conn.transaction().unwrap();
            apply_registrations(&tx, &w, &[entry(0), entry(1)]).unwrap();
            tx.commit().unwrap();
        }
        let loaded = load_state(&conn, &w).expect("both key classes load").ecdsa;
        assert_eq!(loaded.len(), 2, "distinct key classes must both persist");
        let key_classes: HashSet<u32> = loaded
            .iter()
            .map(|e| match e.account_type {
                key_wallet::account::AccountType::PlatformPayment { key_class, .. } => key_class,
                _ => unreachable!("only PlatformPayment was inserted"),
            })
            .collect();
        assert_eq!(key_classes, HashSet::from([0, 1]));
    }

    /// Two `DashpayReceivingFunds` accounts at the same `index` but for
    /// different contacts (distinct `friend_identity_id`) must both survive —
    /// the per-contact identity pair is the real account key and must not
    /// collapse on the shared `(account_type, account_index)`.
    #[test]
    fn distinct_dashpay_friends_do_not_collide() {
        let mut conn = migrated_conn();
        let w = [0x55u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        let entry = |friend: [u8; 32]| AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::DashpayReceivingFunds {
                index: 0,
                user_identity_id: [0xAB; 32],
                friend_identity_id: friend,
            },
            account_xpub: test_xpub(),
        };
        {
            let tx = conn.transaction().unwrap();
            apply_registrations(&tx, &w, &[entry([0x01; 32]), entry([0x02; 32])]).unwrap();
            tx.commit().unwrap();
        }
        let loaded = load_state(&conn, &w).expect("both contacts load").ecdsa;
        assert_eq!(loaded.len(), 2, "distinct contacts must both persist");
        let friends: HashSet<[u8; 32]> = loaded
            .iter()
            .map(|e| match e.account_type {
                key_wallet::account::AccountType::DashpayReceivingFunds {
                    friend_identity_id,
                    ..
                } => friend_identity_id,
                _ => unreachable!("only DashpayReceivingFunds was inserted"),
            })
            .collect();
        assert_eq!(friends, HashSet::from([[0x01; 32], [0x02; 32]]));
    }

    /// Re-persisting the same account (identical full `AccountType`) updates in
    /// place rather than inserting a duplicate — the idempotent upsert the
    /// widened PK must preserve.
    #[test]
    fn idempotent_repersist_does_not_duplicate() {
        let mut conn = migrated_conn();
        let w = [0x66u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 2,
                key_class: 1,
            },
            account_xpub: test_xpub(),
        };
        for _ in 0..2 {
            let tx = conn.transaction().unwrap();
            apply_registrations(&tx, &w, std::slice::from_ref(&entry)).unwrap();
            tx.commit().unwrap();
        }
        let loaded = load_state(&conn, &w).expect("load").ecdsa;
        assert_eq!(loaded.len(), 1, "re-persist must not duplicate the row");
    }

    /// Every [`key_wallet::account::AccountType`] variant; the wildcard-free
    /// match below fails to compile if upstream adds one. `Standard` appears
    /// twice — once per `StandardAccountType` — because both map to distinct
    /// labels.
    fn all_account_type_variants() -> Vec<key_wallet::account::AccountType> {
        use key_wallet::account::{AccountType, StandardAccountType};
        let variants = vec![
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP32Account,
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

    /// The reader's SQL inlines these two labels (SQLite has no list
    /// binding), so a rename upstream must break here rather than silently
    /// route every provider row into the ECDSA decode path.
    #[test]
    fn provider_key_account_labels_match_sql_literals() {
        use key_wallet::account::AccountType;
        assert_eq!(
            account_type_db_label(&AccountType::ProviderOperatorKeys),
            "provider_operator"
        );
        assert_eq!(
            account_type_db_label(&AccountType::ProviderPlatformKeys),
            "provider_platform"
        );
    }

    /// All four provider account types collapse to the same index /
    /// key-class / DashPay sentinels, so `account_type` is the only thing
    /// keeping them off each other's PK. Two that collided would silently
    /// overwrite on upsert.
    #[test]
    fn provider_variants_have_distinct_pk_tuples() {
        use key_wallet::account::AccountType;
        let key = |at: AccountType| {
            (
                account_type_db_label(&at),
                account_index(&at),
                account_key_class(&at),
                account_dashpay_ids(&at),
            )
        };
        let keys: Vec<_> = [
            AccountType::ProviderVotingKeys,
            AccountType::ProviderOwnerKeys,
            AccountType::ProviderOperatorKeys,
            AccountType::ProviderPlatformKeys,
        ]
        .into_iter()
        .map(key)
        .collect();
        for k in &keys {
            assert_eq!((k.1, k.2, k.3), (0, 0, ([0u8; 32], [0u8; 32])));
        }
        let distinct: HashSet<_> = keys.iter().collect();
        assert_eq!(distinct.len(), 4, "provider PK tuples must not collide");
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
}
