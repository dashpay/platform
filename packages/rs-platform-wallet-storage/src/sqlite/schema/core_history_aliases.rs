//! Historical AssetUnlock identities, confined to history projection and accounting.
//!
//! The original-record archive proves identity only: it never supplies ownership,
//! context or a resurrected record. An active canonical record is required. No
//! financial outpoint, claimant, proof or confirmation column is rewritten here.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::{OutPoint, Txid};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use platform_wallet::changeset::most_advanced_transaction_context;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::{blob, core_history, core_state};
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::LoadCtx;

pub(super) use super::core_state::is_historical_unlock as is_unlock;

fn validate(key: &Txid, payload: &[u8]) -> Result<TransactionRecord, WalletStorageError> {
    let record = core_state::decode_transaction_record(payload)?;
    if record.txid != *key {
        return Err(WalletStorageError::blob_decode(
            "historical identity evidence disagrees with its physical key",
        ));
    }
    Ok(record)
}

fn archived(
    conn: &Connection,
    wallet: &WalletId,
    key: &Txid,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    let len: Option<i64> = conn.query_row(
        "SELECT length(record_blob) FROM core_transaction_record_originals WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet.as_slice(), key.as_byte_array().as_slice()], |row| row.get(0),
    ).optional()?;
    let Some(len) = len else { return Ok(None) };
    blob::check_size(len)?;
    let bytes: Vec<u8> = conn.query_row(
        "SELECT record_blob FROM core_transaction_record_originals WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet.as_slice(), key.as_byte_array().as_slice()], |row| row.get(0),
    )?;
    validate(key, &bytes).map(Some)
}

struct Family {
    record: TransactionRecord,
    aliases: BTreeSet<Txid>,
    physical_aliases: Vec<Txid>,
}

#[derive(Default)]
struct Members {
    aliases: BTreeSet<Txid>,
    physical: Vec<TransactionRecord>,
}

/// Validated identity evidence for one wallet snapshot, shared by its families.
#[derive(Default)]
pub(super) struct AliasIndex(BTreeMap<Txid, Members>);

