//! `dpns_name_states` table writer + reader (DPNS username marketplace).
//!
//! Every field maps to an explicit column (the entry is all-primitive), so a
//! row reconstructs a [`DpnsNameStateEntry`] directly — no blob. The status
//! enum's `Sold { to } / Transferred { to }` payload is flattened into the
//! `counterparty_id` column (NULL for `owned`), with the pairing enforced by
//! a table CHECK.

use rusqlite::{params, Transaction};

use platform_wallet::changeset::{DpnsNameSaleStatus, DpnsNameStateChangeSet};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;

// Imports used only by the test-gated reader below.
#[cfg(any(test, feature = "__test-helpers"))]
use {
    dpp::prelude::Identifier, platform_wallet::changeset::DpnsNameStateEntry, rusqlite::Connection,
    std::collections::BTreeMap,
};

pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &DpnsNameStateChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.names.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO dpns_name_states \
                (wallet_id, document_id, identity_id, label, normalized_label, \
                 normalized_parent_domain, price, status, counterparty_id, \
                 created_at_ms, updated_at_ms, transferred_at_ms, last_synced_at_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) \
             ON CONFLICT(wallet_id, document_id) DO UPDATE SET \
                identity_id = excluded.identity_id, \
                label = excluded.label, \
                normalized_label = excluded.normalized_label, \
                normalized_parent_domain = excluded.normalized_parent_domain, \
                price = excluded.price, \
                status = excluded.status, \
                counterparty_id = excluded.counterparty_id, \
                created_at_ms = excluded.created_at_ms, \
                updated_at_ms = excluded.updated_at_ms, \
                transferred_at_ms = excluded.transferred_at_ms, \
                last_synced_at_ms = excluded.last_synced_at_ms",
        )?;
        for (document_id, entry) in &cs.names {
            let (status, counterparty) = status_columns(&entry.status);
            let price = entry
                .price
                .map(|p| crate::sqlite::util::safe_cast::u64_to_i64("dpns_name_states.price", p))
                .transpose()?;
            stmt.execute(params![
                wallet_id.as_slice(),
                document_id.as_slice(),
                entry.wallet_identity_id.as_slice(),
                entry.label,
                entry.normalized_label,
                entry.normalized_parent_domain_name,
                price,
                status,
                counterparty.map(|c| c.to_vec()),
                entry
                    .created_at_ms
                    .map(|v| crate::sqlite::util::safe_cast::u64_to_i64(
                        "dpns_name_states.created_at_ms",
                        v
                    ))
                    .transpose()?,
                entry
                    .updated_at_ms
                    .map(|v| crate::sqlite::util::safe_cast::u64_to_i64(
                        "dpns_name_states.updated_at_ms",
                        v
                    ))
                    .transpose()?,
                entry
                    .transferred_at_ms
                    .map(|v| crate::sqlite::util::safe_cast::u64_to_i64(
                        "dpns_name_states.transferred_at_ms",
                        v
                    ))
                    .transpose()?,
                crate::sqlite::util::safe_cast::u64_to_i64(
                    "dpns_name_states.last_synced_at_ms",
                    entry.last_synced_at_ms
                )?,
            ])?;
        }
    }
    if !cs.removed.is_empty() {
        let mut stmt = tx.prepare_cached(
            "DELETE FROM dpns_name_states WHERE wallet_id = ?1 AND document_id = ?2",
        )?;
        for document_id in &cs.removed {
            stmt.execute(params![wallet_id.as_slice(), document_id.as_slice()])?;
        }
    }
    Ok(())
}

/// Single source of truth for the `dpns_name_states.status` TEXT-column
/// domain + counterparty flattening. The `CHECK (status IN …)` in
/// `migrations/V005__dpns_name_states.rs` must list exactly these values.
pub(crate) fn status_columns(s: &DpnsNameSaleStatus) -> (&'static str, Option<[u8; 32]>) {
    match s {
        DpnsNameSaleStatus::Owned => ("owned", None),
        DpnsNameSaleStatus::Sold { to } => ("sold", Some(to.to_buffer())),
        DpnsNameSaleStatus::Transferred { to } => ("transferred", Some(to.to_buffer())),
    }
}

#[cfg(any(test, feature = "__test-helpers"))]
fn status_from_columns(
    status: &str,
    counterparty: Option<Vec<u8>>,
) -> Result<DpnsNameSaleStatus, WalletStorageError> {
    let to = || -> Result<Identifier, WalletStorageError> {
        let bytes = counterparty
            .as_deref()
            .ok_or_else(|| WalletStorageError::blob_decode("missing counterparty_id for row"))?;
        Identifier::from_bytes(bytes)
            .map_err(|_| WalletStorageError::blob_decode("counterparty_id is not 32 bytes"))
    };
    match status {
        "owned" => Ok(DpnsNameSaleStatus::Owned),
        "sold" => Ok(DpnsNameSaleStatus::Sold { to: to()? }),
        "transferred" => Ok(DpnsNameSaleStatus::Transferred { to: to()? }),
        _ => Err(WalletStorageError::blob_decode(
            "unknown dpns_name_states.status value in row",
        )),
    }
}

