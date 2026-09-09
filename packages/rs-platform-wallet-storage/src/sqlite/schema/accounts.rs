//! `account_registrations` writer + keyless reader (platform-payment
//! registrations and the rehydration account-manifest oracle), including
//! provider key-material accounts.

use std::collections::BTreeMap;

use key_wallet::account::AccountType;
use key_wallet::bip32::ExtendedPubKey;
use rusqlite::{params, Connection, Transaction};
use sha2::{Digest, Sha256};

use platform_wallet::changeset::{
    AccountRegistrationEntry, ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;

/// Bind the exact stored blob to its wallet id. Store generation is excluded
/// because legitimate backup restores rotate it.
fn account_registration_checksum(wallet_id: &WalletId, account_xpub_bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(wallet_id.as_slice());
    hasher.update(account_xpub_bytes);
    hasher.finalize().into()
}

// PUBLIC material only: the account-registration xpub manifest reaching
// the `account_xpub_bytes` blob column.
impl_persistable_blob!(AccountRegistrationEntry);

// PUBLIC material only: the provider account's own-curve extended PUBLIC key
// (BLS operator / EdDSA platform node) reaching the same blob column. The type
// has no field that could carry signing material.
impl_persistable_blob!(ProviderKeyAccountEntry);

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
    typed_key_class: i64,
    xpub_bytes: &[u8],
) -> Result<PlatformPaymentRegistration, WalletStorageError> {
    let typed_index = crate::sqlite::util::safe_cast::i64_to_u32(
        "account_registrations.account_index",
        typed_index,
    )?;
    let typed_key_class = crate::sqlite::util::safe_cast::i64_to_u32(
        "account_registrations.key_class",
        typed_key_class,
    )?;
    let entry: AccountRegistrationEntry = blob::decode(xpub_bytes)?;
    // Callers select `WHERE account_type = 'platform_payment'`, so the decoded
    // blob must agree: a PlatformPayment account at the same index AND key_class.
    // key_class is a real discriminator — two PlatformPayment accounts can share
    // `(account_type, account_index)` and differ only here (the widened PK exists
    // for exactly that) — so cross-check it like `load_state` does, or the oracle
    // could hand back a row keyed by a different key class than its blob names.
    if account_type_db_label(&entry.account_type) != "platform_payment"
        || account_index(&entry.account_type) != typed_index
        || account_key_class(&entry.account_type) != typed_key_class
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
        "SELECT account_index, key_class, length(account_xpub_bytes), account_xpub_bytes \
         FROM account_registrations \
         WHERE wallet_id = ?1 AND account_type = 'platform_payment' \
         ORDER BY account_index",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let idx: i64 = row.get(0)?;
        let key_class: i64 = row.get(1)?;
        blob::check_size(row.get::<_, i64>(2)?)?;
        let bytes: Vec<u8> = row.get(3)?;
        out.push(decode_platform_payment_row(idx, key_class, &bytes)?);
    }
    Ok(out)
}

/// Bulk variant of [`list_platform_payment_registrations`]: every
/// wallet's `platform_payment` registrations in one scan, grouped by
/// `wallet_id`. Used by `load()` to avoid a per-wallet registrations
/// query.
pub(crate) fn all_platform_payment_registrations(
    conn: &Connection,
) -> Result<
    BTreeMap<WalletId, Result<Vec<PlatformPaymentRegistration>, WalletStorageError>>,
    WalletStorageError,
> {
    let mut stmt = conn.prepare(
        "SELECT length(wallet_id), wallet_id, account_index, key_class, \
                length(account_xpub_bytes), account_xpub_bytes, checksum \
         FROM account_registrations \
         WHERE account_type = 'platform_payment' \
         ORDER BY wallet_id, account_index",
    )?;
    let mut rows = stmt.query([])?;
    let mut out: BTreeMap<WalletId, Result<Vec<PlatformPaymentRegistration>, WalletStorageError>> =
        BTreeMap::new();
    while let Some(row) = rows.next()? {
        blob::check_fixed_width(row.get::<_, i64>(0)?, 32, "account_registrations.wallet_id")?;
        let wid_bytes: Vec<u8> = row.get(1)?;
        let idx: i64 = row.get(2)?;
        let key_class: i64 = row.get(3)?;
        let payload_width: i64 = row.get(4)?;
        let bytes: Vec<u8> = row.get(5)?;
        // An id that is not 32 bytes belongs to no wallet, so it stays
        // file-fatal; everything after it is attributable to one.
        let wallet_id = super::id32("account_registrations.wallet_id", &wid_bytes)?;
        let stored_checksum: Option<Vec<u8>> = row.get(6)?;
        let decoded = blob::check_size(payload_width)
            .and_then(|()| {
                if stored_checksum.as_deref()
                    == Some(account_registration_checksum(&wallet_id, &bytes).as_slice())
                {
                    Ok(())
                } else {
                    Err(WalletStorageError::ManifestIntegrityMismatch)
                }
            })
            .and_then(|()| decode_platform_payment_row(idx, key_class, &bytes));
        match decoded {
            // A wallet already recorded as failed keeps its first cause;
            // its remaining rows cannot change the outcome.
            Ok(decoded) => {
                if let Ok(rows) = out.entry(wallet_id).or_insert_with(|| Ok(Vec::new())) {
                    rows.push(decoded);
                }
            }
            Err(err) => {
                let slot = out.entry(wallet_id).or_insert_with(|| Ok(Vec::new()));
                if slot.is_ok() {
                    *slot = Err(err);
                }
            }
        }
    }
    Ok(out)
}

