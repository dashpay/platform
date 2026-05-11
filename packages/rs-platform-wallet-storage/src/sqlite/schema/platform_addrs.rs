//! `platform_addresses` + `platform_address_sync` writers.

use rusqlite::{params, Connection, Transaction};

use dash_sdk::platform::address_sync::AddressFunds;
use key_wallet::PlatformP2PKHAddress;
use platform_wallet::changeset::PlatformAddressChangeSet;
use platform_wallet::changeset::PlatformAddressSyncStartState;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformAddressChangeSet,
) -> Result<(), SqlitePersisterError> {
    for entry in &cs.addresses {
        tx.execute(
            "INSERT INTO platform_addresses \
                (wallet_id, account_index, address_index, address, balance, nonce) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(wallet_id, address) DO UPDATE SET \
                account_index = excluded.account_index, \
                address_index = excluded.address_index, \
                balance = excluded.balance, \
                nonce = excluded.nonce",
            params![
                wallet_id.as_slice(),
                entry.account_index as i64,
                entry.address_index as i64,
                entry.address.as_bytes(),
                entry.funds.balance as i64,
                entry.funds.nonce as i64,
            ],
        )?;
    }
    // Sync watermark — store the latest non-None values.
    if cs.sync_height.is_some()
        || cs.sync_timestamp.is_some()
        || cs.last_known_recent_block.is_some()
    {
        let (cur_h, cur_t, cur_r): (i64, i64, i64) = tx
            .query_row(
                "SELECT sync_height, sync_timestamp, last_known_recent_block \
                 FROM platform_address_sync WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap_or((0, 0, 0));
        let h = cs
            .sync_height
            .map(|x| x.max(cur_h as u64))
            .unwrap_or(cur_h as u64);
        let t = cs
            .sync_timestamp
            .map(|x| x.max(cur_t as u64))
            .unwrap_or(cur_t as u64);
        let r = cs
            .last_known_recent_block
            .map(|x| x.max(cur_r as u64))
            .unwrap_or(cur_r as u64);
        tx.execute(
            "INSERT INTO platform_address_sync \
                (wallet_id, sync_height, sync_timestamp, last_known_recent_block) \
             VALUES (?1, ?2, ?3, ?4) \
             ON CONFLICT(wallet_id) DO UPDATE SET \
                sync_height = excluded.sync_height, \
                sync_timestamp = excluded.sync_timestamp, \
                last_known_recent_block = excluded.last_known_recent_block",
            params![wallet_id.as_slice(), h as i64, t as i64, r as i64],
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
) -> Result<Vec<PlatformAddressRow>, SqlitePersisterError> {
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
            return Err(SqlitePersisterError::serialization(
                "platform_addresses.address column is not 20 bytes",
            ));
        }
        let mut hash160 = [0u8; 20];
        hash160.copy_from_slice(&address_bytes);
        out.push(PlatformAddressRow {
            account_index: account_index as u32,
            address_index: address_index as u32,
            address: PlatformP2PKHAddress::new(hash160),
            funds: AddressFunds {
                balance: balance as u64,
                nonce: nonce as u32,
            },
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
) -> Result<PlatformAddressSyncStartState, SqlitePersisterError> {
    let row: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT sync_height, sync_timestamp, last_known_recent_block \
             FROM platform_address_sync WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok();
    let (h, t, r) = row.unwrap_or((0, 0, 0));
    Ok(PlatformAddressSyncStartState {
        per_account: Default::default(),
        sync_height: h as u64,
        sync_timestamp: t as u64,
        last_known_recent_block: r as u64,
    })
}

/// Total `platform_addresses` row count per wallet — used by tests that
/// want a stable lower-bound check without re-deriving the address.
pub fn count_per_wallet(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<usize, SqlitePersisterError> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM platform_addresses WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
        |row| row.get(0),
    )?;
    Ok(n as usize)
}
