//! `tracked_masternodes` table writer + reader (wallet-independent
//! masternodes the user follows).
//!
//! Whole-set semantics per network: `replace_all` deletes the network's
//! rows and re-inserts the supplied set inside the caller's transaction —
//! the set is user-curated and small, and the trait contract
//! (`PlatformWalletPersistence::persist_tracked_masternodes`) is a
//! whole-set write. `snapshot_json` is an opaque versioned document of
//! PUBLIC material only (see the V013 migration doc); this module never
//! interprets it beyond storing and returning it.

use rusqlite::{params, Connection, Transaction};

use platform_wallet::masternode::{snapshot_from_json, snapshot_to_json, TrackedMasternode};

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::wallets::network_to_str;
use crate::sqlite::util::safe_cast;

/// Replace every row for `network` with `records`.
pub fn replace_all(
    tx: &Transaction<'_>,
    network: dashcore::Network,
    records: &[TrackedMasternode],
) -> Result<(), WalletStorageError> {
    // Bound through `network_to_str`, not `Display`: the V013 CHECK pins
    // this label domain and `network_labels_match_enum` is what keeps the
    // two in step. A `Display` change upstream would otherwise turn every
    // write into a CHECK failure with no compile-time signal.
    let network = network_to_str(network);
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
///
/// # Errors
///
/// [`WalletStorageError::InvalidWalletIdLength`] for a `pro_tx_hash` that
/// is not 32 bytes, under [`LoadPolicy::Strict`](crate::LoadPolicy). Under
/// Recovery the row is skipped and counted into the degradation report
/// rather than vanishing silently.
pub fn load_all(
    conn: &Connection,
    network: dashcore::Network,
    ctx: &LoadCtx,
) -> Result<Vec<TrackedMasternode>, WalletStorageError> {
    let mut stmt = conn.prepare_cached(
        "SELECT pro_tx_hash, label, added_at, snapshot_json \
         FROM tracked_masternodes WHERE network = ?1 \
         ORDER BY added_at, pro_tx_hash",
    )?;
    let rows = stmt.query_map(params![network_to_str(network)], |row| {
        let hash: Vec<u8> = row.get(0)?;
        let label: Option<String> = row.get(1)?;
        let added_at: i64 = row.get(2)?;
        let snapshot: String = row.get(3)?;
        Ok((hash, label, added_at, snapshot))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (hash, label, added_at, snapshot) = row?;
        let pro_tx_hash = match <[u8; 32]>::try_from(hash.as_slice()) {
            Ok(pro_tx_hash) => pro_tx_hash,
            Err(_) => {
                ctx.tolerate(
                    LoadSite::TrackedMasternodeIdLength,
                    WalletStorageError::InvalidWalletIdLength {
                        column: "tracked_masternodes.pro_tx_hash",
                        actual: hash.len(),
                    },
                )?;
                continue;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A row whose `pro_tx_hash` is not 32 bytes — the shape that can only
    /// reach the file with the V013 CHECK bypassed.
    fn conn_with_short_pro_tx_hash() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        conn.execute(
            "INSERT INTO tracked_masternodes \
                (network, pro_tx_hash, label, added_at, snapshot_json) \
             VALUES ('testnet', ?1, 'short', 1, '{}')",
            params![&[0xAAu8; 8][..]],
        )
        .unwrap();
        conn
    }

    /// Strict's contract is that any inconsistency aborts the load. A
    /// silent `continue` returned `Ok` with the row simply gone.
    #[test]
    fn strict_aborts_on_a_short_pro_tx_hash() {
        let conn = conn_with_short_pro_tx_hash();
        let err = load_all(&conn, dashcore::Network::Testnet, &LoadCtx::strict())
            .expect_err("Strict must abort, not return Ok with the row dropped");
        match err {
            WalletStorageError::InvalidWalletIdLength { column, actual } => {
                assert_eq!(column, "tracked_masternodes.pro_tx_hash");
                assert_eq!(actual, 8);
            }
            other => panic!("expected InvalidWalletIdLength, got {other:?}"),
        }
    }

    /// Recovery may drop the row, but the degradation report is the whole
    /// point of the mode — a drop that reports clean is the worst outcome.
    #[test]
    fn recovery_counts_the_dropped_row_into_the_degradation_report() {
        let conn = conn_with_short_pro_tx_hash();
        let ctx = LoadCtx::recovery();
        let rows = load_all(&conn, dashcore::Network::Testnet, &ctx)
            .expect("Recovery must tolerate the row rather than fail");
        assert!(rows.is_empty(), "the malformed row cannot be rehydrated");
        let degradation = ctx.degradation();
        assert!(
            degradation.degraded,
            "a dropped row must mark the load degraded"
        );
        assert_eq!(
            degradation
                .by_site
                .get(&LoadSite::TrackedMasternodeIdLength),
            Some(&1),
            "the drop must be counted at its own site"
        );
    }

    /// `network` is CHECK-constrained to the label domain `network_to_str`
    /// produces. Binding it through `Display` instead would turn an
    /// upstream rendering change into a silent CHECK failure on every
    /// write, with no compile-time signal.
    #[test]
    fn every_network_label_round_trips_through_the_check_constraint() {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        for network in [
            dashcore::Network::Mainnet,
            dashcore::Network::Testnet,
            dashcore::Network::Devnet,
            dashcore::Network::Regtest,
        ] {
            let record = TrackedMasternode {
                pro_tx_hash: [0x31u8; 32],
                label: None,
                added_at: 0,
                snapshot: Default::default(),
            };
            let tx = conn.transaction().unwrap();
            replace_all(&tx, network, std::slice::from_ref(&record))
                .expect("the bound label must satisfy the network CHECK");
            tx.commit().unwrap();
            let rows = load_all(&conn, network, &LoadCtx::strict()).expect("load");
            assert_eq!(rows.len(), 1, "{network:?} must round-trip");
        }
    }
}
