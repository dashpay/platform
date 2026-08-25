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
    let chainlock_height = cs
        .last_applied_chain_lock
        .as_ref()
        .map(|cl| cl.block_height);
    let heights_advanced = cs.last_processed_height.is_some()
        || cs.synced_height.is_some()
        || chainlock_height.is_some();
    if heights_advanced {
        upsert_sync_state(
            tx,
            wallet_id,
            cs.last_processed_height,
            cs.synced_height,
            chainlock_height,
        )?;
    }
    // Sweeps run last so a winner arriving in this very changeset has its
    // own rows committed before the removal below touches the coins it took,
    // and batch by batch in order: each sweep is only true of the wallet it
    // saw, so a later one keeping a coin spent has to be able to correct an
    // earlier one that freed it.
    if cs.sweeps.is_empty() {
        // The ordinary round. Everything below serves the sweep loop, and
        // building the survivor set would hash every input of every record
        // for a loop that never runs — with the write transaction open.
        if heights_advanced {
            collect_finalized_tombstones(tx, wallet_id)?;
        }
        return Ok(());
    }

    // The surviving claims are a property of the whole changeset, not of any
    // one batch, so they are built once: the adapter folds up to a full drain
    // into a single store, and rebuilding them per batch would re-hash every
    // swept txid and every surviving record input once per sweep, with the
    // write transaction open the whole time.
    //
    // `apply_sweep` below is what attributes a held input to `superseded_by`
    // via `spent_in_txid`, and that only happens once it runs — so at this
    // point in the round the table cannot yet tell a live claim in *this*
    // round from the one a sweep is about to displace. The changeset carries
    // the answer instead: any record in this round that is not swept by *any*
    // batch and spends a released outpoint is that live claim, and the coin
    // stays spent.
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
    // The changeset is not the whole answer, though. Upstream computes
    // `released_outpoints` from its *live* records, and under the default
    // `keep-finalized-transactions = off` a chainlocked record is pruned to
    // its bare txid — the pinned `TransactionsSwept::released_outpoints`
    // doc records this exact limitation ("the inputs of a pruned record
    // survive nowhere else, so this cannot be resolved at this layer").
    // It CAN be resolved at this layer: this store never prunes a
    // `core_transactions` row on finalization, so the full input set of
    // every settled spend the wallet has forgotten is still on disk. A
    // release naming a coin such a record still claims is upstream
    // reporting its own amnesia — honouring it flips the materialized UTXO
    // to `spent = 0` and hands a provably consumed coin back as spendable
    // after the next load, a guaranteed double spend. The same applies
    // after a restart for records of ANY context: hydration rebuilds the
    // in-memory wallet without its transaction history, so every claim the
    // store holds is one upstream can no longer see.
    //
    // `stored_input_claims` is therefore upstream's own `retain_unclaimed`
    // predicate — "drop outpoints some surviving record still spends" —
    // re-evaluated against the unpruned history. Built lazily and at most
    // once per round: only a batch whose released set survives the
    // in-round filter above pays for it, and the common sweep (a resend
    // whose winner spends every input its loser did) releases nothing.
    let mut stored_claims: Option<HashSet<dashcore::OutPoint>> = None;
    for batch in &cs.sweeps {
        // Only this stays per batch: a release is true of the wallet its own
        // sweep saw, which is what lets a later batch correct an earlier one.
        let mut released: HashSet<dashcore::OutPoint> = batch
            .released_outpoints
            .iter()
            .filter(|outpoint| !claimed_by_survivors.contains(outpoint))
            .copied()
            .collect();
        if !released.is_empty() {
            let claims = match stored_claims.as_ref() {
                Some(claims) => claims,
                None => {
                    stored_claims =
                        Some(surviving_stored_input_claims(tx, wallet_id, &swept_txids)?);
                    stored_claims.as_ref().expect("just assigned")
                }
            };
            released.retain(|outpoint| !claims.contains(outpoint));
        }
        for loser_txid in &batch.txids {
            apply_sweep(
                tx,
                wallet_id,
                loser_txid,
                &batch.superseded_by,
                &released,
                &swept_txids,
                batch.winner_mined_height,
            )?;
        }
        // Releases are outpoint-keyed facts, so they are applied by outpoint
        // once the batch's losers are done — not only through each loser's
        // decoded inputs above. A chained-sweep claim is a `core_utxos`
        // placeholder that exists independently of any transaction row, and
        // the loser now freeing it need not have one: a fatal flush error
        // wipes a buffered round (the winner's record with it) while the
        // faulted wallet keeps persisting later rounds, and `apply_sweep`
        // above returns before its input loop when the swept txid has no
        // row. Dropping the release set there would leave the `:340` valve
        // holding the placeholder's `spent_in_txid` forever — the release
        // is the one channel that clears it. Running after the loser loop
        // rather than inside it changes nothing for inputs the loop already
        // freed (same UPDATE, idempotent), and a coin a surviving record in
        // this round re-claimed was already filtered out of `released`
        // above.
        if !released.is_empty() {
            // A released claim that never materialised is deleted outright
            // rather than flipped to `spent = 0`: the row is all placeholder
            // (`value = 0`, `script = X''`, `height` NULL — no writer but the
            // tombstone insert leaves `height` NULL), so releasing it in
            // place would surface a zero-value phantom coin through
            // `list_unspent_utxos`. No row is the correct end state — if the
            // funding output ever classifies, its ordinary upsert creates
            // the real row freshly unspent, exactly as if the dead claim had
            // never existed. Materialised rows carry real funding data and
            // are released in place as before.
            let mut release_drop_stmt = tx.prepare_cached(
                "DELETE FROM core_utxos \
                 WHERE wallet_id = ?1 AND outpoint = ?2 AND height IS NULL",
            )?;
            let mut release_stmt = tx.prepare_cached(
                "UPDATE core_utxos SET spent = 0, spent_in_txid = NULL \
                 WHERE wallet_id = ?1 AND outpoint = ?2",
            )?;
            for outpoint in &released {
                let key = blob::encode_outpoint(outpoint)?;
                let dropped = release_drop_stmt.execute(params![wallet_id.as_slice(), &key[..]])?;
                if dropped == 0 {
                    release_stmt.execute(params![wallet_id.as_slice(), &key[..]])?;
                }
            }
        }
    }
    if heights_advanced {
        collect_finalized_tombstones(tx, wallet_id)?;
    }
    Ok(())
}

