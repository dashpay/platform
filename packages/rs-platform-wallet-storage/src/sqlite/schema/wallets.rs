//! `wallets` writer + helpers.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::WalletMetadataEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;

/// Insert / replace a `wallets` row.
pub fn upsert(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entry: &WalletMetadataEntry,
) -> Result<(), WalletStorageError> {
    let network = network_to_str(entry.network);
    let mut stmt = tx.prepare_cached(
        "INSERT INTO wallets (wallet_id, network, birth_height) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id) DO UPDATE SET network = excluded.network, \
                                              birth_height = excluded.birth_height",
    )?;
    stmt.execute(params![wallet_id.as_slice(), network, entry.birth_height])?;
    Ok(())
}

/// Ensure a `wallets` parent row exists for the given id (test helper).
/// Idempotent; defaults `network = "testnet"`, `birth_height = 0`.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn ensure_exists(conn: &Connection, wallet_id: &WalletId) -> Result<(), WalletStorageError> {
    conn.execute(
        "INSERT OR IGNORE INTO wallets (wallet_id, network, birth_height) \
         VALUES (?1, ?2, ?3)",
        params![wallet_id.as_slice(), "testnet", 0i64],
    )?;
    Ok(())
}

/// All known wallet ids (used by `delete_wallet`, `load`, `inspect`).
pub fn list_ids(conn: &Connection) -> Result<Vec<WalletId>, WalletStorageError> {
    let mut stmt = conn.prepare("SELECT wallet_id FROM wallets ORDER BY wallet_id")?;
    let rows = stmt.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
    let mut out = Vec::new();
    for r in rows {
        let bytes = r?;
        let wid = super::id32("wallets.wallet_id", &bytes)?;
        out.push(wid);
    }
    Ok(out)
}

/// Lookup `(network, birth_height)` for a wallet, if known.
pub fn fetch(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Option<(String, u32)>, WalletStorageError> {
    let mut stmt =
        conn.prepare("SELECT network, birth_height FROM wallets WHERE wallet_id = ?1")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    if let Some(row) = rows.next()? {
        let network: String = row.get(0)?;
        let height: i64 = row.get(1)?;
        let height = crate::sqlite::util::safe_cast::i64_to_u32("wallets.birth_height", height)?;
        Ok(Some((network, height)))
    } else {
        Ok(None)
    }
}

/// Delete a wallets row (native `ON DELETE CASCADE` fires).
pub fn delete(tx: &Transaction<'_>, wallet_id: &WalletId) -> Result<usize, WalletStorageError> {
    let n = tx.execute(
        "DELETE FROM wallets WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
    )?;
    Ok(n)
}

/// Source of truth for the `wallets.network` TEXT domain, mirroring
/// [`key_wallet::Network`]. `migrations/V001__initial.rs` interpolates it into
/// a `CHECK (network IN (...))` clause; `network_labels_match_enum` keeps it in
/// sync with [`network_to_str`].
pub(crate) const NETWORK_LABELS: &[&str] = &["mainnet", "testnet", "devnet", "regtest"];

pub(crate) fn network_to_str(net: key_wallet::Network) -> &'static str {
    match net {
        key_wallet::Network::Mainnet => "mainnet",
        key_wallet::Network::Testnet => "testnet",
        key_wallet::Network::Devnet => "devnet",
        key_wallet::Network::Regtest => "regtest",
    }
}

/// Inverse of `network_to_str`.
pub fn parse_network(s: &str) -> Option<key_wallet::Network> {
    match s {
        "mainnet" => Some(key_wallet::Network::Mainnet),
        "testnet" => Some(key_wallet::Network::Testnet),
        "devnet" => Some(key_wallet::Network::Devnet),
        "regtest" => Some(key_wallet::Network::Regtest),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Every [`key_wallet::Network`] variant; the `match` below fails to
    /// compile if upstream adds one, keeping the list in lockstep.
    fn all_network_variants() -> Vec<key_wallet::Network> {
        let variants = [
            key_wallet::Network::Mainnet,
            key_wallet::Network::Testnet,
            key_wallet::Network::Devnet,
            key_wallet::Network::Regtest,
        ];
        for v in &variants {
            match v {
                key_wallet::Network::Mainnet
                | key_wallet::Network::Testnet
                | key_wallet::Network::Devnet
                | key_wallet::Network::Regtest => {}
            }
        }
        variants.to_vec()
    }

    #[test]
    fn network_labels_match_enum() {
        let from_writer: HashSet<&'static str> = all_network_variants()
            .iter()
            .copied()
            .map(network_to_str)
            .collect();
        let from_const: HashSet<&'static str> = NETWORK_LABELS.iter().copied().collect();
        assert_eq!(
            from_writer, from_const,
            "NETWORK_LABELS ({:?}) drifted from network_to_str codomain ({:?})",
            from_const, from_writer
        );
    }

    #[test]
    fn parse_network_round_trips_every_label() {
        for label in NETWORK_LABELS {
            let parsed =
                parse_network(label).unwrap_or_else(|| panic!("parse_network({label}) was None"));
            assert_eq!(network_to_str(parsed), *label);
        }
    }
}
