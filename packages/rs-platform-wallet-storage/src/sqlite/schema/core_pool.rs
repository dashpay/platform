//! Writer + account-attribution helper for the `core_address_pool` table.
//!
//! Per-index address-pool rows carrying a `used` flag, scoped by
//! `(wallet_id, account_type, account_index, key_class, user_identity_id,
//! friend_identity_id, pool_type, address_index)` — the DashPay identity pair
//! is in the PK (mirroring `account_registrations`) so distinct contacts on
//! one wallet, which otherwise collapse to the same `(dashpay_receiving, 0)`
//! sentinel, never overwrite each other's pool rows. The first-class row
//! store the reader consumes verbatim —
//! no `core_utxos` script-derivation, no horizon-walk re-derivation. Populated
//! from the `account_address_pools` changeset snapshots; the UTXO writer reads
//! it back to attribute an outpoint to its owning account.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::AccountAddressPoolEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use key_wallet::managed_account::address_pool::{
    AddressPool, AddressPoolType, AddressState, KeySource, PublicKeyType,
};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite, SiteCoords};
use crate::sqlite::rehydrate::MAX_REHYDRATION_DERIVATION_INDEX;
use crate::sqlite::schema::accounts;
use crate::sqlite::schema::blob;
use crate::sqlite::util::safe_cast::{i64_to_u32, i64_to_u64};

/// Stored `pool_type` discriminant. Kept in the primary key so an External
/// and an Internal pool never collide at the same `address_index`.
pub(crate) fn pool_type_to_i64(pool_type: AddressPoolType) -> i64 {
    match pool_type {
        AddressPoolType::External => 0,
        AddressPoolType::Internal => 1,
        AddressPoolType::Absent => 2,
        AddressPoolType::AbsentHardened => 3,
    }
}

// Stable `PublicKeyType` declaration-order discriminants and emitted lengths.
const KEY_TYPE_ECDSA: i64 = 0;
const KEY_TYPE_EDDSA: i64 = 1;
const KEY_TYPE_BLS: i64 = 2;
const ECDSA_PUBLIC_KEY_LEN: usize = 33;
const EDDSA_PUBLIC_KEY_LEN: usize = 32;
const BLS_PUBLIC_KEY_LEN: usize = 48;

fn key_type_to_i64(key_type: &PublicKeyType) -> i64 {
    match key_type {
        PublicKeyType::ECDSA(_) => KEY_TYPE_ECDSA,
        PublicKeyType::EdDSA(_) => KEY_TYPE_EDDSA,
        PublicKeyType::BLS(_) => KEY_TYPE_BLS,
    }
}

// Monotonic final usage clears reservations so a stale Reserved snapshot
// cannot resurrect one after the row has become Used.
const UPSERT_POOL_SQL: &str = "INSERT INTO core_address_pool \
        (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id, \
         pool_type, address_index, script, used, public_key, key_type, reserved_at) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
     ON CONFLICT(wallet_id, account_type, account_index, key_class, user_identity_id, \
                 friend_identity_id, pool_type, address_index) \
     DO UPDATE SET \
        script = excluded.script, \
        public_key = excluded.public_key, \
        key_type = excluded.key_type, \
        used = MAX(used, excluded.used), \
        reserved_at = CASE \
            WHEN MAX(used, excluded.used) = 1 THEN NULL \
            ELSE excluded.reserved_at \
        END";

const TYPED_POOL_CONFLICT_SQL: &str = "SELECT EXISTS( \
        SELECT 1 FROM core_address_pool \
        WHERE wallet_id = ?1 AND account_type = ?2 AND account_index = ?3 \
          AND key_class = ?4 AND user_identity_id = ?5 AND friend_identity_id = ?6 \
          AND pool_type = ?7 AND address_index = ?8 \
          AND key_type IS NOT NULL AND public_key IS NOT NULL \
          AND (?10 IS NULL OR public_key <> ?9 OR key_type <> ?10) \
    )";

