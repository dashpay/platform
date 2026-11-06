//! Historical AssetUnlock identities, confined to history projection and accounting.
//!
//! The original-record archive proves identity only: it never supplies ownership,
//! context or a resurrected record. An active canonical record is required. No
//! financial outpoint, claimant, proof or confirmation column is rewritten here.

use std::collections::BTreeSet;

use dashcore::hashes::Hash;
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{OutPoint, Txid};
use key_wallet::managed_account::transaction_record::TransactionRecord;
use platform_wallet::changeset::most_advanced_transaction_context;
use platform_wallet::wallet::platform_wallet::WalletId;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use super::{blob, core_history, core_state};
use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::LoadCtx;

pub(super) fn is_unlock(record: &TransactionRecord) -> bool {
    (record.transaction.version as i16) >= 3
        && matches!(&record.transaction.special_transaction_payload,
            Some(TransactionPayload::AssetUnlockPayloadType(payload)) if payload.base.version >= 2)
}

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

impl Family {
    /// Exceptional-path wallet scan: no schema change or unauthenticated alias
    /// table. Current blobs supply metadata; archived blobs supply only IDs.
    fn load(
        conn: &Connection,
        wallet: &WalletId,
        canonical: &TransactionRecord,
        ctx: &LoadCtx,
    ) -> Result<Self, WalletStorageError> {
        let canonical_id = canonical.transaction.txid();
        let mut family = Self {
            record: canonical.clone(),
            aliases: BTreeSet::from([canonical_id]),
            physical_aliases: Vec::new(),
        };
        // Query lengths first; no blob is materialized before its size gate.
        let mut stmt = conn.prepare_cached(
            "SELECT length(txid), txid, length(record_blob) FROM core_transactions WHERE wallet_id = ?1 AND record_blob IS NOT NULL ORDER BY txid",
        )?;
        let mut rows = stmt.query(params![wallet.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_fixed_width(row.get(0)?, 32, "core_transactions.txid")?;
            let key = Txid::from_slice(&row.get::<_, Vec<u8>>(1)?)?;
            blob::check_size(row.get(2)?)?;
            if key == canonical_id {
                continue;
            }
            // The physical reader validates the body, key and context columns;
            // public alias projection must never recursively call itself here.
            let Some(record) = core_state::get_tx_record(conn, wallet, &key, ctx)? else {
                continue;
            };
            if is_unlock(&record) && record.transaction.txid() == canonical_id {
                family.merge(wallet, &record)?;
                family.aliases.insert(key);
                family.physical_aliases.push(key);
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
            if is_unlock(&record) && record.transaction.txid() == canonical_id {
                family.aliases.insert(key);
            }
        }
        if !family.physical_aliases.is_empty() {
            // A canonical row already present before adoption is also active
            // evidence. With no legacy sibling, ordinary upsert policy applies.
            if let Some(previous) = core_state::get_tx_record(conn, wallet, &canonical_id, ctx)? {
                family.merge(wallet, &previous)?;
            }
            core_history::recompute_accounting(wallet, &mut family.record)?;
        }
        Ok(family)
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
) -> Result<TransactionRecord, WalletStorageError> {
    let canonical = core_history::preserve_known_details(tx, wallet, incoming)?;
    if !is_unlock(&canonical) {
        return Ok(canonical);
    }
    let family = Family::load(tx, wallet, &canonical, &LoadCtx::strict())?;
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
    Ok(family.record)
}

/// Public history lookup. An archive entry alone can never resurrect history.
pub(crate) fn get_record(
    conn: &Connection,
    wallet: &WalletId,
    key: &Txid,
    ctx: &LoadCtx,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    let physical = core_state::get_tx_record(conn, wallet, key, ctx)?;
    let evidence = match &physical {
        Some(record) => Some(record.clone()),
        None => archived(conn, wallet, key)?,
    };
    let Some(evidence) = evidence else {
        return Ok(physical);
    };
    if !is_unlock(&evidence) {
        return Ok(physical);
    }
    let canonical_id = evidence.transaction.txid();
    let Some(canonical) = core_state::get_tx_record(conn, wallet, &canonical_id, ctx)? else {
        return Ok(physical);
    };
    Family::load(conn, wallet, &canonical, ctx).map(|family| Some(family.record))
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
    for record in canonical {
        let family = Family::load(conn, wallet, &record, &recheck)?;
        records.retain(|record| {
            record.txid == family.record.txid || !family.aliases.contains(&record.txid)
        });
        if let Some(slot) = records
            .iter_mut()
            .find(|record| record.txid == family.record.txid)
        {
            *slot = family.record;
        }
    }
    Ok(())
}

/// Equivalent accounting lookup keys, never rewritten financial outpoints.
pub(super) fn outpoints(
    conn: &Connection,
    wallet: &WalletId,
    outpoint: &OutPoint,
) -> Result<Vec<OutPoint>, WalletStorageError> {
    let Some(record) = get_record(conn, wallet, &outpoint.txid, &LoadCtx::strict())? else {
        return Ok(vec![*outpoint]);
    };
    if !is_unlock(&record) || record.txid != record.transaction.txid() {
        return Ok(vec![*outpoint]);
    }
    let family = Family::load(conn, wallet, &record, &LoadCtx::strict())?;
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
    use super::*;
    use crate::sqlite::migrations;
    use crate::LoadPolicy;

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
