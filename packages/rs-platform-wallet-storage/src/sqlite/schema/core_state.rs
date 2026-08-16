//! Writers + readers for the `core_*` tables.

#[cfg(any(test, feature = "__test-helpers"))]
use std::collections::BTreeMap;
use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::Utxo;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::schema::blob;

/// Apply a `CoreChangeSet` inside a transaction.
pub fn apply(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    cs: &CoreChangeSet,
) -> Result<(), WalletStorageError> {
    if !cs.records.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO core_transactions \
                (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET \
                height = excluded.height, \
                block_hash = excluded.block_hash, \
                block_time = excluded.block_time, \
                finalized = excluded.finalized, \
                record_blob = excluded.record_blob",
        )?;
        for record in &cs.records {
            let block_info = record.block_info();
            let height = block_info.map(|b| i64::from(b.height()));
            let block_hash = block_info.map(|b| AsRef::<[u8]>::as_ref(&b.block_hash()).to_vec());
            let block_time = block_info.map(|b| i64::from(b.timestamp()));
            let finalized = block_info.is_some();
            let payload = blob::encode(record)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                AsRef::<[u8]>::as_ref(&record.txid),
                height,
                block_hash,
                block_time,
                finalized,
                payload,
            ])?;
        }
    }
    // Derived addresses are written BEFORE UTXOs (within the same
    // transaction) so the UTXO writer's address→account_index lookup
    // sees the freshly recorded rows.
    if !cs.addresses_derived.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO core_derived_addresses \
                (wallet_id, account_type, account_index, address, derivation_path, used) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(wallet_id, account_type, address) DO UPDATE SET \
                account_index = excluded.account_index, \
                derivation_path = excluded.derivation_path",
        )?;
        for da in &cs.addresses_derived {
            let account_type =
                crate::sqlite::schema::accounts::account_type_db_label(&da.account_type);
            let account_index = crate::sqlite::schema::accounts::account_index(&da.account_type);
            let pool_type = crate::sqlite::schema::accounts::pool_type_db_label(&da.pool_type);
            let address = da.address.to_string();
            let path = format!("{}/{}", pool_type, da.derivation_index);
            stmt.execute(params![
                wallet_id.as_slice(),
                account_type,
                i64::from(account_index),
                address,
                path,
                false
            ])?;
        }
    }
    if !cs.new_utxos.is_empty() {
        let mut stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        let mut lookup_stmt = tx.prepare_cached(ACCOUNT_INDEX_BY_ADDRESS_SQL)?;
        for utxo in &cs.new_utxos {
            execute_upsert_utxo(&mut stmt, &mut lookup_stmt, wallet_id, utxo, false)?;
        }
    }
    if !cs.spent_utxos.is_empty() {
        let mut exists_stmt =
            tx.prepare_cached("SELECT 1 FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2")?;
        let mut mark_spent_stmt = tx.prepare_cached(
            "UPDATE core_utxos SET spent = 1 WHERE wallet_id = ?1 AND outpoint = ?2",
        )?;
        let mut upsert_stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        let mut lookup_stmt = tx.prepare_cached(ACCOUNT_INDEX_BY_ADDRESS_SQL)?;
        for utxo in &cs.spent_utxos {
            let op = blob::encode_outpoint(&utxo.outpoint)?;
            let exists: bool = exists_stmt
                .query_row(params![wallet_id.as_slice(), &op[..]], |_| Ok(true))
                .optional()?
                .unwrap_or(false);
            if exists {
                mark_spent_stmt.execute(params![wallet_id.as_slice(), &op[..]])?;
            } else {
                // Spent-only synthetic row: best-effort account_index
                // from the derived-address map. A spend of an
                // externally-funded address we never derived defaults
                // to 0 (logged) — harmless, since spent rows are
                // excluded from `list_unspent_utxos`.
                execute_upsert_utxo(&mut upsert_stmt, &mut lookup_stmt, wallet_id, utxo, true)?;
            }
        }
    }
    if !cs.instant_locks_for_non_final_records.is_empty() {
        let mut stmt = tx.prepare_cached(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) \
             VALUES (?1, ?2, ?3) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET islock_blob = excluded.islock_blob",
        )?;
        for (txid, islock) in &cs.instant_locks_for_non_final_records {
            let payload = blob::encode(islock)?;
            stmt.execute(params![
                wallet_id.as_slice(),
                AsRef::<[u8]>::as_ref(txid),
                payload
            ])?;
        }
    }
    if cs.last_processed_height.is_some() || cs.synced_height.is_some() {
        upsert_sync_state(tx, wallet_id, cs.last_processed_height, cs.synced_height)?;
    }
    // Sweeps run last so a winner arriving in this very changeset has its
    // own rows committed before the removal below touches the coins it took,
    // and batch by batch in order: each sweep is only true of the wallet it
    // saw, so a later one keeping a coin spent has to be able to correct an
    // earlier one that freed it.
    for batch in &cs.sweeps {
        // The released set describes the wallet when this sweep was emitted,
        // and a round can fold in a later transaction that legitimately spent
        // one of the freed coins. `apply_sweep` below is what attributes a
        // held input to `superseded_by` via `spent_in_txid`, and that only
        // happens once it runs — so at this point in the round the table
        // cannot yet tell a live claim in *this* round from the one the sweep
        // is about to displace. The changeset carries the answer instead: any
        // record in this round that is not swept by *any* batch and spends a
        // released outpoint is that live claim, and the coin stays spent.
        let swept_txids: HashSet<dashcore::Txid> = cs
            .sweeps
            .iter()
            .flat_map(|b| b.txids.iter())
            .copied()
            .collect();
        let claimed_by_survivors: HashSet<dashcore::OutPoint> = cs
            .records
            .iter()
            .filter(|record| !swept_txids.contains(&record.txid))
            .flat_map(|record| record.transaction.input.iter())
            .map(|input| input.previous_output)
            .collect();
        let released: HashSet<dashcore::OutPoint> = batch
            .released_outpoints
            .iter()
            .filter(|outpoint| !claimed_by_survivors.contains(outpoint))
            .copied()
            .collect();
        for loser_txid in &batch.txids {
            apply_sweep(tx, wallet_id, loser_txid, &batch.superseded_by, &released)?;
        }
    }
    Ok(())
}