/// Expand `account_address_pools` snapshots into per-index
/// `core_address_pool` rows. Idempotent: `script` is derivation-stable and
/// `used` is monotonic (`MAX`), so re-applying the same snapshot is a no-op
/// and a used address can never revert to unused (the reuse-guard invariant).
/// Writes reject any change or erasure of typed key material already stored.
///
/// # Errors
///
/// [`WalletStorageError::TypedPoolKeyConflict`] when incoming key material
/// contradicts what is already stored, and
/// [`WalletStorageError::EmptyPoolAddressScript`] when a row carries a
/// zero-length `script`: `load()` turns every stored script back into an
/// address, so an empty one degrades the load of the owning wallet and fails
/// it outright under a strict policy. This is the only writer of
/// `core_address_pool.script`, so refusing here closes the producer.
///
/// The script guard binds new writes only. Empty scripts already present in
/// an existing database file remain fatal to a strict load, and are
/// deliberately left in place rather than purged — the same posture V015
/// took for the sibling `core_utxos.script` column, where it purged only
/// legacy empty-script *spent* rows and left the surviving balance-bearing
/// ones alone.
pub fn apply_pools(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    pools: &[AccountAddressPoolEntry],
) -> Result<(), WalletStorageError> {
    if pools.is_empty() {
        return Ok(());
    }
    let mut conflict_stmt = tx.prepare_cached(TYPED_POOL_CONFLICT_SQL)?;
    let mut upsert_stmt = tx.prepare_cached(UPSERT_POOL_SQL)?;
    for entry in pools {
        // `account_type` discriminates accounts that collapse to the same
        // `(account_index, key_class)` sentinel (e.g. `IdentityRegistration`
        // and `ProviderVotingKeys`, both `0, 0`); without it they would upsert
        // onto the same PK and overwrite each other's rows.
        let account_type = accounts::account_type_db_label(&entry.account_type);
        let account_index = i64::from(accounts::account_index(&entry.account_type));
        // TODO(key_class): PlatformPayment carries a real key_class; every
        // other account maps to the 0 sentinel until the pool snapshot
        // threads a per-pool key class.
        let key_class = i64::from(accounts::account_key_class(&entry.account_type));
        // DashPay accounts all collapse to (dashpay_receiving/dashpay_external,
        // account_index=0); the identity pair is the real per-contact
        // discriminator, all-zero for every other account type.
        let (user_identity_id, friend_identity_id) =
            accounts::account_dashpay_ids(&entry.account_type);
        let pool_type = pool_type_to_i64(entry.pool_type);
        for info in &entry.addresses {
            if info.script_pubkey.as_bytes().is_empty() {
                return Err(WalletStorageError::EmptyPoolAddressScript {
                    account_type,
                    address_index: info.index,
                });
            }
            let key_type = info.public_key.as_ref().map(key_type_to_i64);
            let (public_key, expected_key_len): (Option<&[u8]>, Option<usize>) =
                match info.public_key.as_ref() {
                    None => (None, None),
                    Some(PublicKeyType::ECDSA(bytes)) => {
                        (Some(bytes.as_slice()), Some(ECDSA_PUBLIC_KEY_LEN))
                    }
                    Some(PublicKeyType::EdDSA(bytes)) => {
                        (Some(bytes.as_slice()), Some(EDDSA_PUBLIC_KEY_LEN))
                    }
                    Some(PublicKeyType::BLS(bytes)) => {
                        (Some(bytes.as_slice()), Some(BLS_PUBLIC_KEY_LEN))
                    }
                };
            if let (Some(public_key), Some(expected_key_len)) = (public_key, expected_key_len) {
                blob::check_fixed_width(
                    i64::try_from(public_key.len()).unwrap_or(i64::MAX),
                    expected_key_len,
                    "core_address_pool.public_key has the wrong length for key_type",
                )?;
            }
            let conflicts = conflict_stmt.query_row(
                params![
                    wallet_id.as_slice(),
                    account_type,
                    account_index,
                    key_class,
                    user_identity_id.as_slice(),
                    friend_identity_id.as_slice(),
                    pool_type,
                    i64::from(info.index),
                    public_key,
                    key_type,
                ],
                |row| row.get::<_, bool>(0),
            )?;
            if conflicts {
                return Err(WalletStorageError::TypedPoolKeyConflict {
                    account_type,
                    address_index: info.index,
                });
            }
            let reserved_at = info
                .reserved_at()
                .map(|at| {
                    crate::sqlite::util::safe_cast::u64_to_i64("core_address_pool.reserved_at", at)
                })
                .transpose()?;
            upsert_stmt.execute(params![
                wallet_id.as_slice(),
                account_type,
                account_index,
                key_class,
                user_identity_id.as_slice(),
                friend_identity_id.as_slice(),
                pool_type,
                i64::from(info.index),
                info.script_pubkey.as_bytes(),
                info.is_used(),
                public_key,
                key_type,
                reserved_at,
            ])?;
        }
    }
    Ok(())
}

/// One restored typed-pool row: `(address_index, script_bytes, public_key, used)`.
///
/// `reserved_at` is not part of the row: [`restore_pools`] restores it for
/// every pool, this one included.
pub type TypedPoolEntry = (u32, Vec<u8>, PublicKeyType, bool);

/// Load typed public-key rows for one account pool, ordered by address index.
///
/// These rows carry pre-derived hardened-only batches, such as platform-node
/// EdDSA keys, that cannot be regenerated from a watch-only account xpub.
/// Invalid key discriminants, missing keys, and wrong key widths are errors.
pub fn load_typed_pool_entries(
    conn: &Connection,
    wallet_id: &WalletId,
    account_type: &key_wallet::account::AccountType,
    pool_type: AddressPoolType,
) -> Result<Vec<TypedPoolEntry>, WalletStorageError> {
    let account_type = accounts::account_type_db_label(account_type);
    let pool_type = pool_type_to_i64(pool_type);
    let (max_script_len, max_public_key_len): (Option<i64>, Option<i64>) = conn.query_row(
        "SELECT MAX(length(script)), MAX(length(public_key)) FROM core_address_pool \
         WHERE wallet_id = ?1 AND account_type = ?2 AND pool_type = ?3 \
           AND (public_key IS NOT NULL OR key_type IS NOT NULL)",
        params![wallet_id.as_slice(), account_type, pool_type],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    if let Some(len) = max_script_len {
        blob::check_size(len)?;
    }
    if let Some(len) = max_public_key_len {
        blob::check_size(len)?;
    }

    let mut stmt = conn.prepare(
        "SELECT address_index, length(script), script, length(public_key), public_key, \
                key_type, used FROM core_address_pool \
         WHERE wallet_id = ?1 AND account_type = ?2 AND pool_type = ?3 \
           AND (public_key IS NOT NULL OR key_type IS NOT NULL) \
         ORDER BY address_index",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), account_type, pool_type,])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let address_index = crate::sqlite::util::safe_cast::i64_to_u32(
            "core_address_pool.address_index",
            row.get::<_, i64>(0)?,
        )?;
        blob::check_size(row.get::<_, i64>(1)?)?;
        let script = row.get::<_, Vec<u8>>(2)?;
        let public_key_len = row.get::<_, Option<i64>>(3)?;
        let key_type = row.get::<_, Option<i64>>(5)?;
        let (public_key_len, key_type) = match (public_key_len, key_type) {
            (Some(public_key_len), Some(key_type)) => (public_key_len, key_type),
            _ => {
                return Err(WalletStorageError::blob_decode(
                    "core_address_pool.public_key and key_type nullability differ",
                ));
            }
        };
        let expected_key_len = match key_type {
            KEY_TYPE_ECDSA => ECDSA_PUBLIC_KEY_LEN,
            KEY_TYPE_EDDSA => EDDSA_PUBLIC_KEY_LEN,
            KEY_TYPE_BLS => BLS_PUBLIC_KEY_LEN,
            _ => {
                return Err(WalletStorageError::blob_decode(
                    "core_address_pool.key_type is outside 0..=2",
                ));
            }
        };
        blob::check_fixed_width(
            public_key_len,
            expected_key_len,
            "core_address_pool.public_key has the wrong length for key_type",
        )?;
        let Some(public_key) = row.get::<_, Option<Vec<u8>>>(4)? else {
            return Err(WalletStorageError::blob_decode(
                "core_address_pool.public_key is NULL for a typed row",
            ));
        };
        let public_key = match key_type {
            KEY_TYPE_ECDSA => PublicKeyType::ECDSA(public_key),
            KEY_TYPE_EDDSA => PublicKeyType::EdDSA(public_key),
            KEY_TYPE_BLS => PublicKeyType::BLS(public_key),
            _ => {
                return Err(WalletStorageError::blob_decode(
                    "core_address_pool.key_type is outside 0..=2",
                ));
            }
        };
        out.push((address_index, script, public_key, row.get(6)?));
    }
    Ok(out)
}

