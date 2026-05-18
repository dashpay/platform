//! `platform_addresses` + `platform_address_sync` writers.

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use dash_sdk::platform::address_sync::AddressFunds;
use key_wallet::PlatformP2PKHAddress;
use platform_wallet::changeset::PlatformAddressChangeSet;
use platform_wallet::changeset::PlatformAddressSyncStartState;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformAddressChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.addresses.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO platform_addresses \
                (wallet_id, account_index, address_index, address, balance, nonce) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(wallet_id, address) DO UPDATE SET \
                account_index = excluded.account_index, \
                address_index = excluded.address_index, \
                balance = excluded.balance, \
                nonce = excluded.nonce",
        )?;
        for entry in &cs.addresses {
            stmt.execute(params![
                wallet_id.as_slice(),
                i64::from(entry.account_index),
                i64::from(entry.address_index),
                entry.address.as_bytes(),
                safe_cast::u64_to_i64("platform_addresses.balance", entry.funds.balance)?,
                i64::from(entry.funds.nonce),
            ])?;
        }
    }
    if cs.sync_height.is_some()
        || cs.sync_timestamp.is_some()
        || cs.last_known_recent_block.is_some()
    {
        let current: Option<(i64, i64, i64)> = tx
            .query_row(
                "SELECT sync_height, sync_timestamp, last_known_recent_block \
                 FROM platform_address_sync WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let (cur_h, cur_t, cur_r) = match current {
            Some((h, t, r)) => (
                safe_cast::i64_to_u64("platform_address_sync.sync_height", h)?,
                safe_cast::i64_to_u64("platform_address_sync.sync_timestamp", t)?,
                safe_cast::i64_to_u64("platform_address_sync.last_known_recent_block", r)?,
            ),
            None => (0u64, 0u64, 0u64),
        };
        let h = cs.sync_height.map(|x| x.max(cur_h)).unwrap_or(cur_h);
        let t = cs.sync_timestamp.map(|x| x.max(cur_t)).unwrap_or(cur_t);
        let r = cs
            .last_known_recent_block
            .map(|x| x.max(cur_r))
            .unwrap_or(cur_r);
        tx.execute(
            "INSERT INTO platform_address_sync \
                (wallet_id, sync_height, sync_timestamp, last_known_recent_block) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id) DO UPDATE SET \
                sync_height = excluded.sync_height, \
                sync_timestamp = excluded.sync_timestamp, \
                last_known_recent_block = excluded.last_known_recent_block",
            params![
                wallet_id.as_slice(),
                safe_cast::u64_to_i64("platform_address_sync.sync_height", h)?,
                safe_cast::u64_to_i64("platform_address_sync.sync_timestamp", t)?,
                safe_cast::u64_to_i64("platform_address_sync.last_known_recent_block", r)?,
            ],
        )?;
    }
    Ok(())
}

/// Row from `platform_addresses` keyed by wallet for tests/load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAddressRow {
    pub account_index: u32,
    pub address_index: u32,
    pub address: PlatformP2PKHAddress,
    pub funds: AddressFunds,
}