/// Delete a swept transaction's row and outputs, then resolve the coins it
/// claimed to spend.
///
/// A swept transaction was a recorded spend that a later, final transaction
/// provably beat to one of its inputs, so it can never confirm — the wallet
/// has already dropped it. Leaving the mirrored row in place would hand it
/// back at the next `load()` and replay a balance the wallet has already
/// corrected. It would also leave an InstantSend loser answerable through
/// `get_core_tx_record`, which sent-payment reconciliation reads as final and
/// would use to advance a dead DashPay payment to `Confirmed`.
///
/// Deleting the row and the UTXOs it created is the easy half. The coins it
/// claimed to *spend* split in two, and `released` — computed upstream and
/// carried on the changeset — is the authority on which is which: an input
/// named there came free, because no surviving transaction spends it too;
/// every other input the loser claimed was taken by the transaction that beat
/// it and is gone for good.
///
/// Recomputing that split here is not an option even though this schema
/// stores whole records. The transaction that took the rest need not be
/// wallet-relevant at all — it can spend our coin while paying only external
/// addresses, and then it is never recorded anywhere in this store — and even
/// a relevant one is not guaranteed to arrive in the same round as the sweep.
///
/// A held input can also have no `core_utxos` row at all: this wallet can
/// persist the loser before its own funding output was ever classified as
/// ours, so the outpoint the loser claims to spend has nothing to update.
/// Losing that claim would matter — the funding transaction has not shown up
/// yet, and when it eventually does, the ordinary UTXO upsert would treat the
/// outpoint as freshly unspent — so a held-but-absent input gets a row of its
/// own here: `spent = 1`, `spent_in_txid = superseded_by`, everything else a
/// placeholder the real funding data overwrites on arrival.
/// `execute_upsert_utxo`'s conflict clause is what makes that placeholder
/// durable — it refuses to clear `spent` while `spent_in_txid` is set, so the
/// claim survives the funding upsert instead of being upserted away by it.
///
/// Idempotent: a txid this store never recorded is a successful no-op, not an
/// error. A sweep can legitimately name a transaction this wallet dropped, or
/// never derived an address for in the first place.
fn apply_sweep(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    loser_txid: &dashcore::Txid,
    superseded_by: &dashcore::Txid,
    released: &HashSet<dashcore::OutPoint>,
) -> Result<(), WalletStorageError> {
    let loser_blob: Option<Vec<u8>> = tx
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
            |row| row.get(0),
        )
        .optional()?;
    let Some(loser_blob) = loser_blob else {
        return Ok(());
    };
    let loser: TransactionRecord = blob::decode(&loser_blob)?;

    tx.execute(
        "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
    )?;
    // An InstantSend-locked loser is evictable by a chainlocked winner, so a
    // swept transaction can own a row here. Nothing ties that table to
    // `core_transactions` — no foreign key, no trigger — so the lock would
    // outlive the transaction it describes forever.
    tx.execute(
        "DELETE FROM core_instant_locks WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
    )?;
    let mut delete_output_stmt =
        tx.prepare_cached("DELETE FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2")?;
    for vout in 0..loser.transaction.output.len() as u32 {
        let op = blob::encode_outpoint(&dashcore::OutPoint {
            txid: *loser_txid,
            vout,
        })?;
        delete_output_stmt.execute(params![wallet_id.as_slice(), &op[..]])?;
    }
    drop(delete_output_stmt);

    // Each input is set outright rather than only touched when it changes:
    // whichever way it went, the row must end this round agreeing with the
    // wallet, and a coin the sweep did not free stays out of the unspent
    // query even if nothing had marked it spent yet (upstream sweeps only
    // unconfirmed records, whose spends this schema does not mark).
    // `spent_in_txid` moves with `spent`: a released input clears back to
    // NULL (nobody's claim), a held one is attributed to `superseded_by` so
    // the claim outlives this row's own deletion below.
    let mut spend_stmt = tx.prepare_cached(
        "UPDATE core_utxos SET spent = ?3, spent_in_txid = ?4 \
         WHERE wallet_id = ?1 AND outpoint = ?2",
    )?;
    // Only reached for a held input with no existing row — see the
    // doc comment above. `value`/`script`/`height`/`account_index` are
    // placeholders; the funding UTXO's own upsert overwrites them (and,
    // thanks to the `spent_in_txid` guard in `execute_upsert_utxo`, does
    // not clear `spent` while doing it).
    let mut tombstone_stmt = tx.prepare_cached(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
         VALUES (?1, ?2, 0, X'', NULL, 0, 1, ?3)",
    )?;
    for input in &loser.transaction.input {
        let outpoint = input.previous_output;
        let key = blob::encode_outpoint(&outpoint)?;
        let freed = released.contains(&outpoint);
        let spent_in_txid: Option<&[u8]> = if freed {
            None
        } else {
            Some(AsRef::<[u8]>::as_ref(superseded_by))
        };
        let affected = spend_stmt.execute(params![
            wallet_id.as_slice(),
            &key[..],
            !freed,
            spent_in_txid
        ])?;
        if affected == 0 && !freed {
            tombstone_stmt.execute(params![
                wallet_id.as_slice(),
                &key[..],
                AsRef::<[u8]>::as_ref(superseded_by)
            ])?;
        }
    }

    Ok(())
}

