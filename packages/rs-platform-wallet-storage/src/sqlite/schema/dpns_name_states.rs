//! `dpns_name_states` table writer + reader (DPNS username marketplace).
//!
//! Every field maps to an explicit column (the entry is all-primitive), so a
//! row reconstructs a [`DpnsNameStateEntry`] directly — no blob. The status
//! enum's `Sold { to } / Transferred { to }` payload is flattened into the
//! `counterparty_id` column (NULL for `owned`), with the pairing enforced by
//! a table CHECK.

use dpp::prelude::Identifier;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use platform_wallet::changeset::{DpnsNameSaleStatus, DpnsNameStateChangeSet, DpnsNameStateEntry};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::util::safe_cast::i64_to_u64;

// Import used only by the test-gated whole-table reader below.
#[cfg(any(test, feature = "__test-helpers"))]
use std::collections::BTreeMap;

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
/// `migrations/V011__dpns_name_states.rs` must list exactly these values.
pub(crate) fn status_columns(s: &DpnsNameSaleStatus) -> (&'static str, Option<[u8; 32]>) {
    match s {
        DpnsNameSaleStatus::Owned => ("owned", None),
        DpnsNameSaleStatus::Sold { to } => ("sold", Some(to.to_buffer())),
        DpnsNameSaleStatus::Transferred { to } => ("transferred", Some(to.to_buffer())),
    }
}

/// Inverse of [`status_columns`]. Ungated: the production
/// [`get_by_identity_and_label`] reader decodes rows too, not just the
/// test-only whole-table reader.
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

/// The projection every reader in this module selects, in the exact order
/// [`row_columns`] indexes and [`entry_from_columns`] destructures. Shared
/// so the two readers cannot drift apart.
const ROW_PROJECTION: &str = "document_id, identity_id, label, normalized_label, \
     normalized_parent_domain, price, status, counterparty_id, created_at_ms, \
     updated_at_ms, transferred_at_ms, last_synced_at_ms";

/// One raw row in [`ROW_PROJECTION`] order. Named so the readers stay under
/// clippy's `type_complexity` bar.
type RowColumns = (
    Vec<u8>,
    Vec<u8>,
    String,
    String,
    String,
    Option<i64>,
    String,
    Option<Vec<u8>>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    i64,
);

/// Pull one row's columns out inside a rusqlite row callback, which may
/// only fail with [`rusqlite::Error`]. Typed decoding (which needs
/// [`WalletStorageError`]) happens afterwards in [`entry_from_columns`].
fn row_columns(row: &rusqlite::Row<'_>) -> rusqlite::Result<RowColumns> {
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
}

/// Rebuild a [`DpnsNameStateEntry`] from one [`ROW_PROJECTION`] row.
///
/// Every signed column crosses back to `u64` through [`i64_to_u64`]
/// rather than `as`. Only `price` is barred from holding a negative by a
/// column `CHECK`; the four timestamp columns are unconstrained, so an
/// externally modified or corrupted `-1` would otherwise decode as
/// `u64::MAX` — a year-584-million timestamp that reads as valid
/// marketplace state. Checked, it surfaces as the typed
/// [`WalletStorageError::IntegerOverflow`] naming the offending column.
/// This matters more than it did: the decoder now backs a production
/// read ([`get_by_identity_and_label`]), not only the test-gated
/// whole-table helper. A malformed identifier blob is likewise rejected
/// rather than truncated.
fn entry_from_columns(columns: RowColumns) -> Result<DpnsNameStateEntry, WalletStorageError> {
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
    ) = columns;
    let document_id = Identifier::from_bytes(&doc_bytes)
        .map_err(|_| WalletStorageError::blob_decode("document_id is not 32 bytes"))?;
    let wallet_identity_id = Identifier::from_bytes(&identity_bytes)
        .map_err(|_| WalletStorageError::blob_decode("identity_id is not 32 bytes"))?;
    Ok(DpnsNameStateEntry {
        document_id,
        wallet_identity_id,
        label,
        normalized_label,
        normalized_parent_domain_name: normalized_parent,
        price: price
            .map(|value| i64_to_u64("dpns_name_states.price", value))
            .transpose()?,
        status: status_from_columns(&status, counterparty)?,
        created_at_ms: created_at
            .map(|value| i64_to_u64("dpns_name_states.created_at_ms", value))
            .transpose()?,
        updated_at_ms: updated_at
            .map(|value| i64_to_u64("dpns_name_states.updated_at_ms", value))
            .transpose()?,
        transferred_at_ms: transferred_at
            .map(|value| i64_to_u64("dpns_name_states.transferred_at_ms", value))
            .transpose()?,
        last_synced_at_ms: i64_to_u64("dpns_name_states.last_synced_at_ms", last_synced)?,
    })
}

