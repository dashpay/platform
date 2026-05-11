//! `wallet_metadata` writer + helpers.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::WalletMetadataEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::SqlitePersisterError;

/// Insert / replace a `wallet_metadata` row.
pub fn upsert(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entry: &WalletMetadataEntry,
) -> Result<(), SqlitePersisterError> {
    let network = network_to_str(entry.network);
    tx.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id) DO UPDATE SET network = excluded.network, \
                                              birth_height = excluded.birth_height",
        params![wallet_id.as_slice(), network, entry.birth_height],
    )?;
    Ok(())
}

/// Ensure a `wallet_metadata` parent row exists for the given id. Used
/// by tests that exercise persistence without going through registration.
///
/// Idempotent — silently a no-op when the row already exists. Defaults
/// `network = "testnet"`, `birth_height = 0` (the same fall-back the
/// SPV scan uses when the chain tip is unknown).
pub fn ensure_exists(conn: &Connection, wallet_id: &WalletId) -> Result<(), SqlitePersisterError> {
    conn.execute(
        "INSERT OR IGNORE INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, ?2, ?3)",
        params![wallet_id.as_slice(), "testnet", 0i64],
    )?;
    Ok(())
}

/// All known wallet ids (used by `delete_wallet`, `load`, `inspect`).
pub fn list_ids(conn: &Connection) -> Result<Vec<WalletId>, SqlitePersisterError> {
    let mut stmt = conn.prepare("SELECT wallet_id FROM wallet_metadata ORDER BY wallet_id")?;
    let rows = stmt.query_map([], |row| {
        let bytes: Vec<u8> = row.get(0)?;
        let mut wid = [0u8; 32];
        if bytes.len() == 32 {
            wid.copy_from_slice(&bytes);
        }
        Ok(wid)
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Lookup `(network, birth_height)` for a wallet, if known.
pub fn fetch(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Option<(String, u32)>, SqlitePersisterError> {
    let mut stmt =
        conn.prepare("SELECT network, birth_height FROM wallet_metadata WHERE wallet_id = ?1")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    if let Some(row) = rows.next()? {
        let network: String = row.get(0)?;
        let height: i64 = row.get(1)?;
        Ok(Some((network, height as u32)))
    } else {
        Ok(None)
    }
}

/// Delete a wallet_metadata row (cascade triggers fire).
pub fn delete(tx: &Transaction<'_>, wallet_id: &WalletId) -> Result<usize, SqlitePersisterError> {
    let n = tx.execute(
        "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
    )?;
    Ok(n)
}

fn network_to_str(net: key_wallet::Network) -> &'static str {
    match net {
        key_wallet::Network::Mainnet => "mainnet",
        key_wallet::Network::Testnet => "testnet",
        key_wallet::Network::Devnet => "devnet",
        key_wallet::Network::Regtest => "regtest",
    }
}

/// Inverse of [`network_to_str`].
pub fn parse_network(s: &str) -> Option<key_wallet::Network> {
    match s {
        "mainnet" => Some(key_wallet::Network::Mainnet),
        "testnet" => Some(key_wallet::Network::Testnet),
        "devnet" => Some(key_wallet::Network::Devnet),
        "regtest" => Some(key_wallet::Network::Regtest),
        _ => None,
    }
}
