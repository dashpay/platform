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

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use platform_wallet::changeset::AccountAddressPoolEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use key_wallet::managed_account::address_pool::AddressPoolType;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::accounts;
use crate::sqlite::schema::blob;

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

const UPSERT_POOL_SQL: &str = "INSERT INTO core_address_pool \
        (wallet_id, account_type, account_index, key_class, user_identity_id, friend_identity_id, \
         pool_type, address_index, script, used) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
     ON CONFLICT(wallet_id, account_type, account_index, key_class, user_identity_id, \
                 friend_identity_id, pool_type, address_index) \
     DO UPDATE SET \
        script = excluded.script, \
        used = MAX(used, excluded.used)";

/// Expand `account_address_pools` snapshots into per-index
/// `core_address_pool` rows. Idempotent: `script` is derivation-stable and
/// `used` is monotonic (`MAX`), so re-applying the same snapshot is a no-op
/// and a used address can never revert to unused (the reuse-guard invariant).
pub fn apply_pools(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    pools: &[AccountAddressPoolEntry],
) -> Result<(), WalletStorageError> {
    if pools.is_empty() {
        return Ok(());
    }
    let mut stmt = tx.prepare_cached(UPSERT_POOL_SQL)?;
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
            stmt.execute(params![
                wallet_id.as_slice(),
                account_type,
                account_index,
                key_class,
                user_identity_id.as_slice(),
                friend_identity_id.as_slice(),
                pool_type,
                i64::from(info.index),
                info.script_pubkey.as_bytes(),
                info.used,
            ])?;
        }
    }
    Ok(())
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

/// Full owning-account identity for a UTXO, matched by its `script_pubkey`
/// against a pool row. `None` when no pool row covers the script — the caller
/// then falls back to the one-way historical-attribution default (R7): funds
/// are never dropped, only conservatively bucketed.
pub(crate) fn owning_account_for_script(
    conn: &Connection,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<Option<OwningAccount>, WalletStorageError> {
    // A script can appear under several pool rows (distinct account_type /
    // key_class / identity pair / pool_type share the same `script_pubkey`
    // for reused keys); an explicit PK-ordered tie-break makes the pick
    // deterministic instead of relying on SQLite's arbitrary `LIMIT 1` row.
    let row = conn
        .prepare_cached(
            "SELECT account_type, account_index, user_identity_id, friend_identity_id \
             FROM core_address_pool \
             WHERE wallet_id = ?1 AND script = ?2 \
             ORDER BY account_type, account_index, key_class, user_identity_id, \
                      friend_identity_id, pool_type, address_index ASC \
             LIMIT 1",
        )?
        .query_row(params![wallet_id.as_slice(), script], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })
        .optional()?;
    let Some((account_type, index, user, friend)) = row else {
        return Ok(None);
    };
    let account_index =
        crate::sqlite::util::safe_cast::i64_to_u32("core_address_pool.account_index", index)?;
    let user_identity_id = <[u8; 32]>::try_from(user.as_slice())
        .map_err(|_| WalletStorageError::blob_decode("core_address_pool.user_identity_id"))?;
    let friend_identity_id = <[u8; 32]>::try_from(friend.as_slice())
        .map_err(|_| WalletStorageError::blob_decode("core_address_pool.friend_identity_id"))?;
    Ok(Some(OwningAccount {
        account_type,
        account_index,
        user_identity_id,
        friend_identity_id,
    }))
}

