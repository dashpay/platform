//! `tracked_masternodes` table writer + reader (wallet-independent
//! masternodes the user follows).
//!
//! Whole-set semantics per network: `replace_all` deletes the network's
//! rows and re-inserts the supplied set inside the caller's transaction —
//! the set is user-curated and small, and the trait contract
//! (`PlatformWalletPersistence::persist_tracked_masternodes`) is a
//! whole-set write. `snapshot_json` is an opaque versioned document of
//! PUBLIC material only (see the V006 migration doc); this module never
//! interprets it beyond storing and returning it.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::masternode::{snapshot_from_json, snapshot_to_json, TrackedMasternode};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast;

/// Replace every row for `network` with `records`.
pub fn replace_all(
    tx: &Transaction<'_>,
    network: dashcore::Network,
    records: &[TrackedMasternode],
) -> Result<(), WalletStorageError> {
    let network = network.to_string();
    tx.execute(
        "DELETE FROM tracked_masternodes WHERE network = ?1",
        params![network],
    )?;
    if records.is_empty() {
        return Ok(());
    }
    let mut stmt = tx.prepare_cached(
        "INSERT INTO tracked_masternodes \
            (network, pro_tx_hash, label, added_at, snapshot_json) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for record in records {
        let added_at = safe_cast::u64_to_i64("tracked_masternodes.added_at", record.added_at)?;
        stmt.execute(params![
            network,
            record.pro_tx_hash.as_slice(),
            record.label,
            added_at,
            snapshot_to_json(&record.snapshot),
        ])?;
    }
    Ok(())
}

/// Every row for `network`, oldest-tracked first.
pub fn load_all(
    conn: &Connection,
    network: dashcore::Network,
) -> Result<Vec<TrackedMasternode>, WalletStorageError> {
    let mut stmt = conn.prepare_cached(
        "SELECT pro_tx_hash, label, added_at, snapshot_json \
         FROM tracked_masternodes WHERE network = ?1 \
         ORDER BY added_at, pro_tx_hash",
    )?;
    let rows = stmt.query_map(params![network.to_string()], |row| {
        let hash: Vec<u8> = row.get(0)?;
        let label: Option<String> = row.get(1)?;
        let added_at: i64 = row.get(2)?;
        let snapshot: String = row.get(3)?;
        Ok((hash, label, added_at, snapshot))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (hash, label, added_at, snapshot) = row?;
        let Ok(pro_tx_hash) = <[u8; 32]>::try_from(hash.as_slice()) else {
            // Length is CHECK-constrained; a mismatch means external
            // tampering — skip rather than fail the whole load.
            continue;
        };
        out.push(TrackedMasternode {
            pro_tx_hash,
            label,
            added_at: added_at.max(0) as u64,
            snapshot: snapshot_from_json(&snapshot),
        });
    }
    Ok(out)
}