/// Identity of the funds account that owns an address, matched against a
/// `core_address_pool` row. Enough to select one account among funding accounts
/// that share a numeric `account_index` (Standard BIP44/BIP32 and CoinJoin can
/// all sit at index 0; DashPay accounts all persist index 0 and are told apart
/// by the identity pair).
///
/// Carries four of the writer's five pool-PK account discriminators
/// (`UPSERT_POOL_SQL`); `key_class` is intentionally omitted. Every funds
/// account maps to the `key_class = 0` sentinel — only the non-funds
/// `PlatformPayment` account carries a real class, and it is never a funding
/// account — so `key_class` cannot distinguish two funds accounts and adds
/// nothing here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwningAccount {
    /// `account_type` DB label (see [`accounts::account_type_db_label`]).
    pub account_type: String,
    /// Numeric account index within its type.
    pub account_index: u32,
    /// DashPay owner identity (all-zero for non-DashPay accounts).
    pub user_identity_id: [u8; 32],
    /// DashPay contact identity (all-zero for non-DashPay accounts).
    pub friend_identity_id: [u8; 32],
}

fn validate_account_type(label: &str) -> Result<(), WalletStorageError> {
    if accounts::ACCOUNT_TYPE_LABELS.contains(&label) || label == accounts::LEGACY_STANDARD_LABEL {
        Ok(())
    } else {
        Err(WalletStorageError::blob_decode(
            "core_address_pool.account_type is unknown",
        ))
    }
}

/// Full owning-account identity for a UTXO, matched by its `script_pubkey`
/// against a pool row. `None` when no pool row covers the script.
pub(crate) fn owning_account_for_script(
    conn: &Connection,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<Option<OwningAccount>, WalletStorageError> {
    // A script can appear under several pool rows (distinct account_type /
    // key_class / identity pair / pool_type share the same `script_pubkey`
    // for reused keys); an explicit PK-ordered tie-break makes the pick
    // deterministic instead of relying on SQLite's arbitrary `LIMIT 1` row.
    let mut stmt = conn.prepare_cached(
        "SELECT account_type, account_index, length(user_identity_id), user_identity_id, \
                length(friend_identity_id), friend_identity_id \
         FROM core_address_pool \
         WHERE wallet_id = ?1 AND script = ?2 \
         ORDER BY account_type, account_index, key_class, user_identity_id, \
                  friend_identity_id, pool_type, address_index ASC \
         LIMIT 1",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), script])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let account_type = row.get::<_, String>(0)?;
    validate_account_type(&account_type)?;
    let index = row.get::<_, i64>(1)?;
    blob::check_fixed_width(
        row.get::<_, i64>(2)?,
        32,
        "core_address_pool.user_identity_id",
    )?;
    let user = row.get::<_, Vec<u8>>(3)?;
    blob::check_fixed_width(
        row.get::<_, i64>(4)?,
        32,
        "core_address_pool.friend_identity_id",
    )?;
    let friend = row.get::<_, Vec<u8>>(5)?;
    let account_index =
        crate::sqlite::util::safe_cast::i64_to_u32("core_address_pool.account_index", index)?;
    let user_identity_id = super::id32("core_address_pool.user_identity_id", &user)?;
    let friend_identity_id = super::id32("core_address_pool.friend_identity_id", &friend)?;
    Ok(Some(OwningAccount {
        account_type,
        account_index,
        user_identity_id,
        friend_identity_id,
    }))
}