/// Resolve the owning account index for a UTXO by its rendered address,
/// joining against the `core_derived_addresses` map written earlier in
/// the same transaction.
const ACCOUNT_INDEX_BY_ADDRESS_SQL: &str =
    "SELECT account_index FROM core_derived_addresses WHERE wallet_id = ?1 AND address = ?2";

// `spent` only takes the incoming value when the existing row has no
// `spent_in_txid`. A coin held spent with no spender on record is the
// documented recovery state — the wallet handing it back as a UTXO is
// what clears it. A coin held spent *with* `spent_in_txid` set is
// `apply_sweep`'s tombstone for an input the loser claimed but the funding
// row hadn't arrived for yet; the funding upsert (this statement) is
// exactly the arrival that tombstone exists to survive, so it must not
// double as the thing that erases it. `spent_in_txid` itself is left out of
// the SET list entirely — untouched, it carries the claim forward.
const UPSERT_UTXO_SQL: &str = "INSERT INTO core_utxos \
        (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL) \
     ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
        value = excluded.value, \
        script = excluded.script, \
        height = excluded.height, \
        account_index = excluded.account_index, \
        spent = CASE WHEN core_utxos.spent_in_txid IS NOT NULL \
            THEN core_utxos.spent ELSE excluded.spent END";

fn execute_upsert_utxo(
    stmt: &mut rusqlite::CachedStatement<'_>,
    lookup_stmt: &mut rusqlite::CachedStatement<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
    spent: bool,
) -> Result<(), WalletStorageError> {
    let op = blob::encode_outpoint(&utxo.outpoint)?;
    let address = utxo.address.to_string();
    // `Utxo` carries no account index; recover it from the
    // derived-address map written earlier in this transaction.
    let looked_up: Option<i64> = lookup_stmt
        .query_row(params![wallet_id.as_slice(), &address], |row| row.get(0))
        .optional()?;
    let account_index: i64 = match looked_up {
        Some(idx) => idx,
        // An unspent UTXO whose address we never derived would land in
        // the wallet's funds under account 0 and never re-derive — silent
        // mis-bucketing of live money. Refuse it. The spent-only
        // placeholder path tolerates the fallback because spent rows are
        // excluded from `list_unspent_utxos`, so a wrong index there is
        // inert.
        None if !spent => {
            return Err(WalletStorageError::UtxoAddressNotDerived {
                address: address.clone(),
            });
        }
        None => {
            tracing::debug!(
                wallet_id = %hex::encode(wallet_id),
                address = %address,
                "spent-only UTXO address not found in core_derived_addresses; using account_index 0 placeholder"
            );
            0
        }
    };
    stmt.execute(params![
        wallet_id.as_slice(),
        &op[..],
        crate::sqlite::util::safe_cast::u64_to_i64("core_utxos.value", utxo.value())?,
        utxo.txout.script_pubkey.as_bytes(),
        i64::from(utxo.height),
        account_index,
        spent,
    ])?;
    Ok(())
}