impl AliasIndex {
    fn load(
        conn: &Connection,
        wallet: &WalletId,
        ctx: &LoadCtx,
        skip: Option<Txid>,
    ) -> Result<Self, WalletStorageError> {
        let mut index = Self::default();
        // Query lengths first; no blob is materialized before its size gate.
        let mut stmt = conn.prepare_cached(
            "SELECT length(txid), txid, length(record_blob) FROM core_transactions WHERE wallet_id = ?1 AND record_blob IS NOT NULL ORDER BY txid",
        )?;
        let mut rows = stmt.query(params![wallet.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_fixed_width(row.get(0)?, 32, "core_transactions.txid")?;
            let key = Txid::from_slice(&row.get::<_, Vec<u8>>(1)?)?;
            blob::check_size(row.get(2)?)?;
            if Some(key) == skip {
                continue;
            }
            // Keep physical validation separate from public alias projection.
            let Some(record) = core_state::get_tx_record(conn, wallet, &key, ctx)? else {
                continue;
            };
            let canonical_id = record.transaction.txid();
            if is_unlock(&record) && key != canonical_id {
                let members = index.0.entry(canonical_id).or_default();
                members.aliases.insert(key);
                members.physical.push(record);
            }
        }
        let mut stmt = conn.prepare_cached(
            "SELECT length(txid), txid FROM core_transaction_record_originals WHERE wallet_id = ?1 ORDER BY txid",
        )?;
        let mut rows = stmt.query(params![wallet.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_fixed_width(row.get(0)?, 32, "core_transaction_record_originals.txid")?;
            let key = Txid::from_slice(&row.get::<_, Vec<u8>>(1)?)?;
            let Some(record) = archived(conn, wallet, &key)? else {
                continue;
            };
            let canonical_id = record.transaction.txid();
            if is_unlock(&record) && key != canonical_id {
                index.0.entry(canonical_id).or_default().aliases.insert(key);
            }
        }
        Ok(index)
    }

    fn family(
        &self,
        conn: &Connection,
        wallet: &WalletId,
        canonical: &TransactionRecord,
        ctx: &LoadCtx,
    ) -> Result<Family, WalletStorageError> {
        let canonical_id = canonical.transaction.txid();
        let mut family = Family::single(canonical.clone());
        let Some(members) = self.0.get(&canonical_id) else {
            return Ok(family);
        };
        family.aliases.extend(&members.aliases);
        for record in &members.physical {
            family.merge(wallet, record)?;
            family.physical_aliases.push(record.txid);
        }
        if !family.physical_aliases.is_empty() {
            // Archive-only aliases never promote context or ownership.
            if let Some(previous) = core_state::get_tx_record(conn, wallet, &canonical_id, ctx)? {
                family.merge(wallet, &previous)?;
            }
            core_history::recompute_accounting(wallet, &mut family.record)?;
        }
        Ok(family)
    }
}

impl Family {
    fn single(record: TransactionRecord) -> Self {
        Self {
            aliases: BTreeSet::from([record.txid]),
            record,
            physical_aliases: Vec::new(),
        }
    }

    fn load(
        conn: &Connection,
        wallet: &WalletId,
        canonical: &TransactionRecord,
        ctx: &LoadCtx,
    ) -> Result<Self, WalletStorageError> {
        AliasIndex::load(conn, wallet, ctx, Some(canonical.transaction.txid()))?
            .family(conn, wallet, canonical, ctx)
    }

    fn merge(
        &mut self,
        wallet: &WalletId,
        previous: &TransactionRecord,
    ) -> Result<(), WalletStorageError> {
        if let (Some(old), Some(new)) = (previous.block_info(), self.record.block_info()) {
            if old.height() != new.height()
                || old.block_hash() != new.block_hash()
                || old.timestamp() != new.timestamp()
                || matches!((old.position(), new.position()), (Some(a), Some(b)) if a != b)
            {
                return Err(WalletStorageError::HistoryAliasContextConflict {
                    wallet_id: *wallet,
                    txid: self.record.txid,
                });
            }
        }
        let context =
            most_advanced_transaction_context(&self.record.context, &previous.context).clone();
        let body = self.record.transaction.clone();
        self.record = core_history::merge_known_details(wallet, previous, &self.record)?;
        self.record.transaction = body;
        self.record.context = context;
        if self.record.fee.is_none() {
            self.record.fee = previous.fee;
        }
        if self.record.label.is_empty() {
            self.record.label.clone_from(&previous.label);
        }
        Ok(())
    }
}

/// Adopt only validated current writes. Legacy-only reads remain physically keyed.
pub(super) fn adopt(
    tx: &Transaction<'_>,
    wallet: &WalletId,
    incoming: &TransactionRecord,
    index: &mut Option<AliasIndex>,
) -> Result<TransactionRecord, WalletStorageError> {
    let canonical = core_history::preserve_known_details(tx, wallet, incoming)?;
    if !is_unlock(&canonical) {
        return Ok(canonical);
    }
    let ctx = LoadCtx::strict();
    let index = match index {
        Some(index) => index,
        slot @ None => slot.insert(AliasIndex::load(tx, wallet, &ctx, Some(canonical.txid))?),
    };
    let family = index.family(tx, wallet, &canonical, &ctx)?;
    for alias in &family.physical_aliases {
        // Append before clearing. Existing archive entries were validated above;
        // the first original stays byte-for-byte immutable.
        tx.execute(
            "INSERT OR IGNORE INTO core_transaction_record_originals (wallet_id, txid, record_blob) SELECT wallet_id, txid, record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet.as_slice(), alias.as_byte_array().as_slice()],
        )?;
        // Keep the physical anchor: DELETE would clear durable claimant links
        // through V014's trigger. Height/proof/finality metadata is untouched.
        tx.execute(
            "UPDATE core_transactions SET record_blob = NULL WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet.as_slice(), alias.as_byte_array().as_slice()],
        )?;
        tx.execute(
            "DELETE FROM core_transaction_inputs WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet.as_slice(), alias.as_byte_array().as_slice()],
        )?;
    }
    if let Some(members) = index.0.get_mut(&canonical.txid) {
        members.physical.clear();
    }
    Ok(family.record)
}

/// Public history lookup. An archive entry alone can never resurrect history.
pub(crate) fn get_record(
    conn: &Connection,
    wallet: &WalletId,
    key: &Txid,
    ctx: &LoadCtx,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    Ok(get_family(conn, wallet, key, ctx)?.map(|family| family.record))
}

fn get_family(
    conn: &Connection,
    wallet: &WalletId,
    key: &Txid,
    ctx: &LoadCtx,
) -> Result<Option<Family>, WalletStorageError> {
    let physical = core_state::get_tx_record(conn, wallet, key, ctx)?;
    let evidence = match &physical {
        Some(record) => Some(record.clone()),
        None => archived(conn, wallet, key)?,
    };
    let Some(evidence) = evidence else {
        return Ok(physical.map(Family::single));
    };
    if !is_unlock(&evidence) {
        return Ok(physical.map(Family::single));
    }
    let canonical_id = evidence.transaction.txid();
    let Some(canonical) = core_state::get_tx_record(conn, wallet, &canonical_id, ctx)? else {
        return Ok(physical.map(Family::single));
    };
    Family::load(conn, wallet, &canonical, ctx).map(Some)
}

/// Deduplicate only adopted families; backend readers keep physical row identities.
pub(crate) fn project_records(
    conn: &Connection,
    wallet: &WalletId,
    records: &mut Vec<TransactionRecord>,
    ctx: &LoadCtx,
) -> Result<(), WalletStorageError> {
    let recheck = ctx.for_recheck();
    let canonical: Vec<_> = records
        .iter()
        .filter(|record| is_unlock(record) && record.txid == record.transaction.txid())
        .cloned()
        .collect();
    if canonical.is_empty() {
        return Ok(());
    }
    let index = AliasIndex::load(conn, wallet, &recheck, None)?;
    let mut projected = BTreeMap::new();
    let mut aliases = BTreeSet::new();
    for record in canonical {
        let family = index.family(conn, wallet, &record, &recheck)?;
        aliases.extend(
            family
                .aliases
                .into_iter()
                .filter(|id| *id != family.record.txid),
        );
        projected.insert(family.record.txid, family.record);
    }
    records.retain_mut(|record| {
        if let Some(projected) = projected.remove(&record.txid) {
            *record = projected;
            true
        } else {
            !aliases.contains(&record.txid)
        }
    });
    Ok(())
}

/// Equivalent accounting lookup keys, never rewritten financial outpoints.
pub(super) fn outpoints(
    conn: &Connection,
    wallet: &WalletId,
    outpoint: &OutPoint,
) -> Result<Vec<OutPoint>, WalletStorageError> {
    let Some(family) = get_family(conn, wallet, &outpoint.txid, &LoadCtx::strict())? else {
        return Ok(vec![*outpoint]);
    };
    if !is_unlock(&family.record) || family.record.txid != family.record.transaction.txid() {
        return Ok(vec![*outpoint]);
    }
    Ok(std::iter::once(*outpoint)
        .chain(
            family
                .aliases
                .into_iter()
                .filter(|id| *id != outpoint.txid)
                .map(|id| OutPoint::new(id, outpoint.vout)),
        )
        .collect())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::sqlite::migrations;
    use crate::LoadPolicy;

    thread_local! {
        static QUERY_COUNT: Cell<usize> = const { Cell::new(0) };
        static SCAN_COUNT: Cell<usize> = const { Cell::new(0) };
    }

    fn count_queries(event: rusqlite::trace::TraceEvent<'_>) {
        if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
            QUERY_COUNT.with(|count| count.set(count.get() + 1));
            if sql.starts_with("SELECT length(txid), txid, length(record_blob)") {
                SCAN_COUNT.with(|count| count.set(count.get() + 1));
            }
        }
    }

    fn populated_families(
        count: usize,
        with_aliases: bool,
    ) -> (Connection, WalletId, Vec<TransactionRecord>) {
        let wallet = [0x66; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run_for_open(&mut conn, LoadPolicy::Strict).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet.as_slice()],
        )
        .unwrap();
        let fixture: TransactionRecord = blob::decode(
            &hex::decode(
                include_str!("../../../tests/fixtures/v019_asset_unlock_v2.txt")
                    .split_whitespace()
                    .nth(2)
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        for index in 0..count {
            let mut legacy = fixture.clone();
            legacy.transaction.lock_time = index as u32;
            legacy.txid = legacy.transaction.instance_hash();
            let mut canonical = legacy.clone();
            canonical.txid = canonical.transaction.txid();
            if with_aliases {
                conn.execute("INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, ?3)",
                    params![wallet.as_slice(), legacy.txid.as_byte_array().as_slice(), blob::encode(&legacy).unwrap()]).unwrap();
                conn.execute("INSERT INTO core_transaction_record_originals (wallet_id, txid, record_blob) VALUES (?1, ?2, ?3)",
                    params![wallet.as_slice(), legacy.txid.as_byte_array().as_slice(), blob::encode(&legacy).unwrap()]).unwrap();
                canonical.output_details.clear();
                canonical.net_amount = 0;
            }
            conn.execute("INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, ?3)",
                params![wallet.as_slice(), canonical.txid.as_byte_array().as_slice(), blob::encode(&canonical).unwrap()]).unwrap();
        }
        let (state, _) = core_state::load_state(
            &conn,
            &wallet,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(count_queries),
        )
        .unwrap();
        QUERY_COUNT.with(|count| count.set(0));
        SCAN_COUNT.with(|count| count.set(0));
        (conn, wallet, state.records)
    }

    #[test]
    fn should_project_history_with_linear_query_growth() {
        let queries = |families| {
            let (conn, wallet, mut records) = populated_families(families, true);
            project_records(&conn, &wallet, &mut records, &LoadCtx::strict()).unwrap();
            assert_eq!(records.len(), families);
            assert!(records
                .iter()
                .all(|record| record.txid == record.transaction.txid()
                    && record.output_details.len() == 1
                    && record.net_amount == 150_000));
            QUERY_COUNT.with(|count| count.get())
        };
        let one = queries(1);
        let eight = queries(8);
        assert!(
            eight <= one * 8,
            "query count must grow at most linearly: {one} -> {eight}"
        );
    }

    #[test]
    fn should_scan_history_only_once_for_equivalent_outpoints() {
        let (conn, wallet, records) = populated_families(1, true);
        let canonical = records
            .iter()
            .find(|record| record.txid == record.transaction.txid())
            .unwrap();
        let equivalents = outpoints(&conn, &wallet, &OutPoint::new(canonical.txid, 0)).unwrap();
        assert_eq!(equivalents.len(), 2);
        for record in records {
            assert!(equivalents.contains(&OutPoint::new(record.txid, 0)));
        }
        assert_eq!(SCAN_COUNT.with(|count| count.get()), 1);
    }

    #[test]
    fn should_leave_alias_free_families_unchanged_without_further_queries() {
        let (conn, wallet, records) = populated_families(8, false);
        let index = AliasIndex::load(&conn, &wallet, &LoadCtx::strict(), None).unwrap();
        QUERY_COUNT.with(|count| count.set(0));
        for record in records {
            let family = index
                .family(&conn, &wallet, &record, &LoadCtx::strict())
                .unwrap();
            assert_eq!(family.aliases, BTreeSet::from([record.txid]));
            assert_eq!(
                blob::encode(&family.record).unwrap(),
                blob::encode(&record).unwrap()
            );
        }
        assert_eq!(QUERY_COUNT.with(|count| count.get()), 0);
    }

    #[test]
    fn should_share_identity_scan_across_adoptions() {
        let (mut conn, wallet, records) = populated_families(8, true);
        let mut canonical: Vec<_> = records
            .into_iter()
            .filter(|record| record.txid == record.transaction.txid())
            .collect();
        let mut repeated = canonical[0].clone();
        repeated.label = "redelivery".into();
        repeated.net_amount = 123_456;
        let repeated_id = repeated.txid;
        canonical.push(repeated);
        let tx = conn.transaction().unwrap();
        let mut index = None;
        for incoming in canonical {
            let adopted = adopt(&tx, &wallet, &incoming, &mut index).unwrap();
            // An adopted family follows ordinary canonical accounting on redelivery.
            assert_eq!(
                adopted.net_amount,
                if incoming.label == "redelivery" {
                    123_456
                } else {
                    150_000
                }
            );
            tx.execute(
                "UPDATE core_transactions SET record_blob = ?1 WHERE wallet_id = ?2 AND txid = ?3",
                params![
                    blob::encode(&adopted).unwrap(),
                    wallet.as_slice(),
                    adopted.txid.as_byte_array().as_slice()
                ],
            )
            .unwrap();
        }
        assert_eq!(SCAN_COUNT.with(|count| count.get()), 1);
        let record = core_state::get_tx_record(&tx, &wallet, &repeated_id, &LoadCtx::strict())
            .unwrap()
            .unwrap();
        assert_eq!(record.label, "redelivery");
        assert_eq!(record.output_details.len(), 1);
        assert_eq!(record.net_amount, 123_456);
        assert_eq!(
            tx.query_row::<i64, _, _>(
                "SELECT count(*) FROM core_transactions WHERE record_blob IS NULL",
                [],
                |row| row.get(0)
            )
            .unwrap(),
            8
        );
    }

    #[test]
    fn should_project_existing_alias_duplicates_without_mutating_physical_history() {
        let wallet = [0x65; 32];
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::run_for_open(&mut conn, LoadPolicy::Strict).unwrap();
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![wallet.as_slice()],
        )
        .unwrap();
        let legacy: TransactionRecord = blob::decode(
            &hex::decode(
                include_str!("../../../tests/fixtures/v019_asset_unlock_v2.txt")
                    .lines()
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .nth(2)
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let mut canonical = legacy.clone();
        canonical.txid = canonical.transaction.txid();
        canonical.output_details.clear();
        canonical.net_amount = 0;
        for record in [&legacy, &canonical] {
            conn.execute("INSERT INTO core_transactions (wallet_id, txid, finalized, record_blob) VALUES (?1, ?2, 0, ?3)", params![wallet.as_slice(), record.txid.as_byte_array().as_slice(), blob::encode(record).unwrap()]).unwrap();
        }
        let (mut raw, _) = core_state::load_state(
            &conn,
            &wallet,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(
            raw.records.len(),
            2,
            "backend readers keep exact physical identities"
        );
        project_records(&conn, &wallet, &mut raw.records, &LoadCtx::strict()).unwrap();
        assert_eq!(raw.records.len(), 1);
        assert_eq!(raw.records[0].txid, canonical.txid);
        assert_eq!(raw.records[0].output_details.len(), 1);
        assert_eq!(raw.records[0].net_amount, 150_000);
        for id in [legacy.txid, canonical.txid] {
            let projected = get_record(&conn, &wallet, &id, &LoadCtx::strict())
                .unwrap()
                .unwrap();
            assert_eq!(
                blob::encode(&projected).unwrap(),
                blob::encode(&raw.records[0]).unwrap()
            );
        }
        let (physical, _) = core_state::load_state(
            &conn,
            &wallet,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(physical.records.len(), 2);
        assert_eq!(
            core_state::get_tx_record(&conn, &wallet, &canonical.txid, &LoadCtx::strict())
                .unwrap()
                .unwrap()
                .output_details
                .len(),
            0
        );
        assert_eq!(
            conn.query_row::<i64, _, _>(
                "SELECT count(*) FROM core_transaction_record_originals",
                [],
                |row| row.get(0)
            )
            .unwrap(),
            0
        );
        conn.execute(
            "UPDATE core_transactions SET height = 999 WHERE txid = ?1",
            params![legacy.txid.as_byte_array().as_slice()],
        )
        .unwrap();
        let recovery = LoadCtx::recovery();
        let (mut raw, _) =
            core_state::load_state(&conn, &wallet, dashcore::Network::Testnet, &recovery).unwrap();
        let before = recovery.degradation().total;
        assert_eq!(before, 1);
        project_records(&conn, &wallet, &mut raw.records, &recovery).unwrap();
        assert_eq!(
            recovery.degradation().total,
            before,
            "projection must not count a previously reported physical-row drift again"
        );
    }
}