/// Owning account index for a UTXO, matched by its `script_pubkey` against a
/// pool row. `None` when no pool row covers the script — the UTXO writer
/// then falls back to account 0 (the one-way historical-attribution default,
/// R7): funds are never dropped, only conservatively bucketed.
pub fn account_index_for_script(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    script: &[u8],
) -> Result<Option<u32>, WalletStorageError> {
    Ok(owning_account_for_script(tx, wallet_id, script)?.map(|o| o.account_index))
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
/// A `script` reused across accounts yields several rows; the owner is picked
/// by the same PK-ordered tie-break as [`owning_account_for_script`]
/// (`account_type` first), so both resolvers attribute a shared script
/// identically. `network` turns each stored `script` back into an
/// [`Address`](dashcore::Address); a script that isn't a valid address is a
/// hard error — corruption is never silently dropped, matching
/// [`crate::sqlite::schema::core_state::load_used_addresses`].
pub fn load_used_addresses(
    conn: &rusqlite::Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<Vec<(dashcore::Address, OwningAccount)>, WalletStorageError> {
    // Gate the largest stored `script` with a cheap aggregate BEFORE the
    // ordered read materializes or sorts any blob, so a corrupt/oversize
    // column raises a typed `BlobTooLarge` (the crate's 16 MiB cap) rather
    // than SQLite's own `TooBig` mid-sort, and never OOMs the host.
    let max_script_len: Option<i64> = conn.query_row(
        "SELECT MAX(length(script)) FROM core_address_pool \
         WHERE wallet_id = ?1 AND used = 1",
        params![wallet_id.as_slice()],
        |row| row.get(0),
    )?;
    if let Some(len) = max_script_len {
        blob::check_size(len)?;
    }
    // Order by script then the pool PK so the first row per script is the
    // tie-break winner; a `HashSet` on script drops the trailing duplicates.
    let mut stmt = conn.prepare(
        "SELECT script, account_type, account_index, user_identity_id, friend_identity_id \
         FROM core_address_pool \
         WHERE wallet_id = ?1 AND used = 1 \
         ORDER BY script, account_type, account_index, key_class, user_identity_id, \
                  friend_identity_id, pool_type, address_index",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Vec<u8>>(3)?,
            row.get::<_, Vec<u8>>(4)?,
        ))
    })?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for r in rows {
        let (raw_script, account_type, index, user, friend) = r?;
        if !seen.insert(raw_script.clone()) {
            continue;
        }
        let script = dashcore::ScriptBuf::from_bytes(raw_script);
        let address = dashcore::Address::from_script(&script, network).map_err(|_| {
            WalletStorageError::blob_decode("core_address_pool.script not an address")
        })?;
        let account_index =
            crate::sqlite::util::safe_cast::i64_to_u32("core_address_pool.account_index", index)?;
        let user_identity_id = <[u8; 32]>::try_from(user.as_slice())
            .map_err(|_| WalletStorageError::blob_decode("core_address_pool.user_identity_id"))?;
        let friend_identity_id = <[u8; 32]>::try_from(friend.as_slice())
            .map_err(|_| WalletStorageError::blob_decode("core_address_pool.friend_identity_id"))?;
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

    /// `account_index_for_script` is deterministic when several pool rows
    /// share one script: the PK-ordered tie-break (`account_type` first) picks
    /// the same row regardless of insert order, closing the `LIMIT 1`-without-
    /// `ORDER BY` non-determinism.
    #[test]
    fn account_index_for_script_is_deterministic_on_shared_script() {
        let mut conn = migrated_conn();
        let w = [0x77u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        let script = [0xABu8; 25];
        let tx = conn.transaction().unwrap();
        // Same script under two account types with different account_index.
        // Insert the later-sorting `standard_bip44` FIRST so a bare `LIMIT 1`
        // could return either row depending on SQLite's scan order.
        tx.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'standard_bip44', 9, 0, 0, 0, ?2, 1)",
            params![&w[..], &script[..]],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO core_address_pool \
                (wallet_id, account_type, account_index, key_class, pool_type, \
                 address_index, script, used) \
             VALUES (?1, 'coinjoin', 4, 0, 0, 0, ?2, 1)",
            params![&w[..], &script[..]],
        )
        .unwrap();
        // ORDER BY account_type ASC: 'coinjoin' < 'standard_bip44', so the
        // coinjoin row (account_index 4) is the deterministic winner.
        let got = account_index_for_script(&tx, &w, &script).unwrap();
        assert_eq!(got, Some(4), "tie-break must pick the account_type-min row");
        tx.commit().unwrap();
    }

    #[test]
    fn pool_type_discriminants_are_stable_and_distinct() {
        let all = [
            AddressPoolType::External,
            AddressPoolType::Internal,
            AddressPoolType::Absent,
            AddressPoolType::AbsentHardened,
        ];
        let mapped: Vec<i64> = all.iter().copied().map(pool_type_to_i64).collect();
        assert_eq!(mapped, vec![0, 1, 2, 3]);
    }
}