fn upsert_sync_state(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    last_processed: Option<u32>,
    synced: Option<u32>,
) -> Result<(), WalletStorageError> {
    // Monotonic-max semantics — keep the larger of (current, new).
    let current_raw: (Option<i64>, Option<i64>) = tx
        .query_row(
            "SELECT last_processed_height, synced_height FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .unwrap_or((None, None));
    let current = (
        sync_height_u32("core_sync_state.last_processed_height", current_raw.0)?,
        sync_height_u32("core_sync_state.synced_height", current_raw.1)?,
    );
    let lp = match (current.0, last_processed) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let sy = match (current.1, synced) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    tx.execute(
        "INSERT INTO core_sync_state (wallet_id, last_processed_height, synced_height) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            last_processed_height = excluded.last_processed_height, \
            synced_height = excluded.synced_height",
        params![wallet_id.as_slice(), lp.map(i64::from), sy.map(i64::from),],
    )?;
    Ok(())
}

/// Convert a stored sync-height column to `u32`, erroring on overflow
/// rather than silently truncating a corrupt/out-of-range value.
fn sync_height_u32(
    field: &'static str,
    value: Option<i64>,
) -> Result<Option<u32>, WalletStorageError> {
    match value {
        None => Ok(None),
        Some(v) => Ok(Some(u32::try_from(v).map_err(|_| {
            WalletStorageError::IntegerOverflow {
                field,
                value: v as u64,
                target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
            }
        })?)),
    }
}

/// Fetch a single transaction record by txid. Returns `Ok(None)` if
/// absent.
pub fn get_tx_record(
    conn: &Connection,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    let row: Option<Vec<u8>> = conn
        .query_row(
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid)],
            |row| row.get(0),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some(payload) => Ok(Some(blob::decode(&payload)?)),
    }
}

/// Row representing one unspent UTXO. Used by tests that probe the
/// `core_utxos` table without going through full `Wallet` reconstruction.
#[cfg(any(test, feature = "__test-helpers"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnspentRow {
    pub outpoint: dashcore::OutPoint,
    pub value: u64,
    pub script: Vec<u8>,
    pub height: Option<u32>,
    pub account_index: u32,
}

/// All UTXOs for a wallet that have not been spent yet, bucketed by
/// account index. Retained for this crate's integration tests.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn list_unspent_utxos(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, Vec<UnspentRow>>, WalletStorageError> {
    let mut stmt = conn.prepare(
        "SELECT outpoint, value, script, height, account_index \
         FROM core_utxos WHERE wallet_id = ?1 AND spent = 0",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let value: i64 = row.get(1)?;
        let script: Vec<u8> = row.get(2)?;
        let height: Option<i64> = row.get(3)?;
        let account_index: i64 = row.get(4)?;
        Ok((op_bytes, value, script, height, account_index))
    })?;
    let mut by_account: BTreeMap<u32, Vec<UnspentRow>> = BTreeMap::new();
    for r in rows {
        let (op_bytes, value, script_bytes, height, account_index) = r?;
        let outpoint = blob::decode_outpoint(&op_bytes)?;
        let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
        let height = match height {
            None => None,
            Some(h) => Some(
                u32::try_from(h).map_err(|_| WalletStorageError::IntegerOverflow {
                    field: "core_utxos.height",
                    value: h as u64,
                    target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
                })?,
            ),
        };
        let account_index =
            u32::try_from(account_index).map_err(|_| WalletStorageError::IntegerOverflow {
                field: "core_utxos.account_index",
                value: account_index as u64,
                target: crate::sqlite::util::safe_cast::SafeCastTarget::U64,
            })?;
        let row = UnspentRow {
            outpoint,
            value,
            script: script_bytes,
            height,
            account_index,
        };
        by_account.entry(account_index).or_default().push(row);
    }
    Ok(by_account)
}