/// Used addresses for a wallet, read verbatim from `core_address_pool`
/// (`used = 1`), each paired with its owning account. This is the
/// *known-owner* reuse-guard source: the pool row carries the account
/// discriminators directly, so no per-script round-trip is needed. Possibly
/// empty. The caller **unions** this with the `core_utxos`-derived set — the
/// reuse guard is monotonic, so a mixed store (historical UTXOs a later
/// partial pool snapshot never enumerates) must surface both sources, never
/// drop the historical ones.
///
/// A `script` reused across accounts yields several rows; this reader applies
/// the same PK-ordered tie-break as [`owning_account_for_script`] among rows
/// marked used. An unused row can therefore own a UTXO while a different used
/// row supplies the reuse guard, which preserves each resolver's distinct
/// source contract. `network` turns each stored `script` back into an
/// [`Address`](dashcore::Address); an invalid script is fatal under Strict and
/// skipped under Recovery, matching
/// [`crate::sqlite::schema::core_state::load_used_addresses`].
/// This compatibility entry point uses Strict; rehydration calls
/// [`load_used_addresses_with_ctx`] with its policy context.
/// Unknown account labels fail the wallet under either policy: dropping a
/// used address would lose its reuse guard. Recovery isolates that wallet.
pub fn load_used_addresses(
    conn: &rusqlite::Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<Vec<(dashcore::Address, OwningAccount)>, WalletStorageError> {
    load_used_addresses_with_ctx(conn, wallet_id, network, &LoadCtx::strict())
}

/// [`load_used_addresses`] under an explicit load policy.
pub fn load_used_addresses_with_ctx(
    conn: &rusqlite::Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
    ctx: &LoadCtx,
) -> Result<Vec<(dashcore::Address, OwningAccount)>, WalletStorageError> {
    // Gate the largest stored `script` with a cheap aggregate BEFORE the
    // ordered read materializes or sorts any blob, so a corrupt/oversize
    // column raises a typed `BlobTooLarge` (the crate's 16 MiB cap) rather
    // than SQLite's own `TooBig` mid-sort, and never OOMs the host.
    blob::check_max_column_len(
        conn,
        "SELECT MAX(length(script)) FROM core_address_pool \
         WHERE wallet_id = ?1 AND used = 1",
        wallet_id,
    )?;
    // Order by script then the pool PK so the first row per script is the
    // tie-break winner; a `HashSet` on script drops the trailing duplicates.
    let mut stmt = conn.prepare(
        "SELECT script, account_type, account_index, \
                length(user_identity_id), user_identity_id, \
                length(friend_identity_id), friend_identity_id \
         FROM core_address_pool \
         WHERE wallet_id = ?1 AND used = 1 \
         ORDER BY script, account_type, account_index, key_class, user_identity_id, \
                  friend_identity_id, pool_type, address_index",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let raw_script = row.get::<_, Vec<u8>>(0)?;
        let account_type = row.get::<_, String>(1)?;
        validate_account_type(&account_type)?;
        let index = row.get::<_, i64>(2)?;
        blob::check_fixed_width(
            row.get::<_, i64>(3)?,
            32,
            "core_address_pool.user_identity_id",
        )?;
        let user = row.get::<_, Vec<u8>>(4)?;
        blob::check_fixed_width(
            row.get::<_, i64>(5)?,
            32,
            "core_address_pool.friend_identity_id",
        )?;
        let friend = row.get::<_, Vec<u8>>(6)?;
        if !seen.insert(raw_script.clone()) {
            continue;
        }
        let address = match blob::decode_script_to_address(raw_script, network) {
            Ok(address) => address,
            Err(error) => {
                ctx.tolerate(LoadSite::UndecodableAddressScript, error)?;
                continue;
            }
        };
        let account_index =
            crate::sqlite::util::safe_cast::i64_to_u32("core_address_pool.account_index", index)?;
        let user_identity_id = super::id32("core_address_pool.user_identity_id", &user)?;
        let friend_identity_id = super::id32("core_address_pool.friend_identity_id", &friend)?;
        out.push((
            address,
            OwningAccount {
                account_type,
                account_index,
                user_identity_id,
                friend_identity_id,
            },
        ));
    }
    Ok(out)
}

/// Public key material that re-derives `account_type`'s pool entries, if the
/// watch-only `wallet` holds any.
fn public_key_source(
    wallet: &key_wallet::wallet::Wallet,
    account_type: key_wallet::account::AccountType,
) -> Option<KeySource> {
    if let Some(account) = wallet.accounts.account_of_type(account_type) {
        return Some(KeySource::Public(account.account_xpub));
    }
    wallet
        .accounts
        .bls_account_of_type(account_type)
        .map(|account| KeySource::BLSPublic(account.bls_public_key.clone()))
}

/// How many indices a stored pool may be missing below its highest row and
/// still be derived all the way through.
///
/// Checking a row means deriving every index below it, and that work has to
/// be bounded: one damaged `address_index` must not buy billions of
/// derivations. Depth is not what marks a damaged index, though. The writer
/// stores a pool row for row however far the wallet has really grown it, so
/// a deep pool brings one row for every index it asks to have derived, and a
/// wallet that issued twenty thousand funding keys is as healthy as one that
/// issued twenty. What marks damage is sparseness: far more indices below a
/// row than there are rows. The derivation budget is therefore the pool's
/// own row count plus this allowance for indices no row covers — the same
/// allowance the discovery walk gives an account, so a partial snapshot it
/// could have resolved is restored here too.
const UNSTORED_INDEX_ALLOWANCE: u32 = MAX_REHYDRATION_DERIVATION_INDEX;

/// Derive `pool` from its account's public key towards `highest_stored`, so
/// the stored rows meet entries this wallet derived itself.
///
/// At most `stored_rows` + [`UNSTORED_INDEX_ALLOWANCE`] indices are derived.
/// A row left above that has no derived entry, and the overlay skips it.
///
/// Hardened pools are left as they are; see [`restore_pools`].
fn derive_towards(
    pool: &mut AddressPool,
    key_source: Option<&KeySource>,
    highest_stored: u32,
    stored_rows: u32,
) -> Result<(), WalletStorageError> {
    if pool.pool_type == AddressPoolType::AbsentHardened {
        return Ok(());
    }
    let derived = pool
        .highest_generated
        .map_or(0, |highest| highest.saturating_add(1));
    if highest_stored < derived {
        return Ok(());
    }
    // Without key material nothing is derived, and the overlay refuses the
    // rows that are left without a derived entry.
    let Some(key_source) = key_source else {
        return Ok(());
    };
    let missing = highest_stored - derived + 1;
    let budget = stored_rows.saturating_add(UNSTORED_INDEX_ALLOWANCE);
    pool.generate_addresses(missing.min(budget), key_source, true)
        .map_err(|e| WalletStorageError::AccountRecordInvalid { e })?;
    Ok(())
}

/// Restore every persisted Core pool, including handed-out and pre-derived keys.
///
/// A restored entry is an address the wallet goes on to hand out, so a row is
/// not taken on trust where it can be checked. Each pool is first derived,
/// from the account's public key in `wallet`, through its highest stored
/// index; a row then overlays an entry this wallet derived itself and must
/// name the same address and key.
///
/// That derivation is budgeted by the pool's own row count (see
/// [`UNSTORED_INDEX_ALLOWANCE`]), so a deep pool the wallet really grew
/// restores in full, while an index with almost nothing stored below it
/// cannot force unbounded work. A row above the budget is not restored: it
/// is counted as [`LoadSite::PoolRowBeyondDerivationBudget`] under either
/// policy and the wallet loads without it, the way the discovery walk defers
/// an address it cannot reach.
///
/// `AbsentHardened` pools are the exception. A hardened child cannot be
/// derived from public key material, and a watch-only load holds nothing
/// else, so their rows — the pre-derived platform-node keys — are trusted as
/// stored, as [`restore_provider_platform_node_pool`] already trusts them.
///
/// [`restore_provider_platform_node_pool`]: crate::sqlite::rehydrate::restore_provider_platform_node_pool
pub(crate) fn restore_pools(
    conn: &Connection,
    wallet_id: &WalletId,
    wallet: &key_wallet::wallet::Wallet,
    wallet_info: &mut key_wallet::wallet::ManagedWalletInfo,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    // Prepared once, and cached like every repeated-shape statement in this
    // module: every account and pool below binds the same three shapes.
    let mut stored_pool_types = conn.prepare_cached(
        "SELECT DISTINCT pool_type FROM core_address_pool \
         WHERE wallet_id = ?1 AND account_type = ?2 AND account_index = ?3 \
           AND key_class = ?4 AND user_identity_id = ?5 AND friend_identity_id = ?6",
    )?;
    let mut highest_stored_index = conn.prepare_cached(
        "SELECT MAX(address_index), COUNT(*) FROM core_address_pool \
         WHERE wallet_id = ?1 AND account_type = ?2 AND account_index = ?3 \
           AND key_class = ?4 AND user_identity_id = ?5 AND friend_identity_id = ?6 \
           AND pool_type = ?7",
    )?;
    let mut stored_rows = conn.prepare_cached(
        "SELECT address_index, length(script), script, length(public_key), public_key, \
         key_type, used, reserved_at FROM core_address_pool \
         WHERE wallet_id = ?1 AND account_type = ?2 AND account_index = ?3 \
           AND key_class = ?4 AND user_identity_id = ?5 AND friend_identity_id = ?6 \
           AND pool_type = ?7 ORDER BY address_index",
    )?;
    for mut account in wallet_info.accounts.all_accounts_mut() {
        let account_type = account.managed_account_type().to_account_type();
        let label = accounts::account_type_db_label(&account_type);
        let (user, friend) = accounts::account_dashpay_ids(&account_type);
        let key_source = public_key_source(wallet, account_type);
        let account_index = i64::from(accounts::account_index(&account_type));
        let key_class = i64::from(accounts::account_key_class(&account_type));
        // A row filed under a pool this account does not have is read by no
        // pool below, so whatever it reserves would be dropped by a load that
        // reports success.
        let pool_types: Vec<i64> = account
            .managed_account_type()
            .address_pools()
            .iter()
            .map(|pool| pool_type_to_i64(pool.pool_type))
            .collect();
        let mut stored = stored_pool_types.query(params![
            wallet_id.as_slice(),
            label,
            account_index,
            key_class,
            user.as_slice(),
            friend.as_slice(),
        ])?;
        while let Some(row) = stored.next()? {
            if !pool_types.contains(&row.get(0)?) {
                return Err(WalletStorageError::blob_decode(
                    "core pool row names a pool its account does not have",
                ));
            }
        }
        for pool in account.managed_account_type_mut().address_pools_mut() {
            let pool_rows = params![
                wallet_id.as_slice(),
                label,
                account_index,
                key_class,
                user.as_slice(),
                friend.as_slice(),
                pool_type_to_i64(pool.pool_type)
            ];
            let (highest_stored, stored): (Option<i64>, i64) =
                highest_stored_index.query_row(pool_rows, |row| Ok((row.get(0)?, row.get(1)?)))?;
            if let Some(highest_stored) = highest_stored {
                derive_towards(
                    pool,
                    key_source.as_ref(),
                    i64_to_u32("core_address_pool.address_index", highest_stored)?,
                    // Saturating: a larger count only widens the budget.
                    u32::try_from(stored).unwrap_or(u32::MAX),
                )?;
            }
            let mut beyond_budget = 0usize;
            let mut rows = stored_rows.query(pool_rows)?;
            while let Some(row) = rows.next()? {
                let index = i64_to_u32("core_address_pool.address_index", row.get(0)?)?;
                blob::check_size(row.get(1)?)?;
                let script = dashcore::ScriptBuf::from_bytes(row.get(2)?);
                let address = match dashcore::Address::from_script(&script, pool.network) {
                    Ok(address) => address,
                    Err(source) => {
                        // Earlier readers already account for used and typed platform rows.
                        if row.get::<_, bool>(6)?
                            || (account_type
                                == key_wallet::account::AccountType::ProviderPlatformKeys
                                && (row.get::<_, Option<i64>>(3)?.is_some()
                                    || row.get::<_, Option<i64>>(5)?.is_some()))
                        {
                            continue;
                        }
                        ctx.tolerate_at(
                            LoadSite::UndecodableAddressScript,
                            SiteCoords {
                                wallet_id: Some(*wallet_id),
                                account_type: &account_type,
                                affected: 1,
                                detail: Some(&pool.pool_type),
                            },
                            WalletStorageError::from(source),
                        )?;
                        continue;
                    }
                };
                let key_len: Option<i64> = row.get(3)?;
                let key_type: Option<i64> = row.get(5)?;
                let public_key = match (key_len, key_type) {
                    (None, None) => None,
                    (Some(len), Some(kind)) => {
                        let expected_kind = match account_type {
                            key_wallet::account::AccountType::ProviderOperatorKeys => KEY_TYPE_BLS,
                            key_wallet::account::AccountType::ProviderPlatformKeys => {
                                KEY_TYPE_EDDSA
                            }
                            _ => KEY_TYPE_ECDSA,
                        };
                        if kind != expected_kind {
                            return Err(WalletStorageError::blob_decode(
                                "core pool key type does not match its account",
                            ));
                        }
                        let expected = match kind {
                            KEY_TYPE_ECDSA => ECDSA_PUBLIC_KEY_LEN,
                            KEY_TYPE_EDDSA => EDDSA_PUBLIC_KEY_LEN,
                            KEY_TYPE_BLS => BLS_PUBLIC_KEY_LEN,
                            _ => {
                                return Err(WalletStorageError::blob_decode(
                                    "core_address_pool.key_type is outside 0..=2",
                                ))
                            }
                        };
                        blob::check_fixed_width(
                            len,
                            expected,
                            "core_address_pool.public_key has the wrong length for key_type",
                        )?;
                        let bytes = row.get(4)?;
                        Some(match kind {
                            KEY_TYPE_ECDSA => PublicKeyType::ECDSA(bytes),
                            KEY_TYPE_EDDSA => PublicKeyType::EdDSA(bytes),
                            _ => PublicKeyType::BLS(bytes),
                        })
                    }
                    _ => {
                        return Err(WalletStorageError::blob_decode(
                            "core_address_pool.public_key and key_type nullability differ",
                        ))
                    }
                };
                let used: bool = row.get(6)?;
                let reserved: Option<i64> = row.get(7)?;
                let reserved = reserved
                    .map(|at| i64_to_u64("core_address_pool.reserved_at", at))
                    .transpose()?;
                let child = if pool.pool_type == AddressPoolType::AbsentHardened {
                    key_wallet::bip32::ChildNumber::from_hardened_idx(index)
                } else {
                    key_wallet::bip32::ChildNumber::from_normal_idx(index)
                }
                .map_err(|source| WalletStorageError::AccountRecordInvalid {
                    e: key_wallet::error::Error::Bip32(source),
                })?;
                let mut path = pool.base_path.clone();
                path.push(child);
                let previous = pool.addresses.get(&index);
                if previous.is_none() && pool.pool_type != AddressPoolType::AbsentHardened {
                    // The pool was derived as far as its budget reaches, so a
                    // row above its end is one the budget left out.
                    if key_source.is_some() && pool.highest_generated.is_some_and(|end| index > end)
                    {
                        beyond_budget += 1;
                        continue;
                    }
                    return Err(WalletStorageError::blob_decode(
                        "core pool row has no derived entry to be checked against",
                    ));
                }
                if previous.is_some_and(|info| {
                    info.address != address
                        || public_key
                            .as_ref()
                            .zip(info.public_key.as_ref())
                            .is_some_and(|(a, b)| a != b)
                }) {
                    return Err(WalletStorageError::blob_decode(
                        "core pool row conflicts with its derived address or key",
                    ));
                }
                if pool
                    .address_index
                    .get(&address)
                    .is_some_and(|other| *other != index)
                {
                    return Err(WalletStorageError::blob_decode(
                        "core pool address has multiple derivation indices",
                    ));
                }
                // Chain-derived usage takes precedence over an older pool snapshot.
                let used = used || previous.is_some_and(|info| info.is_used());
                let state = if used {
                    AddressState::Used
                } else if let Some(at) = reserved {
                    AddressState::Reserved { at }
                } else {
                    AddressState::Available
                };
                let info = key_wallet::AddressInfo {
                    address,
                    script_pubkey: script,
                    public_key: public_key
                        .or_else(|| previous.and_then(|info| info.public_key.clone())),
                    index,
                    path,
                    state,
                    tx_count: previous.map_or(0, |info| info.tx_count),
                    total_received: previous.map_or(0, |info| info.total_received),
                    total_sent: previous.map_or(0, |info| info.total_sent),
                    balance: previous.map_or(0, |info| info.balance),
                    label: previous.and_then(|info| info.label.clone()),
                    metadata: previous.map_or_else(Default::default, |info| info.metadata.clone()),
                };
                if let Some(previous) = pool.addresses.remove(&index) {
                    pool.address_index.remove(&previous.address);
                    pool.script_pubkey_index.remove(&previous.script_pubkey);
                }
                pool.address_index.insert(info.address.clone(), index);
                pool.script_pubkey_index
                    .insert(info.script_pubkey.clone(), index);
                pool.addresses.insert(index, info);
                pool.highest_generated =
                    Some(pool.highest_generated.map_or(index, |old| old.max(index)));
                if used {
                    pool.used_indices.insert(index);
                    pool.highest_used = Some(
                        pool.highest_used
                            .map_or(index, |highest| highest.max(index)),
                    );
                }
            }
            if beyond_budget > 0 {
                ctx.note_degraded(
                    LoadSite::PoolRowBeyondDerivationBudget,
                    SiteCoords {
                        wallet_id: Some(*wallet_id),
                        account_type: &account_type,
                        affected: beyond_budget,
                        detail: Some(&pool.pool_type),
                    },
                    "stored pool rows sit further above the rest of their pool than the \
                     derivation budget reaches, so they cannot be checked and are not restored",
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory connection with the full schema migrated in, so tests insert
    /// through the production DDL.
    fn migrated_conn() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn
    }

    #[test]
    fn should_accept_current_and_legacy_pool_account_labels() {
        use dashcore::hashes::Hash;

        let conn = migrated_conn();
        let wallet = [0x79; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet.as_slice()],
        )
        .unwrap();
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            dashcore::address::Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array([7; 20])),
        );
        let script = address.script_pubkey();
        for label in accounts::ACCOUNT_TYPE_LABELS
            .iter()
            .copied()
            .chain(std::iter::once(accounts::LEGACY_STANDARD_LABEL))
        {
            conn.execute("DELETE FROM core_address_pool", []).unwrap();
            conn.execute(
                "INSERT INTO core_address_pool \
                    (wallet_id, account_type, account_index, key_class, pool_type, \
                     address_index, script, used) \
                 VALUES (?1, ?2, 0, 0, 0, 0, ?3, 1)",
                params![wallet.as_slice(), label, script.as_bytes()],
            )
            .unwrap();
            let owner = owning_account_for_script(&conn, &wallet, script.as_bytes())
                .unwrap()
                .unwrap();
            assert_eq!(owner.account_type, label);
            let used = load_used_addresses(&conn, &wallet, dashcore::Network::Testnet).unwrap();
            assert_eq!(used, vec![(address.clone(), owner)]);
        }
    }

    /// UTXO ownership considers every pool row, while the reuse guard selects
    /// only rows explicitly marked used.
    #[test]
    fn shared_script_resolvers_apply_their_distinct_source_filters() {
        use dashcore::address::Payload;
        use dashcore::hashes::Hash;
        use dashcore::PubkeyHash;

        let mut conn = migrated_conn();
        let w = [0x77u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0xAB; 20])),
        );
        let script = address.script_pubkey();
        let tx = conn.transaction().unwrap();
        // Same script under two account types with different account_index.
        // Insert the later-sorting `standard_bip44` FIRST so a bare `LIMIT 1`
        // could return either row depending on SQLite's scan order.
        tx.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'standard_bip44', 9, 0, 0, 0, ?2, 1)",
            params![&w[..], script.as_bytes()],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'coinjoin', 4, 0, 0, 0, ?2, 0)",
            params![&w[..], script.as_bytes()],
        )
        .unwrap();
        // ORDER BY account_type ASC: 'coinjoin' < 'standard_bip44', so the
        // coinjoin row (account_index 4) is the deterministic winner.
        let got = owning_account_for_script(&tx, &w, script.as_bytes())
            .unwrap()
            .expect("shared script owner");
        assert_eq!(
            got.account_index, 4,
            "tie-break must pick the account_type-min row"
        );
        tx.commit().unwrap();

        let used =
            load_used_addresses_with_ctx(&conn, &w, dashcore::Network::Testnet, &LoadCtx::strict())
                .unwrap();
        assert_eq!(used.len(), 1);
        assert_eq!(used[0].0, address);
        assert_eq!(used[0].1.account_index, 9);
    }

    /// A stored `script` that parses as bytes but not as an address must
    /// surface `AddressDecode` — carrying the upstream
    /// `dashcore::address::Error` — not the context-free `BlobDecode` that
    /// discards *why* the script failed.
    #[test]
    fn load_used_addresses_wraps_address_error_as_address_decode() {
        let conn = migrated_conn();
        let w = [0x88u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        // A bare OP_RETURN script is well-formed bytes but not any address
        // type, so `Address::from_script` returns `UnrecognizedScript`.
        let bad_script = [0x6au8];
        conn.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'coinjoin', 0, 0, 0, 0, ?2, 1)",
            params![&w[..], &bad_script[..]],
        )
        .unwrap();

        let err =
            load_used_addresses_with_ctx(&conn, &w, dashcore::Network::Testnet, &LoadCtx::strict())
                .expect_err("an unparseable script must be a hard error");
        assert!(
            matches!(err, WalletStorageError::AddressDecode { .. }),
            "expected AddressDecode carrying the upstream error, got {err:?}"
        );
    }

    /// An empty `script` must be refused by the WRITER, not discovered by
    /// the reader: `load()` turns every stored pool script back into an
    /// address, so such a row degrades the load of the owning wallet and
    /// fails it under a strict policy. `apply_pools` is the only writer of
    /// `core_address_pool.script`, so guarding it closes the producer.
    #[test]
    fn apply_pools_refuses_an_empty_script() {
        use dashcore::address::Payload;
        use dashcore::hashes::Hash;
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::bip32::DerivationPath;
        use key_wallet::managed_account::address_pool::AddressState;
        use key_wallet::AddressInfo;
        use platform_wallet::changeset::AccountAddressPoolEntry;

        let mut conn = migrated_conn();
        let w = [0x99u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(dashcore::PubkeyHash::from_byte_array([0xCD; 20])),
        );
        let entry = AccountAddressPoolEntry {
            account_type: AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            pool_type: AddressPoolType::External,
            addresses: vec![AddressInfo {
                address,
                script_pubkey: dashcore::ScriptBuf::from_bytes(Vec::new()),
                public_key: None,
                index: 7,
                path: DerivationPath::master(),
                state: AddressState::Available,
                tx_count: 0,
                total_received: 0,
                total_sent: 0,
                balance: 0,
                label: None,
                metadata: Default::default(),
            }],
        };

        let tx = conn.transaction().unwrap();
        let err = apply_pools(&tx, &w, std::slice::from_ref(&entry))
            .expect_err("an empty script must be refused");
        match err {
            WalletStorageError::EmptyPoolAddressScript {
                account_type,
                address_index,
            } => {
                assert_eq!(account_type, "standard_bip44");
                assert_eq!(address_index, 7, "the error must name the offending row");
            }
            other => panic!("expected EmptyPoolAddressScript, got {other:?}"),
        }
        let rows: i64 = tx
            .query_row("SELECT COUNT(*) FROM core_address_pool", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(rows, 0, "the guard must refuse before binding the row");
    }

    fn with_pool_of<R>(
        info: &mut key_wallet::wallet::ManagedWalletInfo,
        address: &dashcore::Address,
        read: impl Fn(&mut AddressPool, u32) -> R,
    ) -> R {
        for mut account in info.accounts.all_accounts_mut() {
            for pool in account.managed_account_type_mut().address_pools_mut() {
                if let Some(index) = pool.address_index.get(address).copied() {
                    return read(pool, index);
                }
            }
        }
        panic!("the address must belong to a pool");
    }

    /// A wallet whose BIP44 external pool is stored exactly as derived, and
    /// the first receive address of that pool.
    fn wallet_with_stored_external_pool(
        conn: &mut rusqlite::Connection,
    ) -> (
        key_wallet::wallet::Wallet,
        key_wallet::wallet::ManagedWalletInfo,
        dashcore::Address,
    ) {
        use key_wallet::account::{AccountType, StandardAccountType};
        use key_wallet::wallet::initialization::WalletAccountCreationOptions;
        use key_wallet::wallet::{ManagedWalletInfo, Wallet};

        let wallet = Wallet::new_random(
            dashcore::Network::Testnet,
            WalletAccountCreationOptions::Default,
        )
        .unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet.wallet_id.as_slice()],
        )
        .unwrap();
        let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);
        let xpub = wallet.accounts.standard_bip44_accounts[&0].account_xpub;
        let address = info
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .unwrap()
            .next_receive_address(Some(&xpub), true)
            .unwrap();
        let stored = with_pool_of(&mut info, &address, |pool, _| AccountAddressPoolEntry {
            account_type: AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            pool_type: pool.pool_type,
            addresses: pool.addresses.values().cloned().collect(),
        });
        let tx = conn.transaction().unwrap();
        apply_pools(&tx, &wallet.wallet_id, std::slice::from_ref(&stored)).unwrap();
        tx.commit().unwrap();
        (wallet, info, address)
    }

    /// `restore_pools` overlays a persisted row on the entry the load
    /// projection already built, and a repaired wallet is rebuilt twice in
    /// one load. The accounting that entry carries must come through
    /// unchanged: neither reset by the overlay nor counted a second time.
    #[test]
    fn restore_pools_keeps_existing_address_accounting_exactly_once() {
        let mut conn = migrated_conn();
        let (wallet, mut info, address) = wallet_with_stored_external_pool(&mut conn);
        with_pool_of(&mut info, &address, |pool, index| {
            let entry = pool.addresses.get_mut(&index).unwrap();
            entry.update_stats(700, 200);
            entry.balance = 500;
        });

        let ctx = LoadCtx::strict();
        for round in 1..=2 {
            restore_pools(&conn, &wallet.wallet_id, &wallet, &mut info, &ctx).unwrap();
            let accounting = with_pool_of(&mut info, &address, |pool, index| {
                let entry = &pool.addresses[&index];
                (
                    entry.tx_count,
                    entry.total_received,
                    entry.total_sent,
                    entry.balance,
                )
            });
            assert_eq!(accounting, (1, 700, 200, 500), "restore round {round}");
        }
    }

    /// Checking a stored row means deriving every index below it. An index
    /// with almost nothing stored below it is left out instead: the pool is
    /// derived as far as its budget reaches and no further, the row is
    /// counted, and the rest of the pool is restored. The index used here
    /// would take billions of derivations if the budget did not hold.
    #[test]
    fn restore_pools_skips_a_row_the_derivation_budget_cannot_reach() {
        let mut conn = migrated_conn();
        let (wallet, mut info, address) = wallet_with_stored_external_pool(&mut conn);
        let derived_end = with_pool_of(&mut info, &address, |pool, _| pool.highest_generated)
            .expect("the stored pool is derived");
        let stored_rows: u32 = conn
            .query_row("SELECT COUNT(*) FROM core_address_pool", [], |row| {
                row.get(0)
            })
            .unwrap();
        let unreachable = u32::try_from(i32::MAX).unwrap();
        let moved = conn
            .execute(
                "UPDATE core_address_pool SET address_index = ?1 \
                 WHERE address_index = (SELECT MAX(address_index) FROM core_address_pool)",
                params![i64::from(unreachable)],
            )
            .unwrap();
        assert_eq!(moved, 1);

        let ctx = LoadCtx::strict();
        restore_pools(&conn, &wallet.wallet_id, &wallet, &mut info, &ctx)
            .expect("an unreachable row must not fail the wallet");
        assert_eq!(
            ctx.degradation()
                .by_site
                .get(&LoadSite::PoolRowBeyondDerivationBudget),
            Some(&1),
            "the row left out must be counted"
        );
        with_pool_of(&mut info, &address, |pool, index| {
            assert!(
                !pool.addresses.contains_key(&unreachable),
                "a row that could not be checked must not be restored"
            );
            assert_eq!(pool.addresses[&index].address, address);
            assert_eq!(
                pool.highest_generated,
                Some(derived_end + stored_rows + UNSTORED_INDEX_ALLOWANCE),
                "nothing may be derived beyond the budget"
            );
        });
    }

    #[test]
    fn integer_discriminants_match_migration_domains() {
        use std::collections::BTreeSet;

        let pool_variants = [
            AddressPoolType::External,
            AddressPoolType::Internal,
            AddressPoolType::Absent,
            AddressPoolType::AbsentHardened,
        ];
        for variant in pool_variants {
            match variant {
                AddressPoolType::External
                | AddressPoolType::Internal
                | AddressPoolType::Absent
                | AddressPoolType::AbsentHardened => {}
            }
        }
        let pool_discriminants: BTreeSet<_> = pool_variants
            .iter()
            .copied()
            .map(pool_type_to_i64)
            .collect();
        assert_eq!(pool_discriminants, BTreeSet::from([0, 1, 2, 3]));
        assert_eq!(pool_discriminants.len(), pool_variants.len());

        let key_variants = [
            PublicKeyType::ECDSA(Vec::new()),
            PublicKeyType::EdDSA(Vec::new()),
            PublicKeyType::BLS(Vec::new()),
        ];
        for variant in &key_variants {
            match variant {
                PublicKeyType::ECDSA(_) | PublicKeyType::EdDSA(_) | PublicKeyType::BLS(_) => {}
            }
        }
        let key_discriminants: BTreeSet<_> = key_variants.iter().map(key_type_to_i64).collect();
        assert_eq!(key_discriminants, BTreeSet::from([0, 1, 2]));
        assert_eq!(key_discriminants.len(), key_variants.len());
    }
}