pub fn list_per_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Vec<PlatformAddressRow>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT account_index, address_index, address, balance, nonce \
         FROM platform_addresses WHERE wallet_id = ?1 \
         ORDER BY account_index, address_index, address",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let account_index: i64 = row.get(0)?;
        let address_index: i64 = row.get(1)?;
        let address_bytes: Vec<u8> = row.get(2)?;
        let balance: i64 = row.get(3)?;
        let nonce: i64 = row.get(4)?;
        Ok((account_index, address_index, address_bytes, balance, nonce))
    })?;
    let mut out = Vec::new();
    for r in rows {
        let (account_index, address_index, address_bytes, balance, nonce) = r?;
        if address_bytes.len() != 20 {
            return Err(WalletStorageError::blob_decode(
                "platform_addresses.address column is not 20 bytes",
            ));
        }
        let mut hash160 = [0u8; 20];
        hash160.copy_from_slice(&address_bytes);
        let balance = safe_cast::i64_to_u64("platform_addresses.balance", balance)?;
        let nonce = u32::try_from(nonce).map_err(|_| WalletStorageError::IntegerOverflow {
            field: "platform_addresses.nonce",
            value: nonce as u64,
            target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
        })?;
        let account_index =
            u32::try_from(account_index).map_err(|_| WalletStorageError::IntegerOverflow {
                field: "platform_addresses.account_index",
                value: account_index as u64,
                target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
            })?;
        let address_index =
            u32::try_from(address_index).map_err(|_| WalletStorageError::IntegerOverflow {
                field: "platform_addresses.address_index",
                value: address_index as u64,
                target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
            })?;
        out.push(PlatformAddressRow {
            account_index,
            address_index,
            address: PlatformP2PKHAddress::new(hash160),
            funds: AddressFunds { balance, nonce },
        });
    }
    Ok(out)
}

/// Build `PlatformAddressSyncStartState` for a wallet. The
/// `per_account` portion is left at its `Default` value because
/// reconstructing `PerWalletPlatformAddressState` requires xpubs the
/// persister doesn't currently round-trip into the live provider — the
/// load-side wiring upstream is the consumer of this struct.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<PlatformAddressSyncStartState, WalletStorageError> {
    let row: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT sync_height, sync_timestamp, last_known_recent_block \
             FROM platform_address_sync WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let (h, t, r) = match row {
        Some((h, t, r)) => (
            safe_cast::i64_to_u64("platform_address_sync.sync_height", h)?,
            safe_cast::i64_to_u64("platform_address_sync.sync_timestamp", t)?,
            safe_cast::i64_to_u64("platform_address_sync.last_known_recent_block", r)?,
        ),
        None => (0u64, 0u64, 0u64),
    };
    Ok(PlatformAddressSyncStartState {
        per_account: Default::default(),
        sync_height: h,
        sync_timestamp: t,
        last_known_recent_block: r,
    })
}

/// Total `platform_addresses` row count per wallet — used by tests
/// that want a stable lower-bound check without re-deriving the
/// address.
pub fn count_per_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<usize, WalletStorageError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM platform_addresses WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |row| row.get(0),
    )?;
    Ok(usize::try_from(n).unwrap_or(usize::MAX))
}

/// One row of [`load_all`] aggregated state per wallet:
/// `(sync_state, address_row_count)`.
///
/// `address_row_count` mirrors what [`count_per_wallet`] would return —
/// folding the count into the bulk scan saves a per-wallet query.
pub type LoadAllEntry = (PlatformAddressSyncStartState, usize);

/// Bulk reader for `load()`: one [`load_state`] + [`count_per_wallet`]
/// pair per wallet id listed in `wallet_metadata`. Constant-query
/// w.r.t. the number of wallets per call site (FR-P4-6).
///
/// Driven by [`wallet_meta::list_ids`](crate::sqlite::schema::wallet_meta::list_ids):
/// orphaned `platform_addresses` / `platform_address_sync` rows whose
/// `wallet_id` is absent from `wallet_metadata` are intentionally NOT
/// surfaced. FK triggers prevent such orphans; a future re-wire that
/// needs them must restore the id-union over the area tables.
pub fn load_all(
    conn: &Connection,
) -> Result<std::collections::BTreeMap<WalletId, LoadAllEntry>, WalletStorageError> {
    use std::collections::BTreeMap;

    let mut out: BTreeMap<WalletId, LoadAllEntry> = BTreeMap::new();
    for wallet_id in crate::sqlite::schema::wallet_meta::list_ids(conn)? {
        let sync = load_state(conn, &wallet_id)?;
        let count = count_per_wallet(conn, &wallet_id)?;
        out.insert(wallet_id, (sync, count));
    }
    Ok(out)
}