/// Persist ordinary secp256k1 account registrations for one wallet.
///
/// # Errors
///
/// Returns [`WalletStorageError::ProviderKeyAccountEntryMismatch`] if a
/// provider key-material account is submitted through this ECDSA writer.
/// Returns another [`WalletStorageError`] if an entry cannot be encoded or the
/// database write fails.
pub fn apply_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[AccountRegistrationEntry],
) -> Result<(), WalletStorageError> {
    if entries.is_empty() {
        return Ok(());
    }
    if entries
        .iter()
        .any(|entry| is_provider_key_material(&entry.account_type))
    {
        return Err(WalletStorageError::ProviderKeyAccountEntryMismatch);
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

fn is_provider_key_material(account_type: &AccountType) -> bool {
    match account_type {
        AccountType::ProviderOperatorKeys => true,
        AccountType::ProviderPlatformKeys => true,
        AccountType::Standard { .. } => false,
        AccountType::CoinJoin { .. } => false,
        AccountType::IdentityRegistration => false,
        AccountType::IdentityTopUp { .. } => false,
        AccountType::IdentityTopUpNotBoundToIdentity => false,
        AccountType::IdentityInvitation => false,
        AccountType::AssetLockAddressTopUp => false,
        AccountType::AssetLockShieldedAddressTopUp => false,
        AccountType::ProviderVotingKeys => false,
        AccountType::ProviderOwnerKeys => false,
        AccountType::DashpayReceivingFunds { .. } => false,
        AccountType::DashpayExternalAccount { .. } => false,
        AccountType::PlatformPayment { .. } => false,
    }
}

/// Upsert for an ordinary secp256k1 `account_registrations` row.
const UPSERT_ACCOUNT_SQL: &str = "INSERT INTO account_registrations \
        (wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id, account_xpub_bytes, checksum) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
     ON CONFLICT(wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id) DO UPDATE SET \
        account_xpub_bytes = excluded.account_xpub_bytes, checksum = excluded.checksum";

/// Insert a provider key-material account without overwriting persisted bytes.
const UPSERT_PROVIDER_ACCOUNT_SQL: &str = "INSERT INTO account_registrations \
        (wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id, account_xpub_bytes, checksum) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
     ON CONFLICT(wallet_id, account_type, account_index, key_class, \
         user_identity_id, friend_identity_id) DO NOTHING";

/// Bind and execute an account-registration statement. The typed PK columns
/// derive from `account_type` here so writer and reader cross-checks stay aligned.
fn upsert_account_row(
    stmt: &mut rusqlite::CachedStatement<'_>,
    wallet_id: &WalletId,
    account_type: &AccountType,
    payload: Vec<u8>,
) -> Result<(), WalletStorageError> {
    let (user_identity_id, friend_identity_id) = account_dashpay_ids(account_type);
    let checksum = account_registration_checksum(wallet_id, &payload);
    stmt.execute(params![
        wallet_id.as_slice(),
        account_type_db_label(account_type),
        i64::from(account_index(account_type)),
        i64::from(account_key_class(account_type)),
        &user_identity_id[..],
        &friend_identity_id[..],
        payload,
        checksum.as_slice(),
    ])?;
    Ok(())
}

/// Persist provider key-material accounts into `account_registrations`.
///
/// The account row uses the same PK as [`apply_registrations`]: a provider
/// account is index-less, so `(wallet_id, account_type)` plus the sentinel
/// columns is its natural key. Re-persisting an identical row is a no-op.
///
/// # Errors
///
/// [`WalletStorageError::ProviderKeyAccountEntryMismatch`] if an entry pairs an
/// `account_type` with the wrong curve — the same invariant the reader enforces,
/// checked here so a mis-paired entry cannot upsert onto (and destroy) the
/// ECDSA account sharing this table's PK space.
///
/// [`WalletStorageError::ProviderKeyAccountConflict`] if two entries in one call
/// claim the same account with **different** extended public keys, or if an
/// incoming key differs from the one already persisted for that account.
pub fn apply_provider_registrations(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entries: &[ProviderKeyAccountEntry],
) -> Result<(), WalletStorageError> {
    if entries.is_empty() {
        return Ok(());
    }
    // Validate the batch before write SQL runs so rejection cannot leave a
    // sibling half-applied independently of the caller's transaction discipline.
    let mut encoded: Vec<(&ProviderKeyAccountEntry, &'static str, Vec<u8>)> =
        Vec::with_capacity(entries.len());
    for entry in entries {
        if !provider_curve_matches_type(&entry.account_type, &entry.extended_public_key) {
            return Err(WalletStorageError::ProviderKeyAccountEntryMismatch);
        }
        let label = account_type_db_label(&entry.account_type);
        let payload = blob::encode(&ProviderKeyAccountEntry {
            account_type: entry.account_type,
            extended_public_key: entry.extended_public_key.clone(),
        })?;
        // Two entries for one account are expected — `Merge` is append-only, so
        // a re-emitted registration can ride the same flush. Identical ones are
        // equivalent. Two that disagree about the account's own xpub are a
        // contradiction no merge semantic can resolve — one of them is wrong
        // and we cannot tell which, so fail closed rather than let write order
        // decide.
        if let Some((_, _, prior)) = encoded.iter().find(|(_, l, _)| *l == label) {
            if prior != &payload {
                return Err(WalletStorageError::ProviderKeyAccountConflict {
                    account_type: label,
                });
            }
        }
        encoded.push((entry, label, payload));
    }

    // Same-label payloads are byte-identical here, so either surviving index
    // is equivalent.
    let distinct_accounts: BTreeMap<&'static str, usize> = encoded
        .iter()
        .enumerate()
        .map(|(index, (_, label, _))| (*label, index))
        .collect();
    for (&label, &index) in &distinct_accounts {
        let (entry, _, payload) = &encoded[index];
        let conflicts = load_provider_account_payload(tx, wallet_id, &entry.account_type)?
            .is_some_and(|stored| stored.as_slice() != payload.as_slice());
        if conflicts {
            return Err(WalletStorageError::ProviderKeyAccountConflict {
                account_type: label,
            });
        }
    }

    // The conflict checks and `DO NOTHING` jointly prevent account key
    // material from being overwritten.
    let mut account_stmt = tx.prepare_cached(UPSERT_PROVIDER_ACCOUNT_SQL)?;
    for (entry, _, payload) in encoded {
        upsert_account_row(&mut account_stmt, wallet_id, &entry.account_type, payload)?;
    }
    Ok(())
}

/// Return the encoded parent account payload for one provider account.
fn load_provider_account_payload(
    conn: &Connection,
    wallet_id: &WalletId,
    account_type: &AccountType,
) -> Result<Option<Vec<u8>>, WalletStorageError> {
    let (user_identity_id, friend_identity_id) = account_dashpay_ids(account_type);
    let mut stmt = conn.prepare_cached(
        "SELECT length(account_xpub_bytes), account_xpub_bytes \
         FROM account_registrations \
         WHERE wallet_id = ?1 AND account_type = ?2 AND account_index = ?3 \
           AND key_class = ?4 AND user_identity_id = ?5 AND friend_identity_id = ?6",
    )?;
    let mut rows = stmt.query(params![
        wallet_id.as_slice(),
        account_type_db_label(account_type),
        i64::from(account_index(account_type)),
        i64::from(account_key_class(account_type)),
        &user_identity_id[..],
        &friend_identity_id[..],
    ])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    blob::check_size(row.get::<_, i64>(0)?)?;
    Ok(Some(row.get::<_, Vec<u8>>(1)?))
}

/// True when the extended public key's curve is the one its account type
/// mandates. Enforced on both the write and the read path.
///
/// `account_type` is the discriminator that decides the curve — there is no tag
/// byte inside the payload. The FFI backend's restore side picks the same
/// discriminator (`platform-wallet-ffi::persistence`, which branches on
/// `account_type` to choose its decode), so the two backends agree on *how* a
/// provider account is identified. A [`ProviderKeyAccountEntry`] whose payload
/// carries the other curve is cross-curve confusion, not a decodable account.
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

/// Read the provider key-material accounts of one wallet.
///
/// Entries are ordered by `account_type`. Decode failures stay fatal;
/// Recovery skips typed-column drift and wrong-curve rows under distinct
/// degradation sites.
pub(crate) fn load_provider_state(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<Vec<ProviderKeyAccountEntry>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT account_type, account_index, key_class, \
                length(user_identity_id), user_identity_id, \
                length(friend_identity_id), friend_identity_id, \
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
        blob::check_fixed_width(
            row.get::<_, i64>(3)?,
            32,
            "account_registrations.user_identity_id",
        )?;
        let typed_user: Vec<u8> = row.get(4)?;
        blob::check_fixed_width(
            row.get::<_, i64>(5)?,
            32,
            "account_registrations.friend_identity_id",
        )?;
        let typed_friend: Vec<u8> = row.get(6)?;
        blob::check_size(row.get::<_, i64>(7)?)?;
        let payload: Vec<u8> = row.get(8)?;
        let entry = blob::decode::<ProviderKeyAccountEntry>(&payload)?;

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
        if !db_label_matches_entry(typed_type.as_str(), &entry.account_type)
            || account_index(&entry.account_type) != typed_index
            || account_key_class(&entry.account_type) != typed_key_class
            || blob_user.as_slice() != typed_user.as_slice()
            || blob_friend.as_slice() != typed_friend.as_slice()
        {
            ctx.tolerate(
                LoadSite::ProviderKeyRegistrationDrift,
                WalletStorageError::ProviderKeyAccountEntryMismatch,
            )?;
            continue;
        }
        if !provider_curve_matches_type(&entry.account_type, &entry.extended_public_key) {
            ctx.tolerate(
                LoadSite::ProviderKeyCurveMismatch,
                WalletStorageError::ProviderKeyAccountEntryMismatch,
            )?;
            continue;
        }

        out.push(ProviderKeyAccountEntry {
            account_type: entry.account_type,
            extended_public_key: entry.extended_public_key,
        });
    }
    Ok(out)
}

/// Read every `account_registrations` row for `wallet_id` into a keyless
/// [`AccountManifest`] — the rehydration account-set oracle (which accounts to
/// re-derive + the per-account xpubs the wrong-account gate checks). PUBLIC
/// material only (xpub + account type), no `Wallet` minted. Each list is
/// ordered by its typed columns for determinism. Typed-column drift is fatal
/// under Strict; Recovery drops the offending registration row. A pre-split
/// `standard` row is reconciled against the precise-labelled row for the same
/// account, so a forked registration is returned once. Persisted
/// funds attributed to that missing account fall back to the first remaining
/// funds account until the next sync rebuilds per-account attribution.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<AccountManifest, WalletStorageError> {
    Ok(AccountManifest {
        ecdsa: load_ecdsa_state(conn, wallet_id, ctx)?,
        provider: load_provider_state(conn, wallet_id, ctx)?,
    })
}

/// The secp256k1 half of [`load_state`]: every row whose blob is an
/// [`AccountRegistrationEntry`].
fn load_ecdsa_state(
    conn: &Connection,
    wallet_id: &WalletId,
    ctx: &LoadCtx,
) -> Result<Vec<AccountRegistrationEntry>, WalletStorageError> {
    // Select typed columns alongside the blob so we can cross-check them
    // against the decoded entry — a row whose blob disagrees with its indexed
    // columns is a sign of corruption or a schema bug and must be rejected
    // rather than silently mis-bucketed.
    // `length(account_xpub_bytes)` is read first (O(1) from the row header) so
    // an oversize blob is caught before the Vec is allocated.
    // The provider key-material rows are excluded by `account_type`: their
    // blob is a `ProviderKeyAccountEntry` over a non-secp256k1 curve and
    // would hard-error this decode.
    let mut stmt = conn.prepare(
        "SELECT account_type, account_index, key_class, \
                length(user_identity_id), user_identity_id, \
                length(friend_identity_id), friend_identity_id, \
                length(account_xpub_bytes), account_xpub_bytes FROM account_registrations \
         WHERE wallet_id = ?1 \
           AND account_type NOT IN ('provider_operator', 'provider_platform') \
         ORDER BY account_type, account_index, key_class, user_identity_id, friend_identity_id",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut out = Vec::new();
    // Positions in `out` of rows still carrying the pre-split `standard`
    // label. `Vec::new` does not allocate until its first push, so a database
    // written after the split pays one label comparison per row and nothing
    // else — no allocation, and the same `out` it always returned.
    let mut legacy_rows: Vec<usize> = Vec::new();
    while let Some(row) = rows.next()? {
        let typed_type: String = row.get(0)?; // account_type TEXT
        let typed_index: i64 = row.get(1)?; // account_index INTEGER
        let typed_key_class: i64 = row.get(2)?; // key_class INTEGER
        blob::check_fixed_width(
            row.get::<_, i64>(3)?,
            32,
            "account_registrations.user_identity_id",
        )?;
        let typed_user: Vec<u8> = row.get(4)?; // user_identity_id BLOB
        blob::check_fixed_width(
            row.get::<_, i64>(5)?,
            32,
            "account_registrations.friend_identity_id",
        )?;
        let typed_friend: Vec<u8> = row.get(6)?; // friend_identity_id BLOB
        blob::check_size(row.get::<_, i64>(7)?)?;
        let payload: Vec<u8> = row.get(8)?; // account_xpub_bytes BLOB
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
        if !db_label_matches_entry(typed_type.as_str(), &entry.account_type)
            || blob_index != typed_index
            || blob_key_class != typed_key_class
            || blob_user.as_slice() != typed_user.as_slice()
            || blob_friend.as_slice() != typed_friend.as_slice()
        {
            ctx.tolerate(
                LoadSite::AccountRegistrationDrift,
                WalletStorageError::AccountRegistrationEntryMismatch,
            )?;
            continue;
        }
        if typed_type == LEGACY_STANDARD_LABEL {
            legacy_rows.push(out.len());
        }
        out.push(entry);
    }
    if legacy_rows.is_empty() {
        return Ok(out);
    }
    reconcile_legacy_standard_rows(out, &legacy_rows, ctx)
}

/// Collapse each pre-split `standard` row into the precise-labelled row that
/// stands for the same account.
///
/// `V008` admits the legacy label rather than guessing which standard variant
/// such a row is, so one account can hold two rows: the writer's upsert keys
/// on `account_type`, so a post-split save INSERTS a precisely-labelled
/// sibling instead of updating the legacy row. Returning both would make this
/// reader emit one account twice and leave deduplication to a consumer that
/// cannot see why the pair exists.
///
/// A sibling is matched on the whole typed `AccountType`, NOT on
/// `(index, key_class, identity ids)`: BIP44 and BIP32 accounts at one index
/// are different accounts that share those columns, and the coarser key would
/// fuse them — dropping a real registration and calling a healthy wallet
/// drifted. When the matched pair disagrees the legacy row is not a duplicate
/// but genuine drift (only the precise row is ever updated, so a changed xpub
/// leaves the legacy one behind), and it goes to
/// [`LoadSite::AccountRegistrationDrift`] like every other disagreement here.
/// A legacy row with no sibling is the account's only row and is kept.
fn reconcile_legacy_standard_rows(
    entries: Vec<AccountRegistrationEntry>,
    legacy_rows: &[usize],
    ctx: &LoadCtx,
) -> Result<Vec<AccountRegistrationEntry>, WalletStorageError> {
    let mut superseded = vec![false; entries.len()];
    for &legacy in legacy_rows {
        let Some(precise) = entries
            .iter()
            .enumerate()
            .find(|(pos, candidate)| {
                !legacy_rows.contains(pos) && candidate.account_type == entries[legacy].account_type
            })
            .map(|(_, candidate)| candidate)
        else {
            continue;
        };
        if *precise != entries[legacy] {
            ctx.tolerate(
                LoadSite::AccountRegistrationDrift,
                WalletStorageError::AccountRegistrationEntryMismatch,
            )?;
        }
        superseded[legacy] = true;
    }
    Ok(entries
        .into_iter()
        .zip(superseded)
        .filter_map(|(entry, is_superseded)| (!is_superseded).then_some(entry))
        .collect())
}

/// Verify every account manifest blob is bound to its persisted wallet id.
/// Missing or unequal checksums return [`WalletStorageError::ManifestIntegrityMismatch`]
/// before any account is reconstructed.
pub fn verify_manifest_checksums(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<(), WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT length(account_xpub_bytes), account_xpub_bytes, checksum \
         FROM account_registrations WHERE wallet_id = ?1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    while let Some(row) = rows.next()? {
        blob::check_size(row.get::<_, i64>(0)?)?;
        let payload: Vec<u8> = row.get(1)?;
        let stored: Option<Vec<u8>> = row.get(2)?;
        let expected = account_registration_checksum(wallet_id, &payload);
        match stored {
            Some(c) if c.as_slice() == expected => {}
            _ => return Err(WalletStorageError::ManifestIntegrityMismatch),
        }
    }
    Ok(())
}

/// Fill missing manifest checksums inside the V018 migration transaction.
/// Returns the number of rows filled; subsequent calls are idempotent.
pub fn backfill_missing_checksums(conn: &Connection) -> Result<usize, WalletStorageError> {
    let pending: Vec<(i64, Vec<u8>, Vec<u8>)> = {
        let mut stmt = conn.prepare(
            "SELECT rowid, wallet_id, account_xpub_bytes \
             FROM account_registrations WHERE checksum IS NULL",
        )?;
        let mapped = stmt.query_map([], |row| {
            let rowid: i64 = row.get(0)?;
            let wid_bytes: Vec<u8> = row.get(1)?;
            let payload: Vec<u8> = row.get(2)?;
            Ok((rowid, wid_bytes, payload))
        })?;
        mapped.collect::<Result<Vec<_>, _>>()?
    };
    let mut filled = 0usize;
    {
        let mut upd =
            conn.prepare_cached("UPDATE account_registrations SET checksum = ?1 WHERE rowid = ?2")?;
        for (rowid, wid_bytes, payload) in pending {
            let wallet_id = <[u8; 32]>::try_from(wid_bytes.as_slice()).map_err(|_| {
                WalletStorageError::InvalidWalletIdLength {
                    column: "account_registrations.wallet_id",
                    actual: wid_bytes.len(),
                }
            })?;
            let checksum = account_registration_checksum(&wallet_id, &payload);
            upd.execute(params![&checksum[..], rowid])?;
            filled += 1;
        }
    }
    Ok(filled)
}

/// Source of truth for the `account_registrations.account_type` TEXT domain,
/// mirroring [`key_wallet::account::AccountType`]. The migrations interpolate
/// nothing: V001 freezes its own copy of this domain, because a generated-SQL
/// change breaks that migration's Refinery checksum on every database that
/// already applied it. `account_type_labels_match_enum` pins this array to
/// [`account_type_db_label`]; `account_type_labels_frozen_in_v007` pins it to
/// the frozen list in `V008__rehydration_base_schema.rs`, which rebuilt
/// `account_registrations` with the widened domain. V001 carries the narrower
/// domain `v4.2-dev` shipped, in which both standard variants share the label
/// `standard`. An upstream variant addition therefore fails a test with
/// instructions, instead of silently rewriting applied SQL.
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
/// The label `v4.2-dev` wrote for BOTH standard variants.
///
/// Its `account_type_db_label` matched `Standard { .. }` and ignored
/// `standard_account_type`, so a database created before the domain split
/// carries `standard` for BIP44 and BIP32 alike. Which one a row really is was
/// never lost -- it is inside `account_xpub_bytes` -- it is simply not
/// SQL-reachable, so no migration can resolve it. The value is therefore
/// admitted rather than rewritten; see [`db_label_matches_entry`].
pub(crate) const LEGACY_STANDARD_LABEL: &str = "standard";

/// Does the stored `account_type` column agree with the blob's typed
/// `AccountType`?
///
/// Exact match, plus one legacy equivalence: the pre-split `standard` matches
/// EITHER standard variant. Rewriting such a row to one variant would be a
/// guess, and guessing wrong turns a row that loads today into a fatal
/// `AccountRegistrationEntryMismatch` under the default `LoadPolicy::Strict`.
/// The blob stays the sole source of truth for which variant a row is; the
/// column keeps its narrower job of filtering and key uniqueness.
pub(crate) fn db_label_matches_entry(
    column: &str,
    entry_type: &key_wallet::account::AccountType,
) -> bool {
    if column == account_type_db_label(entry_type) {
        return true;
    }
    column == LEGACY_STANDARD_LABEL
        && matches!(
            entry_type,
            key_wallet::account::AccountType::Standard { .. }
        )
}

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
///
/// Wildcard-free on purpose, like [`account_index`] and
/// [`account_type_db_label`]: this feeds a PRIMARY KEY column, so a variant
/// this mapper has not been taught about would be given another variant's
/// sentinel and collapse two distinct accounts onto one key — losing one of
/// them at the next write, with no error anywhere. Listing the zeros costs a
/// dozen lines and converts that silent loss into a compile error.
pub(crate) fn account_key_class(at: &key_wallet::account::AccountType) -> u32 {
    use key_wallet::account::AccountType;
    match at {
        AccountType::PlatformPayment { key_class, .. } => *key_class,
        // No key-class axis: the column's sentinel default.
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
        | AccountType::DashpayExternalAccount { .. } => 0,
    }
}

/// DashPay `(user_identity_id, friend_identity_id)` discriminator pair — the
/// real account key for `DashpayReceivingFunds` / `DashpayExternalAccount`,
/// persisted in the matching PK columns. All-zero for every non-DashPay
/// variant (no identity axis), matching the column default.
///
/// Wildcard-free for the same reason as [`account_key_class`]: these are PK
/// columns, and an untaught variant handed the all-zero sentinel shares a key
/// with every other axis-less account at the same index.
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
        // No identity axis: the columns' sentinel default.
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
        | AccountType::PlatformPayment { .. } => ([0u8; 32], [0u8; 32]),
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

        let err = load_state(&conn, &w, &LoadCtx::strict())
            .expect_err("load_state must fail on type mismatch");
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

        let err = load_state(&conn, &w, &LoadCtx::strict())
            .expect_err("load_state must fail on index mismatch");
        assert!(
            matches!(err, WalletStorageError::AccountRegistrationEntryMismatch),
            "expected AccountRegistrationEntryMismatch, got {err:?}"
        );
    }

    /// The `platform_payment` readers (`all_platform_payment_registrations`,
    /// the production `load()` oracle via `platform_addrs::load_all`, and its
    /// per-wallet sibling `list_platform_payment_registrations`) must reject a
    /// row whose typed `key_class` column disagrees with the blob's
    /// `PlatformPayment.key_class` — the exact discriminator the widened PK
    /// exists to protect — mirroring `load_state`'s full cross-check.
    #[test]
    fn platform_payment_readers_reject_key_class_column_mismatch() {
        let conn = migrated_conn();
        let w = [0x77u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        // Blob encodes key_class = 0 ...
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 0,
                key_class: 0,
            },
            account_xpub: test_xpub(),
        };
        let blob = blob::encode(&entry).unwrap();
        // ... but the typed key_class column says 1 — a mismatch on the very
        // column that keeps distinct key classes from colliding.
        conn.execute(
            "INSERT INTO account_registrations \
                (wallet_id, account_type, account_index, key_class, account_xpub_bytes, checksum) \
             VALUES (?1, 'platform_payment', 0, 1, ?2, ?3)",
            rusqlite::params![
                &w[..],
                &blob,
                account_registration_checksum(&w, &blob).as_slice()
            ],
        )
        .unwrap();

        // The bulk reader refuses the row per WALLET: the scan survives, and
        // the wallet that owns the bad row carries the refusal.
        let all = all_platform_payment_registrations(&conn)
            .expect("the scan itself must survive one bad row");
        let err = all
            .get(&w)
            .expect("the wallet must be present in the scan")
            .as_ref()
            .expect_err("bulk reader must reject key_class mismatch");
        assert!(
            matches!(err, WalletStorageError::AccountRegistrationEntryMismatch),
            "bulk reader: expected AccountRegistrationEntryMismatch, got {err:?}"
        );
        let err = list_platform_payment_registrations(&conn, &w)
            .expect_err("per-wallet reader must reject key_class mismatch");
        assert!(
            matches!(err, WalletStorageError::AccountRegistrationEntryMismatch),
            "per-wallet reader: expected AccountRegistrationEntryMismatch, got {err:?}"
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

        let loaded = load_state(&conn, &w, &LoadCtx::strict())
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
        let loaded = load_state(&conn, &w, &LoadCtx::strict())
            .expect("both key classes load")
            .ecdsa;
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
        let loaded = load_state(&conn, &w, &LoadCtx::strict())
            .expect("both contacts load")
            .ecdsa;
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
        let loaded = load_state(&conn, &w, &LoadCtx::strict())
            .expect("load")
            .ecdsa;
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

    /// Read `(account_xpub_bytes, checksum)` for the single row of `wallet_id`.
    fn read_blob_and_checksum(
        conn: &rusqlite::Connection,
        wallet_id: &WalletId,
    ) -> (Vec<u8>, Option<Vec<u8>>) {
        conn.query_row(
            "SELECT account_xpub_bytes, checksum FROM account_registrations WHERE wallet_id = ?1",
            rusqlite::params![wallet_id.as_slice()],
            |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Option<Vec<u8>>>(1)?)),
        )
        .unwrap()
    }

    /// TC-C-001 — the writer stores a non-NULL checksum equal to
    /// `SHA-256(wallet_id ‖ account_xpub_bytes)` on the exact stored blob.
    #[test]
    fn write_stores_checksum_over_wallet_id_and_blob() {
        let mut conn = migrated_conn();
        let w = [0x77u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 4,
                key_class: 0,
            },
            account_xpub: test_xpub(),
        };
        {
            let tx = conn.transaction().unwrap();
            apply_registrations(&tx, &w, std::slice::from_ref(&entry)).unwrap();
            tx.commit().unwrap();
        }
        let (blob, checksum) = read_blob_and_checksum(&conn, &w);
        let checksum = checksum.expect("checksum must be non-NULL after a write");
        assert_eq!(
            checksum.as_slice(),
            account_registration_checksum(&w, &blob),
            "stored checksum must equal SHA-256(wallet_id ‖ account_xpub_bytes)"
        );
        // The verify path agrees on the freshly written row.
        verify_manifest_checksums(&conn, &w).expect("freshly written checksum verifies");
    }

    /// TC-C-009 — re-persisting the same account keeps the checksum correct and
    /// consistent with the final `account_xpub_bytes`.
    #[test]
    fn repersist_keeps_checksum_consistent() {
        let mut conn = migrated_conn();
        let w = [0x88u8; 32];
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
        let (blob, checksum) = read_blob_and_checksum(&conn, &w);
        assert_eq!(
            checksum.expect("checksum present").as_slice(),
            account_registration_checksum(&w, &blob),
        );
        verify_manifest_checksums(&conn, &w).expect("re-persisted checksum verifies");
    }

    /// TC-C-007 — the backfill fills every NULL checksum with the exact
    /// `SHA-256(wallet_id ‖ account_xpub_bytes)`, and is idempotent (a second
    /// pass fills nothing and leaves every row verifying).
    #[test]
    fn backfill_fills_null_checksums_exactly_and_is_idempotent() {
        let mut conn = migrated_conn();
        let w = [0x99u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        let entry = AccountRegistrationEntry {
            account_type: key_wallet::account::AccountType::PlatformPayment {
                account: 1,
                key_class: 0,
            },
            account_xpub: test_xpub(),
        };
        {
            let tx = conn.transaction().unwrap();
            apply_registrations(&tx, &w, std::slice::from_ref(&entry)).unwrap();
            tx.commit().unwrap();
        }
        // Simulate a pre-V018 row: strip the checksum the writer just set.
        conn.execute(
            "UPDATE account_registrations SET checksum = NULL WHERE wallet_id = ?1",
            rusqlite::params![&w[..]],
        )
        .unwrap();
        assert!(read_blob_and_checksum(&conn, &w).1.is_none());

        let filled = backfill_missing_checksums(&conn).unwrap();
        assert_eq!(filled, 1, "one NULL row must be filled");
        let (blob, checksum) = read_blob_and_checksum(&conn, &w);
        assert_eq!(
            checksum.expect("checksum filled").as_slice(),
            account_registration_checksum(&w, &blob),
        );
        verify_manifest_checksums(&conn, &w).expect("backfilled checksum verifies");

        // Idempotent: nothing left to fill.
        assert_eq!(backfill_missing_checksums(&conn).unwrap(), 0);
    }

    /// No two account types may share a full PK tuple, whatever axes they
    /// carry. This is the runtime half of the wildcard-free mappers: those
    /// make an untaught variant a compile error, and this makes a variant
    /// that IS taught but mapped onto an existing key a test failure.
    ///
    /// Reached through `all_account_type_variants`, whose own exhaustive
    /// match means a new upstream variant cannot arrive without a decision
    /// being taken here.
    #[test]
    fn no_two_account_types_share_a_pk_tuple() {
        let keys: Vec<_> = all_account_type_variants()
            .into_iter()
            .map(|at| {
                (
                    account_type_db_label(&at),
                    account_index(&at),
                    account_key_class(&at),
                    account_dashpay_ids(&at),
                )
            })
            .collect();
        let mut seen: HashSet<_> = HashSet::new();
        let collisions: Vec<_> = keys.iter().filter(|key| !seen.insert(*key)).collect();
        assert!(
            collisions.is_empty(),
            "these primary keys are claimed by more than one account type, so \
             the second account written would overwrite the first: {collisions:?}"
        );
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

    /// Pins the live domain to the list frozen in the latest migration that
    /// rebuilt `account_registrations` (`V008__rehydration_base_schema.rs`).
    /// That frozen list is this array plus [`LEGACY_STANDARD_LABEL`], which no
    /// writer emits but pre-split rows still carry.
    ///
    /// IF THIS FAILS: do NOT edit V008's list to match. Refinery checksums a
    /// migration's rendered SQL, so changing an applied migration's body makes
    /// every database that already ran it fail to open, permanently. Append a
    /// migration rebuilding the table with the widened CHECK (the
    /// `V018__asset_lock_recovered_status.rs` pattern), then update this pin.
    #[test]
    fn account_type_labels_frozen_in_v007() {
        assert_eq!(
            ACCOUNT_TYPE_LABELS,
            &[
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
            ]
        );
    }

    /// The pre-split `standard` label matches EITHER standard variant, so a
    /// database written before the domain split still cross-checks clean. A
    /// migration cannot resolve which variant such a row is -- the answer is in
    /// the blob, not in SQL -- so rewriting the label would be a guess, and a
    /// wrong guess makes a row that loads today fail under `LoadPolicy::Strict`.
    #[test]
    fn legacy_standard_label_matches_either_standard_variant() {
        use key_wallet::account::{AccountType, StandardAccountType};
        for standard_account_type in [
            StandardAccountType::BIP44Account,
            StandardAccountType::BIP32Account,
        ] {
            let entry_type = AccountType::Standard {
                index: 0,
                standard_account_type,
            };
            assert!(
                db_label_matches_entry(LEGACY_STANDARD_LABEL, &entry_type),
                "legacy `standard` must match {standard_account_type:?}"
            );
            assert!(
                db_label_matches_entry(account_type_db_label(&entry_type), &entry_type),
                "the split label must still match its own variant"
            );
        }
    }

    /// The legacy equivalence is narrow: it admits `standard` for a Standard
    /// account and nothing else. It must not let one split label stand in for
    /// the other, nor `standard` stand in for a non-standard account.
    #[test]
    fn legacy_standard_label_equivalence_is_narrow() {
        use key_wallet::account::{AccountType, StandardAccountType};
        let bip44 = AccountType::Standard {
            index: 0,
            standard_account_type: StandardAccountType::BIP44Account,
        };
        assert!(
            !db_label_matches_entry("standard_bip32", &bip44),
            "one split label must never stand in for the other"
        );
        assert!(
            !db_label_matches_entry(LEGACY_STANDARD_LABEL, &AccountType::IdentityRegistration),
            "legacy `standard` must not match a non-standard account"
        );
    }
}