/// The one DPNS marketplace row a wallet identity holds for
/// `normalized_label`, if any.
///
/// Backs `PlatformWalletPersistence::get_dpns_name_state` — the durable
/// fallback the marketplace sync pass uses to recover a departed name's
/// `document_id` when the session-scoped in-memory map is empty (i.e. on
/// the first sync pass after any process start). Without it the removal
/// delta is skipped and this very table keeps an orphaned row for a name
/// the wallet no longer holds.
///
/// Filtered on all three of `wallet_id`, `identity_id` and
/// `normalized_label`: `sold` / `transferred` rows are retained here, so
/// dropping the identity predicate could return a row that belongs to a
/// different identity and remove the wrong document.
///
/// **Which of several matches wins.** The triple is NOT unique. The
/// schema's primary key is `(wallet_id, document_id)`; a DPNS name can be
/// deleted and re-registered under a fresh document id, and this table
/// deliberately retains the earlier `sold` / `transferred` row. One
/// identity can therefore hold a historical row AND the current row for
/// the same normalized label. The `ORDER BY` picks the CURRENT one
/// deterministically — `owned` ahead of any retained historical status,
/// then the most recently synced row, then the highest document id as a
/// final tie-break — and matches the preference
/// `PlatformWalletPersistence::get_dpns_name_state` documents for every
/// backend. An unordered `LIMIT 1` instead follows primary-key scan
/// order, which is document-id order and carries no relation to
/// recency: it can hand back the historical row, whose removal delta
/// then deletes that row and drops the identity's label, leaving the
/// CURRENT row orphaned with nothing left to ever trigger its removal.
///
/// **Query cost.** No dedicated index exists for this predicate; SQLite
/// serves it from the `(wallet_id, document_id)` primary-key index,
/// scanning only the rows of this one wallet — bounded by the wallet's
/// DPNS name count, and hit at most once per departed name per sync pass.
/// The `ORDER BY` sorts only the rows that already satisfied the
/// three-way filter (normally one), so it adds no scan.
pub fn get_by_identity_and_label(
    conn: &Connection,
    wallet_id: &WalletId,
    wallet_identity_id: &Identifier,
    normalized_label: &str,
) -> Result<Option<DpnsNameStateEntry>, WalletStorageError> {
    let sql = format!(
        "SELECT {ROW_PROJECTION} FROM dpns_name_states \
         WHERE wallet_id = ?1 AND identity_id = ?2 AND normalized_label = ?3 \
         ORDER BY CASE status WHEN 'owned' THEN 0 ELSE 1 END, \
                  last_synced_at_ms DESC, document_id DESC \
         LIMIT 1"
    );
    let columns = conn
        .prepare_cached(&sql)?
        .query_row(
            params![
                wallet_id.as_slice(),
                wallet_identity_id.as_slice(),
                normalized_label
            ],
            row_columns,
        )
        .optional()?;
    columns.map(entry_from_columns).transpose()
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
    let sql = format!("SELECT {ROW_PROJECTION} FROM dpns_name_states WHERE wallet_id = ?1");
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], row_columns)?;
    let mut out = BTreeMap::new();
    for row in rows {
        let entry = entry_from_columns(row?)?;
        out.insert(entry.document_id, entry);
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
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
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
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
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

    /// The durable fallback behind `get_dpns_name_state`, exercised
    /// against real SQL rather than a test double.
    ///
    /// Covers the three-way filter the doc comment promises: a match is
    /// found by NORMALIZED label, and neither another identity's row nor
    /// another wallet's row can satisfy the lookup. A reader that dropped
    /// the identity predicate would remove the wrong document on
    /// departure; one that dropped the wallet predicate would cross
    /// wallets.
    #[test]
    fn get_by_identity_and_label_is_scoped_to_wallet_identity_and_normalized_label() {
        let wallet_id: WalletId = [0x44; 32];
        let other_wallet_id: WalletId = [0x55; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        for w in [&wallet_id, &other_wallet_id] {
            conn.execute(
                "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
                params![&w[..]],
            )
            .unwrap();
        }

        // Ours, plus a same-label row owned by a DIFFERENT identity in the
        // same wallet, plus a same-label row in a DIFFERENT wallet.
        let ours = entry(0, DpnsNameSaleStatus::Owned, Some(5_000));
        let mut other_identity = entry(1, DpnsNameSaleStatus::Owned, None);
        other_identity.wallet_identity_id = Identifier::from([0xCC; 32]);
        other_identity.label = ours.label.clone();
        other_identity.normalized_label = ours.normalized_label.clone();
        let mut other_wallet = entry(2, DpnsNameSaleStatus::Owned, None);
        other_wallet.label = ours.label.clone();
        other_wallet.normalized_label = ours.normalized_label.clone();

        let mut cs = DpnsNameStateChangeSet::default();
        cs.names.insert(ours.document_id, ours.clone());
        cs.names
            .insert(other_identity.document_id, other_identity.clone());
        let mut cs_other = DpnsNameStateChangeSet::default();
        cs_other
            .names
            .insert(other_wallet.document_id, other_wallet.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs).unwrap();
            apply(&tx, &other_wallet_id, &cs_other).unwrap();
            tx.commit().unwrap();
        }

        // Exact round-trip of every field, by normalized label.
        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &wallet_id,
                &ours.wallet_identity_id,
                &ours.normalized_label,
            )
            .unwrap(),
            Some(ours.clone()),
        );

        // The same label under another identity resolves to THAT row, not ours.
        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &wallet_id,
                &other_identity.wallet_identity_id,
                &ours.normalized_label,
            )
            .unwrap()
            .map(|e| e.document_id),
            Some(other_identity.document_id),
        );

        // Wallet scoping discriminates: the SAME identity and label in
        // another wallet resolves to that wallet's row, never ours. (A
        // reader missing the wallet predicate would return whichever row
        // the PK index reached first.)
        assert_ne!(other_wallet.document_id, ours.document_id);
        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &other_wallet_id,
                &ours.wallet_identity_id,
                &ours.normalized_label,
            )
            .unwrap()
            .map(|e| e.document_id),
            Some(other_wallet.document_id),
        );

        // A wallet holding no such row at all answers None.
        let empty_wallet_id: WalletId = [0x77; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&empty_wallet_id[..]],
        )
        .unwrap();
        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &empty_wallet_id,
                &ours.wallet_identity_id,
                &ours.normalized_label,
            )
            .unwrap(),
            None,
        );

        // Display label is not the key — only the normalized form is.
        assert_eq!(
            get_by_identity_and_label(&conn, &wallet_id, &ours.wallet_identity_id, &ours.label)
                .unwrap(),
            None,
        );

        // Unknown label.
        assert_eq!(
            get_by_identity_and_label(&conn, &wallet_id, &ours.wallet_identity_id, "nope").unwrap(),
            None,
        );
    }

    /// THE ROUND-3 REGRESSION. `(wallet_id, identity_id,
    /// normalized_label)` is not unique — a name can be deleted and
    /// re-registered under a fresh document id while this table retains
    /// the earlier `sold` row — so one identity can hold BOTH a
    /// historical row and the current one for the same label. An
    /// unordered `LIMIT 1` follows primary-key (document-id) scan order,
    /// which is unrelated to recency; when it hands back the historical
    /// row the caller removes THAT row plus the identity's label, and the
    /// current row is orphaned with nothing left to trigger its removal.
    ///
    /// The historical row is given the LOWER document id here precisely
    /// so scan order favours it: the raw unordered query is asserted
    /// first, so this test cannot pass vacuously.
    #[test]
    fn get_by_identity_and_label_prefers_the_current_owned_row_over_a_retained_one() {
        let wallet_id: WalletId = [0x88; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        // Same identity, same normalized label, two documents.
        let mut historical = entry(0x01, DpnsNameSaleStatus::Owned, None);
        historical.status = DpnsNameSaleStatus::Sold {
            to: Identifier::from([0xBB; 32]),
        };
        historical.transferred_at_ms = Some(1_500_000_000_000);
        historical.last_synced_at_ms = 1_500_000_000_000;
        let mut current = entry(0x02, DpnsNameSaleStatus::Owned, Some(9_000));
        current.label = historical.label.clone();
        current.normalized_label = historical.normalized_label.clone();
        current.last_synced_at_ms = 1_900_000_000_000;
        assert!(
            historical.document_id < current.document_id,
            "fixture: the historical row must sort FIRST in primary-key order"
        );

        let mut cs = DpnsNameStateChangeSet::default();
        cs.names.insert(historical.document_id, historical.clone());
        cs.names.insert(current.document_id, current.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs).unwrap();
            tx.commit().unwrap();
        }

        // Precondition: the pre-fix query really does pick the wrong row.
        let unordered: Vec<u8> = conn
            .query_row(
                "SELECT document_id FROM dpns_name_states \
                 WHERE wallet_id = ?1 AND identity_id = ?2 AND normalized_label = ?3 LIMIT 1",
                params![
                    &wallet_id[..],
                    current.wallet_identity_id.as_slice(),
                    &current.normalized_label
                ],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            unordered,
            historical.document_id.to_vec(),
            "test precondition: an unordered LIMIT 1 selects the historical row, \
             which is the orphaning bug this ORDER BY closes"
        );

        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &wallet_id,
                &current.wallet_identity_id,
                &current.normalized_label,
            )
            .unwrap(),
            Some(current),
            "the CURRENT owned row must win over the retained historical one"
        );
    }

    /// With no `owned` row left — the name departed and was later
    /// re-acquired and sold again — recency decides. The tie-break order
    /// is `last_synced_at_ms DESC` BEFORE `document_id DESC`, so the
    /// fixture gives the fresher row the LOWER document id: a reader that
    /// ordered by document id alone would pick the stale one.
    #[test]
    fn get_by_identity_and_label_breaks_ties_on_last_synced_before_document_id() {
        let wallet_id: WalletId = [0x99; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let buyer = Identifier::from([0xBB; 32]);
        let mut fresher = entry(0x03, DpnsNameSaleStatus::Sold { to: buyer }, None);
        fresher.transferred_at_ms = Some(1_900_000_000_000);
        fresher.last_synced_at_ms = 1_900_000_000_000;
        let mut staler = entry(0x04, DpnsNameSaleStatus::Transferred { to: buyer }, None);
        staler.label = fresher.label.clone();
        staler.normalized_label = fresher.normalized_label.clone();
        staler.transferred_at_ms = Some(1_400_000_000_000);
        staler.last_synced_at_ms = 1_400_000_000_000;
        assert!(
            fresher.document_id < staler.document_id,
            "fixture: document-id order must DISAGREE with recency order"
        );

        let mut cs = DpnsNameStateChangeSet::default();
        cs.names.insert(fresher.document_id, fresher.clone());
        cs.names.insert(staler.document_id, staler.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs).unwrap();
            tx.commit().unwrap();
        }

        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &wallet_id,
                &fresher.wallet_identity_id,
                &fresher.normalized_label,
            )
            .unwrap()
            .map(|e| e.document_id),
            Some(fresher.document_id),
        );
    }

    /// The timestamp columns carry no `CHECK`, so a hand-edited or
    /// corrupted row can hold a negative. `as u64` turned `-1` into
    /// `u64::MAX`, which reads back as a perfectly valid (year
    /// 584-million) timestamp; the checked decode surfaces the typed
    /// [`WalletStorageError::IntegerOverflow`] naming the column instead.
    #[test]
    fn negative_timestamps_are_rejected_by_the_decoder() {
        use crate::sqlite::util::safe_cast::SafeCastTarget;

        let wallet_id: WalletId = [0xAB; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        // One row per timestamp column, each with exactly that column
        // negative, so the error must name the right one. The row is
        // inserted valid and then corrupted with an UPDATE: that is also
        // the only way `last_synced_at_ms` (NOT NULL) can be reached.
        let columns = [
            (0xC1u8, "created_at_ms", "dpns_name_states.created_at_ms"),
            (0xC2u8, "updated_at_ms", "dpns_name_states.updated_at_ms"),
            (
                0xC3u8,
                "transferred_at_ms",
                "dpns_name_states.transferred_at_ms",
            ),
            (
                0xC4u8,
                "last_synced_at_ms",
                "dpns_name_states.last_synced_at_ms",
            ),
        ];
        for (tag, column, field) in columns {
            let doc = [tag; 32];
            let label = format!("a11ce{tag}");
            conn.execute(
                "INSERT INTO dpns_name_states \
                    (wallet_id, document_id, identity_id, label, normalized_label, \
                     normalized_parent_domain, price, status, counterparty_id, \
                     created_at_ms, updated_at_ms, transferred_at_ms, last_synced_at_ms) \
                 VALUES (?1, ?2, ?3, 'Alice', ?4, 'dash', NULL, 'owned', NULL, 1, 1, 1, 1)",
                params![&wallet_id[..], &doc[..], &[0xAAu8; 32][..], &label],
            )
            .unwrap();
            conn.execute(
                &format!(
                    "UPDATE dpns_name_states SET {column} = -1 \
                     WHERE wallet_id = ?1 AND document_id = ?2"
                ),
                params![&wallet_id[..], &doc[..]],
            )
            .unwrap();

            let error =
                get_by_identity_and_label(&conn, &wallet_id, &Identifier::from([0xAA; 32]), &label)
                    .expect_err("a negative timestamp must not decode");
            match error {
                WalletStorageError::IntegerOverflow {
                    field: got, target, ..
                } => {
                    assert_eq!(got, field, "the error must name the offending column");
                    assert_eq!(target, SafeCastTarget::U64);
                }
                other => panic!("expected a typed IntegerOverflow, got {other:?}"),
            }
        }
    }

    /// A retained `Sold` row — the exact shape a departed name leaves
    /// behind — must still be recoverable, since that is what the
    /// post-restart departure path looks for.
    #[test]
    fn get_by_identity_and_label_recovers_a_retained_sold_row() {
        let wallet_id: WalletId = [0x66; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let mut sold = entry(7, DpnsNameSaleStatus::Owned, None);
        sold.status = DpnsNameSaleStatus::Sold {
            to: Identifier::from([0xBB; 32]),
        };
        sold.transferred_at_ms = Some(1_800_000_100_000);
        let mut cs = DpnsNameStateChangeSet::default();
        cs.names.insert(sold.document_id, sold.clone());
        {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &cs).unwrap();
            tx.commit().unwrap();
        }

        assert_eq!(
            get_by_identity_and_label(
                &conn,
                &wallet_id,
                &sold.wallet_identity_id,
                &sold.normalized_label,
            )
            .unwrap(),
            Some(sold),
        );
    }
}