/// Read every DPNS name-state row for a wallet, keyed by document id.
/// Test/round-trip helper (the production load path does not re-hydrate
/// name states into the Rust manager; the Swift SwiftData mirror is the UI
/// source).
#[cfg(any(test, feature = "__test-helpers"))]
pub fn read_all(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<Identifier, DpnsNameStateEntry>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT document_id, identity_id, label, normalized_label, normalized_parent_domain, \
                price, status, counterparty_id, created_at_ms, updated_at_ms, \
                transferred_at_ms, last_synced_at_ms \
         FROM dpns_name_states WHERE wallet_id = ?1",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        Ok((
            row.get::<_, Vec<u8>>(0)?,
            row.get::<_, Vec<u8>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<i64>>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<Vec<u8>>>(7)?,
            row.get::<_, Option<i64>>(8)?,
            row.get::<_, Option<i64>>(9)?,
            row.get::<_, Option<i64>>(10)?,
            row.get::<_, i64>(11)?,
        ))
    })?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (
            doc_bytes,
            identity_bytes,
            label,
            normalized_label,
            normalized_parent,
            price,
            status,
            counterparty,
            created_at,
            updated_at,
            transferred_at,
            last_synced,
        ) = row?;
        let document_id = Identifier::from_bytes(&doc_bytes)
            .map_err(|_| WalletStorageError::blob_decode("document_id is not 32 bytes"))?;
        let wallet_identity_id = Identifier::from_bytes(&identity_bytes)
            .map_err(|_| WalletStorageError::blob_decode("identity_id is not 32 bytes"))?;
        out.insert(
            document_id,
            DpnsNameStateEntry {
                document_id,
                wallet_identity_id,
                label,
                normalized_label,
                normalized_parent_domain_name: normalized_parent,
                price: price.map(|p| p as u64),
                status: status_from_columns(&status, counterparty)?,
                created_at_ms: created_at.map(|v| v as u64),
                updated_at_ms: updated_at.map(|v| v as u64),
                transferred_at_ms: transferred_at.map(|v| v as u64),
                last_synced_at_ms: last_synced as u64,
            },
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(tag: u8, status: DpnsNameSaleStatus, price: Option<u64>) -> DpnsNameStateEntry {
        DpnsNameStateEntry {
            document_id: Identifier::from([tag; 32]),
            wallet_identity_id: Identifier::from([0xAA; 32]),
            label: format!("Alice{tag}"),
            normalized_label: format!("a11ce{tag}"),
            normalized_parent_domain_name: "dash".to_string(),
            price,
            status,
            created_at_ms: Some(1_700_000_000_000),
            updated_at_ms: None,
            transferred_at_ms: None,
            last_synced_at_ms: 1_800_000_000_000,
        }
    }

    #[test]
    fn apply_then_read_round_trips_and_upserts_and_removes() {
        let wallet_id: WalletId = [0x22; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        // Insert two: one listed, one unlisted.
        let e0 = entry(0, DpnsNameSaleStatus::Owned, Some(5_000_000_000));
        let e1 = entry(1, DpnsNameSaleStatus::Owned, None);
        let mut cs = DpnsNameStateChangeSet::default();
        cs.names.insert(e0.document_id, e0.clone());
        cs.names.insert(e1.document_id, e1.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs).unwrap();
            tx.commit().unwrap();
        }
        let got = read_all(&conn, &wallet_id).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[&e0.document_id], e0);
        assert_eq!(got[&e1.document_id], e1);

        // Upsert e0 → sold (price cleared by consensus), remove e1.
        let buyer = Identifier::from([0xBB; 32]);
        let mut e0b = e0.clone();
        e0b.price = None;
        e0b.status = DpnsNameSaleStatus::Sold { to: buyer };
        e0b.transferred_at_ms = Some(1_800_000_100_000);
        let mut cs2 = DpnsNameStateChangeSet::default();
        cs2.names.insert(e0b.document_id, e0b.clone());
        cs2.removed.insert(e1.document_id);
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs2).unwrap();
            tx.commit().unwrap();
        }
        let got = read_all(&conn, &wallet_id).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[&e0.document_id], e0b);
    }

    /// `price` is `Credits` (u64) in Rust and the writer routes it through
    /// `u64_to_i64`, so a negative price cannot originate from this crate.
    /// The column CHECK is the backstop that stops a hand-edited or
    /// corrupted row from reading back as valid marketplace state.
    #[test]
    fn negative_price_is_rejected_by_the_schema() {
        let wallet_id: WalletId = [0x33; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let insert_with_price = |conn: &Connection, doc: u8, price: i64| {
            conn.execute(
                "INSERT INTO dpns_name_states \
                    (wallet_id, document_id, identity_id, label, normalized_label, \
                     normalized_parent_domain, price, status, counterparty_id, \
                     last_synced_at_ms) \
                 VALUES (?1, ?2, ?3, 'Alice', 'a11ce', 'dash', ?4, 'owned', NULL, 1)",
                params![&wallet_id[..], &[doc; 32][..], &[0xAAu8; 32][..], price],
            )
        };
        assert!(
            insert_with_price(&conn, 0x01, -1).is_err(),
            "negative price must violate the column CHECK"
        );
        assert!(
            insert_with_price(&conn, 0x02, 0).is_ok(),
            "a zero-credit listing is valid"
        );
    }
}