/// The union of every input outpoint claimed by a surviving
/// `core_transactions` row — every row except this round's swept losers,
/// whose deletion the round itself performs.
///
/// This is the durable mirror of upstream's `retain_unclaimed` claimed-set,
/// with one decisive difference: it includes records the in-memory wallet
/// has pruned (chainlocked, under the default
/// `keep-finalized-transactions = off`) or lost across a restart. Rows of
/// EVERY context count as claimants on purpose. A mined or chainlocked
/// record's spend is settled and may never be re-freed; an InstantSend-
/// locked record's inputs are settled under DIP-10 the moment the lock
/// lands; and a plain mempool record is a claim upstream itself would have
/// retained had it still held the record — refusing matches what
/// `retain_unclaimed` computes from an unpruned, unrestarted wallet, no
/// more and no less. In-session a live claimant never appears in a released
/// set anyway (upstream filters it), so any hit here is by construction a
/// claim upstream forgot.
///
/// One pass over the wallet's rows, decoding each blob once — the same
/// build-the-set-then-probe shape (and rationale) as upstream's
/// `retain_unclaimed`: released sets follow the input count of a
/// transaction a remote peer picks, so probing per candidate would be
/// `O(released × history)` instead. The pass itself is `O(history)` blob
/// decodes, paid only by a round whose sweep actually frees candidate
/// coins — rare organically, and an attacker can only force one per
/// on-chain final transaction they pay for.
fn surviving_stored_input_claims(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    swept_txids: &HashSet<dashcore::Txid>,
) -> Result<HashSet<dashcore::OutPoint>, WalletStorageError> {
    use dashcore::hashes::Hash;

    let mut stmt =
        tx.prepare_cached("SELECT txid, record_blob FROM core_transactions WHERE wallet_id = ?1")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut claims: HashSet<dashcore::OutPoint> = HashSet::new();
    while let Some(row) = rows.next()? {
        let txid_bytes: Vec<u8> = row.get(0)?;
        let Ok(txid_array) = <[u8; 32]>::try_from(txid_bytes.as_slice()) else {
            continue;
        };
        if swept_txids.contains(&dashcore::Txid::from_byte_array(txid_array)) {
            continue;
        }
        let blob_bytes: Vec<u8> = row.get(1)?;
        let record: TransactionRecord = blob::decode(&blob_bytes)?;
        claims.extend(
            record
                .transaction
                .input
                .iter()
                .map(|input| input.previous_output),
        );
    }
    Ok(claims)
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
/// The placeholder is created for EVERY sweep context; only the stamp
/// differs. A BLOCK-CONTEXT sweep (`winner_mined_height` is `Some`)
/// stamps the winner's own mined height — the projection of key-wallet's
/// `observed_spent_outpoints`, which maps each outpoint observed spent in
/// a block to the height of the block that spent it — and
/// `collect_finalized_tombstones` evicts the row once the chainlock
/// finality boundary reaches that height, key-wallet's
/// `prune_finalized_observed_spends` condition verbatim. A
/// MEMPOOL-CONTEXT sweep (IS-locked winner, unmined) writes the same row
/// UNSTAMPED (`winner_mined_height` NULL), and the collector never takes
/// an unstamped row. The in-memory model an unstamped row mirrors is not
/// `observed_spent_outpoints` (which indeed records nothing for an
/// unconfirmed spend) but the account's `spent_outpoints`:
/// `drop_conflicted_transactions` deletes the loser and RETAINS the
/// winner's shared inputs there — a hold that carries no height, because
/// under DIP-10 the IS lock alone settles the input. That set is
/// `serde(skip_serializing)` upstream and rebuilt from live records on
/// load, so after the sweep no record can reconstruct it; this row is the
/// hold's only durable carrier, and dropping it lets a post-restart
/// funding delivery credit a coin the network has already consumed.
///
/// Nothing may collect an unstamped row, ever: an IS-locked winner has no
/// mining deadline, and the funding transaction of an input it spends may
/// itself be IS-locked and unmined (DIP-10 eligibility allows chained
/// locks), so no height watermark can prove the funding output "delivered
/// or never will be". An unstamped row instead leaves the set only
/// through proof: the funding upsert materialises it (a wallet-owned
/// claim — DIP-10 eligibility means the funding tx is mined or will mine,
/// and BIP158 matches its block by our script, so delivery is guaranteed;
/// the row gains a real `height` and becomes an ordinary spent coin), a
/// later block-context sweep re-points it and stamps it into the
/// collectible set, or a release deletes it.
///
/// The residue is foreign inputs — a swept INCOMING payment reaches this
/// loop too, and a sender-owned input's funding output never delivers, so
/// its unstamped row is permanent. It cannot be gated by ownership
/// because nothing anywhere can prove an input foreign (`input_details`
/// and `direction` are computed from the wallet's UTXO snapshot AT RECORD
/// TIME; dashpay/rust-dashcore#968 — the once-proposed "held outpoints
/// attested ours" set is empty by construction). What bounds the residue
/// is attack cost, not collection: masternodes lock first-seen, so for
/// the winner to earn the IS lock this sweep requires, the conflicting
/// loser must have been delivered straight to this wallet while withheld
/// from the network, and every batch of rows costs the attacker a
/// fee-paying, network-accepted double-spend. The unconditional-placeholder
/// shape this narrows (every context leaking rows with no collector at
/// all) does not return: block-context rows still collect at the finality
/// boundary, and only the IS-context shared-input residue is permanent.
///
/// Idempotent: a txid this store never recorded is a successful no-op, not an
/// error. A sweep can legitimately name a transaction this wallet dropped, or
/// never derived an address for in the first place. Only the loser-scoped
/// work is skipped in that case — the batch's released outpoints are applied
/// by the caller, outside this function, precisely so a missing row cannot
/// swallow them.
fn apply_sweep(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    loser_txid: &dashcore::Txid,
    superseded_by: &dashcore::Txid,
    released: &HashSet<dashcore::OutPoint>,
    swept_txids: &HashSet<dashcore::Txid>,
    winner_mined_height: Option<u32>,
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
    // A held, never-materialised claim (`height IS NULL`) is re-stamped
    // with the NEW winner's mined height when this sweep has one — the
    // claim now belongs to that winner, and its height is what the
    // collector compares against the finality boundary. An IS-locked
    // winner (`?5` NULL) re-points the claim but keeps the existing stamp:
    // the earlier block-context observation stands, exactly as upstream's
    // `observed_spent_outpoints` entry is never retracted by an
    // unconfirmed conflict, and collection at the old height stays sound —
    // the funding output of a spent outpoint is mined at or below the
    // height of ANY block-context spender of it, so the boundary passing
    // that height still proves the funding was delivered or never will be.
    // Materialised rows (`height` set) keep their NULL stamp — they are
    // outside the collector's reach either way.
    let mut spend_stmt = tx.prepare_cached(
        "UPDATE core_utxos SET spent = ?3, spent_in_txid = ?4, \
            winner_mined_height = CASE \
                WHEN ?3 AND height IS NULL THEN COALESCE(?5, winner_mined_height) \
                ELSE winner_mined_height END \
         WHERE wallet_id = ?1 AND outpoint = ?2",
    )?;
    // Only reached for a held input with no existing row — see the doc
    // comment above. `value`/`script`/`height`/`account_index` are
    // placeholders; the funding UTXO's own upsert overwrites them (and,
    // thanks to the `spent_in_txid` guard in `execute_upsert_utxo`, does
    // not clear `spent` while doing it). `winner_mined_height` is the
    // winner's own block height when the sweep has one — the row's whole
    // lifetime rule for `collect_finalized_tombstones` — and NULL for an
    // IS-locked, unmined winner, which the collector never touches: the
    // hold then lasts until the funding upsert materialises it, a later
    // block-context sweep stamps it, or a release deletes it.
    let mut tombstone_stmt = tx.prepare_cached(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid, \
             winner_mined_height) \
         VALUES (?1, ?2, 0, X'', NULL, 0, 1, ?3, ?4)",
    )?;
    for input in &loser.transaction.input {
        let outpoint = input.previous_output;
        // An input funded by a transaction this same changeset also sweeps
        // is a dead parent's output — nobody's coin, not something the
        // winner took: upstream's descendant closure always sweeps parent
        // and child together, and its release computation excludes exactly
        // these outpoints (so `freed` below can never be true for one). The
        // right end state is NO row, deleted here outright rather than
        // assumed away or marked:
        //
        // - Assuming the parent's own pass deleted it fails when the
        //   parent's record was lost (the same record-loss threat the
        //   caller's by-outpoint release pass exists for) — that pass
        //   deletes nothing, and skipping the claim here would leave the
        //   dead output `spent = 0`, a phantom spendable coin `load()`
        //   hands back.
        // - Holding it instead (`spent = 1`, `spent_in_txid = winner`, the
        //   ordinary path below) survives as a claim the funding upsert's
        //   valve then defends — against the chainlocked reinstatement
        //   that is the ONE event that can bring the coin back, whose
        //   re-emitted output must land freshly unspent.
        //
        // The delete is idempotent against the parent's own pass in either
        // batch order, and a reinstatement re-creates the real row through
        // the ordinary `utxos_added` upsert with nothing left standing in
        // its way.
        if swept_txids.contains(&outpoint.txid) {
            let key = blob::encode_outpoint(&outpoint)?;
            tx.execute(
                "DELETE FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2",
                params![wallet_id.as_slice(), &key[..]],
            )?;
            continue;
        }
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
            spent_in_txid,
            winner_mined_height.map(i64::from)
        ])?;
        if affected == 0 && !freed {
            // A held input with no row gets a placeholder in EVERY sweep
            // context — `CORE_SWEEP_REMOVAL`'s contract: each non-released
            // input retains a durable spend claim even when its funding
            // TXO has not materialised yet. An IS-locked, unmined winner
            // just leaves the stamp NULL, which the collector never
            // touches — see the doc comment above for what resolves (and
            // what bounds) an unstamped row.
            tombstone_stmt.execute(params![
                wallet_id.as_slice(),
                &key[..],
                AsRef::<[u8]>::as_ref(superseded_by),
                winner_mined_height.map(i64::from)
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
// `winner_mined_height` DOES clear: this statement always binds a real
// funding `height`, so the row it lands on is materialised from here on —
// permanently outside `collect_finalized_tombstones`'s reach — and a stale
// stamp would only mislead.
const UPSERT_UTXO_SQL: &str = "INSERT INTO core_utxos \
        (wallet_id, outpoint, value, script, height, account_index, spent, spent_in_txid) \
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL) \
     ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
        value = excluded.value, \
        script = excluded.script, \
        height = excluded.height, \
        account_index = excluded.account_index, \
        winner_mined_height = NULL, \
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
    chainlock: Option<u32>,
) -> Result<(), WalletStorageError> {
    // Monotonic-max semantics — keep the larger of (current, new).
    let current = read_sync_heights(tx, wallet_id)?;
    let max_or = |a: Option<u32>, b: Option<u32>| match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let lp = max_or(current.0, last_processed);
    let sy = max_or(current.1, synced);
    let cl = max_or(current.2, chainlock);
    tx.execute(
        "INSERT INTO core_sync_state \
            (wallet_id, last_processed_height, synced_height, chainlock_height) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            last_processed_height = excluded.last_processed_height, \
            synced_height = excluded.synced_height, \
            chainlock_height = excluded.chainlock_height",
        params![
            wallet_id.as_slice(),
            lp.map(i64::from),
            sy.map(i64::from),
            cl.map(i64::from),
        ],
    )?;
    Ok(())
}

/// The wallet's `(last_processed_height, synced_height, chainlock_height)`
/// watermark triple as read back from `core_sync_state`.
type SyncHeights = (Option<u32>, Option<u32>, Option<u32>);

/// Read the wallet's [`SyncHeights`] watermarks. All-`None` when the row
/// is absent.
fn read_sync_heights(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
) -> Result<SyncHeights, WalletStorageError> {
    let raw: (Option<i64>, Option<i64>, Option<i64>) = tx
        .query_row(
            "SELECT last_processed_height, synced_height, chainlock_height \
             FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .unwrap_or((None, None, None));
    Ok((
        sync_height_u32("core_sync_state.last_processed_height", raw.0)?,
        sync_height_u32("core_sync_state.synced_height", raw.1)?,
        sync_height_u32("core_sync_state.chainlock_height", raw.2)?,
    ))
}

/// Evict never-materialised sweep tombstones once the chainlock finality
/// boundary reaches their winner's mined height — the storage-side mirror
/// of key-wallet's `prune_finalized_observed_spends`, same condition
/// verbatim: an entry whose spend height is at or below
/// `min(chainlock_height, synced_height)` is safe to forget, because the
/// spend at that height is chain-locked and every BIP158 filter below the
/// boundary has been matched with no false negatives, so the funding
/// transaction of the outpoint it guards — necessarily mined at or below
/// the spend's own height — has either been delivered (materialising the
/// row) or provably never will be. No observation-age margin: the stamp IS
/// the winner's height, carried on the sweep event itself, so nothing here
/// guesses when the winner mined. Rows with no stamp are never collected:
/// a mempool-context sweep (IS-locked winner, unmined) deliberately
/// writes its placeholder unstamped, because such a winner has no mining
/// deadline and no watermark can prove its inputs' funding "delivered or
/// never will be" — an unstamped row is a live hold, resolved only by the
/// funding upsert materialising it, a later block-context sweep stamping
/// it, or a release deleting it (see `apply_sweep`).
///
/// Two passes, both narrowed to `height IS NULL` (only the tombstone
/// insert leaves `height` NULL, so the set is exactly the
/// never-materialised rows, served by the partial index):
///
/// 1. Released leftovers (`spent = 0`) are deleted outright — a released,
///    never-materialised claim holds nothing and would read as a
///    zero-value phantom coin. The release path now deletes these
///    in-line; this pass self-heals rows written before it did.
/// 2. Held rows whose winner height is at or below the boundary are
///    collected.
///
/// Like upstream, a no-op until a chainlock height has been persisted —
/// without a finality boundary nothing can be proven final.
fn collect_finalized_tombstones(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
) -> Result<(), WalletStorageError> {
    tx.execute(
        "DELETE FROM core_utxos \
         WHERE wallet_id = ?1 AND height IS NULL AND spent = 0",
        params![wallet_id.as_slice()],
    )?;
    let (_, sy, cl) = read_sync_heights(tx, wallet_id)?;
    let (Some(sy), Some(cl)) = (sy, cl) else {
        return Ok(());
    };
    let boundary = cl.min(sy);
    tx.execute(
        "DELETE FROM core_utxos \
         WHERE wallet_id = ?1 AND height IS NULL AND spent = 1 \
           AND winner_mined_height <= ?2",
        params![wallet_id.as_slice(), i64::from(boundary)],
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
