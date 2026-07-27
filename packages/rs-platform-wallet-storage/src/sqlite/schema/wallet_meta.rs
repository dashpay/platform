//! `wallet_metadata` writer + helpers.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::changeset::WalletMetadataEntry;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;

/// Insert / replace a `wallet_metadata` row.
pub fn upsert(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    entry: &WalletMetadataEntry,
) -> Result<(), WalletStorageError> {
    let network = network_to_str(entry.network);
    let mut stmt = tx.prepare_cached(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id) DO UPDATE SET network = excluded.network, \
                                              birth_height = excluded.birth_height",
    )?;
    stmt.execute(params![wallet_id.as_slice(), network, entry.birth_height])?;
    Ok(())
}

/// Ensure a `wallet_metadata` parent row exists for the given id. Used
/// by tests that exercise persistence without going through registration.
///
/// Idempotent — silently a no-op when the row already exists. Defaults
/// `network = "testnet"`, `birth_height = 0` (the same fall-back the
/// SPV scan uses when the chain tip is unknown).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn ensure_exists(conn: &Connection, wallet_id: &WalletId) -> Result<(), WalletStorageError> {
    conn.execute(
        "INSERT OR IGNORE INTO wallet_metadata (wallet_id, network, birth_height) \
         VALUES (?1, ?2, ?3)",
        params![wallet_id.as_slice(), "testnet", 0i64],
    )?;
    Ok(())
}

/// All known wallet ids (used by `delete_wallet`, `load`, `inspect`).
pub fn list_ids(conn: &Connection) -> Result<Vec<WalletId>, WalletStorageError> {
    let mut stmt = conn.prepare("SELECT wallet_id FROM wallet_metadata ORDER BY wallet_id")?;
    let rows = stmt.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
    let mut out = Vec::new();
    for r in rows {
        let bytes = r?;
        let wid = <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| {
            WalletStorageError::InvalidWalletIdLength {
                actual: bytes.len(),
            }
        })?;
        out.push(wid);
    }
    Ok(out)
}

/// Lookup `(network, birth_height)` for a wallet, if known.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn fetch(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<Option<(String, u32)>, WalletStorageError> {
    let mut stmt =
        conn.prepare("SELECT network, birth_height FROM wallet_metadata WHERE wallet_id = ?1")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    if let Some(row) = rows.next()? {
        let network: String = row.get(0)?;
        let height: i64 = row.get(1)?;
        let height = u32::try_from(height).map_err(|_| WalletStorageError::IntegerOverflow {
            field: "wallet_metadata.birth_height",
            value: height as u64,
            target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
        })?;
        Ok(Some((network, height)))
    } else {
        Ok(None)
    }
}

/// Delete a wallet_metadata row (native `ON DELETE CASCADE` fires).
pub fn delete(tx: &Transaction<'_>, wallet_id: &WalletId) -> Result<usize, WalletStorageError> {
    let n = tx.execute(
        "DELETE FROM wallet_metadata WHERE wallet_id = ?1",
        params![wallet_id.as_slice()],
    )?;
    Ok(n)
}

/// Single source of truth for the `wallet_metadata.network` TEXT-column
/// domain.
///
/// Mirrors every variant of [`key_wallet::Network`] (writer side:
/// [`network_to_str`]). The migration in `migrations/V001__initial.rs`
/// interpolates this array into a `CHECK (network IN (...))` clause so
/// an unknown label is rejected at insert time rather than landing as
/// silent garbage. The `network_labels_match_enum` unit test below
/// enforces set-equality between this array and the writer's output —
/// drift (a renamed/added variant) becomes a failing test, not a
/// runtime divergence between Rust and SQLite.
pub(crate) const NETWORK_LABELS: &[&str] = &["mainnet", "testnet", "devnet", "regtest"];

fn network_to_str(net: key_wallet::Network) -> &'static str {
    match net {
        key_wallet::Network::Mainnet => "mainnet",
        key_wallet::Network::Testnet => "testnet",
        key_wallet::Network::Devnet => "devnet",
        key_wallet::Network::Regtest => "regtest",
    }
}

/// Inverse of `network_to_str`.
#[cfg(any(test, feature = "__test-helpers"))]
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

    /// Every [`key_wallet::Network`] variant — kept exhaustive by the
    /// `match` arm below, which the compiler's exhaustiveness check
    /// turns into a build failure if upstream adds a variant.
    fn all_network_variants() -> Vec<key_wallet::Network> {
        // The match's exhaustiveness fails to compile on a new variant.
        // Mapping every existing variant to itself keeps the list and the
        // enum in lockstep.
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
