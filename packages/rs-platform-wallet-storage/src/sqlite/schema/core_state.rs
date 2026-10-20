//! Writers + readers for the `core_*` tables.

#[cfg(any(test, feature = "__test-helpers"))]
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use dashcore::ephemerealdata::chain_lock::ChainLock;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::Utxo;
use platform_wallet::changeset::CoreChangeSet;
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;
use crate::sqlite::load_ctx::{LoadCtx, LoadSite};
use crate::sqlite::schema::blob;
use crate::sqlite::schema::blob::impl_persistable_blob;
use crate::sqlite::schema::core_pool::{owning_account_for_script, OwningAccount};

// PUBLIC material only: core-chain state reaching `record_blob` /
// `islock_blob` (transaction records + InstantLocks are public chain data).
impl_persistable_blob!(TransactionRecord, dashcore::InstantLock);

/// Encode a `ChainLock` to bytes for storage in `core_sync_state`.
fn encode_chain_lock(cl: &ChainLock) -> Result<Vec<u8>, WalletStorageError> {
    Ok(bincode::encode_to_vec(cl, blob::bounded_config())?)
}

/// Decode a `ChainLock` from `core_sync_state.last_applied_chain_lock`.
///
/// Error mapping mirrors [`blob::decode`]: an over-cap payload is
/// [`WalletStorageError::BlobTooLarge`], trailing bytes after the typed
/// length are a `BlobDecode`, and anything else keeps the upstream bincode
/// error as its source.
///
/// # Errors
///
/// Under [`LoadPolicy::Strict`](crate::LoadPolicy) any of the above aborts
/// the load. Under `Recovery` they are counted and the field is left
/// `None`, which the next ChainLock sync repopulates. `BlobTooLarge` is
/// fatal in both — recovery tolerates inconsistent rows, not oversize
/// allocations.
fn decode_chain_lock(bytes: &[u8], ctx: &LoadCtx) -> Result<Option<ChainLock>, WalletStorageError> {
    let failure = match bincode::decode_from_slice::<ChainLock, _>(bytes, blob::bounded_config()) {
        Ok((cl, consumed)) if consumed == bytes.len() => return Ok(Some(cl)),
        Ok(_) => WalletStorageError::blob_decode(
            "unexpected trailing bytes in core_sync_state.last_applied_chain_lock",
        ),
        Err(bincode::error::DecodeError::LimitExceeded) => {
            return Err(WalletStorageError::BlobTooLarge {
                len_bytes: bytes.len(),
                limit_bytes: blob::BLOB_SIZE_LIMIT_BYTES,
            })
        }
        Err(other) => WalletStorageError::from(other),
    };
    ctx.tolerate(LoadSite::ChainLockBlob, failure)?;
    Ok(None)
}

/// Block height of an encoded `last_applied_chain_lock` blob, or `None` if it
/// can't be decoded. Used to monotonic-max-merge the chain lock so an
/// out-of-order lower-height update never regresses the finalized checkpoint.
fn chain_lock_height(bytes: &[u8]) -> Option<u32> {
    match bincode::decode_from_slice::<ChainLock, _>(bytes, blob::bounded_config()) {
        // Require full consumption (like `decode_chain_lock`) so a corrupt
        // stored blob can't out-rank a later valid update and stay stuck.
        Ok((cl, consumed)) if consumed == bytes.len() => Some(cl.block_height),
        _ => None,
    }
}

/// Apply a `CoreChangeSet` inside a transaction.
///
/// Recordless UTXOs write monotonic height-only rows; transaction records win.
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
    // `addresses_derived` is intentionally NOT persisted here — the pool
    // snapshot (`account_address_pools`) is the derived-address source used
    // to resolve UTXO ownership during rehydration.
    if !cs.new_utxos.is_empty() {
        // Blob-bearing records always win; height-only writes never overwrite them.
        // Placeholder confirmation is `height IS NOT NULL`; `finalized` stays 0.
        let mut height_only_stmt = tx.prepare_cached(
            "INSERT INTO core_transactions \
                (wallet_id, txid, height, block_hash, block_time, finalized, record_blob) \
             VALUES (?1, ?2, ?3, NULL, NULL, 0, NULL) \
             ON CONFLICT(wallet_id, txid) DO UPDATE SET height = \
                CASE \
                    WHEN excluded.height IS NOT NULL \
                         AND (core_transactions.height IS NULL \
                              OR excluded.height > core_transactions.height) \
                    THEN excluded.height \
                    ELSE core_transactions.height \
                END \
             WHERE core_transactions.record_blob IS NULL",
        )?;
        let mut utxo_stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        for utxo in &cs.new_utxos {
            let affected = height_only_stmt.execute(params![
                wallet_id.as_slice(),
                AsRef::<[u8]>::as_ref(&utxo.outpoint.txid),
                utxo.is_confirmed.then_some(i64::from(utxo.height)),
            ])?;
            if affected == 0 {
                tracing::debug!(
                    txid = %utxo.outpoint.txid,
                    "existing transaction record blocked a stale height-only write; \
                     refresh the record itself to update its confirmation height"
                );
            }
            execute_upsert_utxo(&mut utxo_stmt, wallet_id, utxo, false)?;
        }
    }
    if !cs.spent_utxos.is_empty() {
        // Only a materialized row takes the in-place fast path. A
        // never-materialised placeholder (`is_sweep_placeholder = 1`, `apply_sweep`'s
        // tombstone) must go through the full upsert instead: the wallet
        // delivering the coin as spent is a delivery, and the collector's
        // soundness argument assumes every delivery materialises the row.
        // Marking the placeholder in place would leave the placeholder flag set and
        // the stamp intact, so `collect_finalized_tombstones` would delete
        // the only durable record of the spend once the boundary passed —
        // and a later rescan re-delivery would land the coin unspent.
        let mut materialised_stmt = tx.prepare_cached(
            "SELECT 1 FROM core_utxos \
             WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 0",
        )?;
        let mut mark_spent_stmt = tx.prepare_cached(
            "UPDATE core_utxos SET spent = 1 WHERE wallet_id = ?1 AND outpoint = ?2",
        )?;
        let mut upsert_stmt = tx.prepare_cached(UPSERT_UTXO_SQL)?;
        for utxo in &cs.spent_utxos {
            let op = blob::encode_outpoint(&utxo.outpoint)?;
            let materialised: bool = materialised_stmt
                .query_row(params![wallet_id.as_slice(), &op[..]], |_| Ok(true))
                .optional()?
                .unwrap_or(false);
            if materialised {
                mark_spent_stmt.execute(params![wallet_id.as_slice(), &op[..]])?;
            } else {
                execute_upsert_utxo(&mut upsert_stmt, wallet_id, utxo, true)?;
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
        let cl_bytes = cs
            .last_applied_chain_lock
            .as_ref()
            .map(encode_chain_lock)
            .transpose()?;
        upsert_sync_state(
            tx,
            wallet_id,
            cs.last_processed_height,
            cs.synced_height,
            cl_bytes,
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
    // Only a round that actually releases something reads this, and the
    // common sweep — a resend whose winner spends every input its loser did
    // — releases nothing. Hashing every surviving record's inputs for such a
    // round would pay a per-record cost, with the writer held, for a set
    // nothing consults. Same reasoning as the lazily-built `stored_claims`
    // below.
    let releases_anything = cs
        .sweeps
        .iter()
        .any(|batch| !batch.released_outpoints.is_empty());
    let claimed_by_survivors: HashSet<dashcore::OutPoint> = if releases_anything {
        cs.records
            .iter()
            .filter(|record| !swept_txids.contains(&record.txid))
            .flat_map(|record| record.transaction.input.iter())
            .map(|input| input.previous_output)
            .collect()
    } else {
        HashSet::new()
    };
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
    // after a restart for every NETWORK-FINAL record (IS-locked, in-block,
    // chainlocked): hydration rebuilds the in-memory wallet without its
    // transaction history, so every settled claim the store holds is one
    // upstream can no longer see. Bare mempool rows are deliberately not
    // part of the veto — see `surviving_stored_input_claims` for why a
    // stale one must not strand a legitimately released coin.
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
        // row. Dropping the release set there would leave the held
        // placeholder in place, with `execute_upsert_utxo`'s valve keeping
        // it spent through every funding upsert, forever — the release is
        // the one channel that clears it. Running after the loser loop
        // rather than inside it changes nothing for inputs the loop already
        // freed (same UPDATE, idempotent), and a coin a surviving record in
        // this round re-claimed was already filtered out of `released`
        // above.
        if !released.is_empty() {
            // A released claim that never materialised is deleted outright
            // rather than flipped to `spent = 0`: the row is all placeholder
            // (`value = 0`, `script = X''`, placeholder flag set), so releasing it in
            // place would surface a zero-value phantom coin through
            // `list_unspent_utxos`. No row is the correct end state — if the
            // funding output ever classifies, its ordinary upsert creates
            // the real row freshly unspent, exactly as if the dead claim had
            // never existed. Materialised rows carry real funding data and
            // are released in place as before.
            //
            // An output of a transaction swept in this very round is the
            // one exception, and it is deleted whatever its shape: a coin
            // created by a dead transaction cannot be unspent, only gone.
            // The loser loop already refuses to release such an outpoint
            // (it deletes the row and moves on), but this pass runs
            // regardless of whether the parent's own record survived —
            // that is its whole point — and with the parent's row lost
            // nothing above has removed the parent's materialised output,
            // so releasing it in place would hand back a spendable coin
            // from a transaction that can never confirm.
            let mut swept_output_drop_stmt =
                tx.prepare_cached("DELETE FROM core_utxos WHERE wallet_id = ?1 AND outpoint = ?2")?;
            let mut release_drop_stmt = tx.prepare_cached(
                "DELETE FROM core_utxos \
                 WHERE wallet_id = ?1 AND outpoint = ?2 AND is_sweep_placeholder = 1",
            )?;
            let mut release_stmt = tx.prepare_cached(
                "UPDATE core_utxos SET spent = 0, spent_in_txid = NULL \
                 WHERE wallet_id = ?1 AND outpoint = ?2",
            )?;
            for outpoint in &released {
                let key = blob::encode_outpoint(outpoint)?;
                if swept_txids.contains(&outpoint.txid) {
                    swept_output_drop_stmt.execute(params![wallet_id.as_slice(), &key[..]])?;
                    continue;
                }
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
/// `keep-finalized-transactions = off`) or lost across a restart.
///
/// Only NETWORK-FINAL claimants count: InstantSend-locked (settled under
/// DIP-10 the moment the lock lands), in-block, or chainlocked. A bare
/// `Mempool` row is deliberately not settled-spend evidence, because it is
/// the one context that can go stale forever: an evicted or abandoned
/// mempool transaction has no removal path in this store other than a later
/// sweep (upstream's abandon path emits no events —
/// dashpay/rust-dashcore#976), and restoration deliberately does not
/// repopulate ordinary transaction history, so nothing ever re-asserts or
/// retracts the row. Letting it veto an authoritative release would leave
/// the coin attributed to an unrelated winner and durably spent — the
/// mirror image of the wrong-release bug this guard exists to stop. This is
/// also exactly the mobile stores' rule: their link guard protects a
/// network-final spender's link and lets a mempool link be replaced. A LIVE
/// mempool claim loses nothing here: in-session upstream holds the record
/// and never names its inputs released, and within the round
/// `claimed_by_survivors` carries the changeset's own mempool records. The
/// one accepted trade: after a restart a still-alive mempool claimant on
/// disk no longer vetoes, so the release wins and the coin may be
/// transiently re-offered while that pending spend races — self-resolving
/// when the pending spend confirms or dies, and strictly better than a
/// permanent strand.
///
/// Fails CLOSED. This scan is the final guard against re-crediting a
/// consumed coin, so a malformed stored key must fail the round rather than
/// silently drop that row's veto: a `txid` column of the wrong length and a
/// record blob whose decoded `TransactionRecord::txid` disagrees with the
/// typed key (the key is what excludes a row as a swept loser) are both
/// `BlobDecode` errors, matching the other typed-column readers.
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
        tx.prepare_cached("SELECT txid, record_blob FROM core_transactions WHERE wallet_id = ?1 AND record_blob IS NOT NULL")?;
    let mut rows = stmt.query(params![wallet_id.as_slice()])?;
    let mut claims: HashSet<dashcore::OutPoint> = HashSet::new();
    while let Some(row) = rows.next()? {
        let txid_bytes: Vec<u8> = row.get(0)?;
        let Ok(txid_array) = <[u8; 32]>::try_from(txid_bytes.as_slice()) else {
            return Err(WalletStorageError::blob_decode(
                "core_transactions.txid must be exactly 32 bytes",
            ));
        };
        let key_txid = dashcore::Txid::from_byte_array(txid_array);
        if swept_txids.contains(&key_txid) {
            continue;
        }
        let blob_bytes: Vec<u8> = row.get(1)?;
        let record: TransactionRecord = blob::decode(&blob_bytes)?;
        if record.txid != key_txid {
            return Err(WalletStorageError::blob_decode(
                "core_transactions.txid disagrees with the decoded record's txid",
            ));
        }

        if matches!(record.context, TransactionContext::Mempool) {
            continue;
        }
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
/// durable — it refuses to clear `spent` on a never-materialised held row
/// (`is_sweep_placeholder = 1 AND spent = 1`), so the claim survives the funding
/// upsert instead of being upserted away by it. The hold is keyed on that
/// shape rather than on `spent_in_txid`, which the
/// `setnull_core_utxos_on_tx_delete` trigger can clear underneath it (see
/// the valve's own comment); the link names the current claimant for the
/// chained-sweep re-point below and is informational otherwise.
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
/// the row clears its placeholder flag and becomes an ordinary spent coin), a
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
            "SELECT record_blob FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2 AND record_blob IS NOT NULL",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
            |row| row.get(0),
        )
        .optional()?;
    // Before the early return, deliberately. `instant_locks_for_non_final_records`
    // is a separate map that merges independently of `records`, so a lock row can
    // outlive its record — a fatal flush that discards a buffered round is the
    // documented way. Nothing ties that table to `core_transactions` (no foreign
    // key, no trigger), so a lock skipped here survives forever, describing a
    // transaction the wallet has removed. The delete is txid-keyed and
    // idempotent, so running it on the missing-record path costs nothing.
    tx.execute(
        "DELETE FROM core_instant_locks WHERE wallet_id = ?1 AND txid = ?2",
        params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
    )?;
    let Some(loser_blob) = loser_blob else {
        // Height-only records carry no input list, but their outputs still belong to the loser.
        tx.execute(
            "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
        )?;
        // `encode_outpoint_txid_occupies_bytes_two_to_thirty_three` pins this prefix.
        tx.execute(
            "DELETE FROM core_utxos WHERE wallet_id = ?1 AND substr(outpoint, 2, 32) = ?2",
            params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(loser_txid)],
        )?;
        return Ok(());
    };
    let loser: TransactionRecord = blob::decode(&loser_blob)?;
    // Fails CLOSED on a key/record disagreement, before anything is deleted
    // or any input is touched. The typed key is what named this row a swept
    // loser — both here and in `surviving_stored_input_claims`, which skips
    // the row on the key alone and so never contributes its blob's claims to
    // the veto set. A row keyed `loser_txid` but holding some other record's
    // blob would therefore have that record's inputs processed as this
    // loser's, with its claimant veto already waived: a release naming a coin
    // the stored record legitimately consumed would mark that coin unspent
    // and delete the only stored evidence of its spender. Same `BlobDecode`
    // verdict as the claim scan's own mismatch check, for the same reason.
    if loser.txid != *loser_txid {
        return Err(WalletStorageError::blob_decode(
            "core_transactions.txid disagrees with the swept record's txid",
        ));
    }

    tx.execute(
        "DELETE FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
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

    // Each input is set outright rather than only touched when it changes:
    // whichever way it went, the row must end this round agreeing with the
    // wallet, and a coin the sweep did not free stays out of the unspent
    // query even if nothing had marked it spent yet (upstream sweeps only
    // unconfirmed records, whose spends this schema does not mark).
    // `spent_in_txid` moves with `spent`: a released input clears back to
    // NULL (nobody's claim), a held one is attributed to `superseded_by` so
    // the claim outlives this row's own deletion below.
    // A held, never-materialised claim (`is_sweep_placeholder = 1`) is re-stamped
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
    // Materialised rows (placeholder flag clear) keep their NULL stamp — they are
    // outside the collector's reach either way.
    let mut spend_stmt = tx.prepare_cached(
        "UPDATE core_utxos SET spent = ?3, spent_in_txid = ?4, \
            winner_mined_height = CASE \
                WHEN ?3 AND is_sweep_placeholder = 1 THEN COALESCE(?5, winner_mined_height) \
                ELSE winner_mined_height END \
         WHERE wallet_id = ?1 AND outpoint = ?2",
    )?;
    // Only reached for a held input with no existing row — see the doc
    // comment above. `value`/`script` are
    // placeholders; the funding UTXO's own upsert overwrites them (and,
    // thanks to the held-placeholder valve in `execute_upsert_utxo`, does
    // not clear `spent` while doing it). `winner_mined_height` is the
    // winner's own block height when the sweep has one — the row's whole
    // lifetime rule for `collect_finalized_tombstones` — and NULL for an
    // IS-locked, unmined winner, which the collector never touches: the
    // hold then lasts until the funding upsert materialises it, a later
    // block-context sweep stamps it, or a release deletes it.
    let mut tombstone_stmt = tx.prepare_cached(
        "INSERT INTO core_utxos \
            (wallet_id, outpoint, value, script, is_sweep_placeholder, spent, spent_in_txid, \
             winner_mined_height) \
         VALUES (?1, ?2, 0, X'', 1, 1, ?3, ?4)",
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
        //   ordinary path below) either survives as a placeholder the
        //   funding upsert's valve then defends — against the chainlocked
        //   reinstatement that is the ONE event that can bring the coin
        //   back, whose re-emitted output must land freshly unspent — or,
        //   for a materialised row, keeps a dead coin on disk until that
        //   reinstatement, with nothing else able to remove it.
        //
        // The delete is idempotent against the parent's own pass in either
        // batch order, and a reinstatement re-creates the real row through
        // the ordinary `utxos_added` upsert with nothing left standing in
        // its way.
        if swept_txids.contains(&outpoint.txid) {
            let key = blob::encode_outpoint(&outpoint)?;
            delete_output_stmt.execute(params![wallet_id.as_slice(), &key[..]])?;
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

const UPSERT_UTXO_SQL: &str = "INSERT INTO core_utxos \
        (wallet_id, outpoint, value, script, spent) \
     VALUES (?1, ?2, ?3, ?4, ?5) \
     ON CONFLICT(wallet_id, outpoint) DO UPDATE SET \
        value = excluded.value, \
        script = excluded.script, \
        is_sweep_placeholder = 0, \
        winner_mined_height = NULL, \
        spent = CASE WHEN core_utxos.is_sweep_placeholder = 1 AND core_utxos.spent \
            THEN 1 ELSE excluded.spent END, \
        spent_in_txid = CASE \
            WHEN core_utxos.is_sweep_placeholder = 1 AND core_utxos.spent THEN core_utxos.spent_in_txid \
            WHEN excluded.spent THEN core_utxos.spent_in_txid \
            ELSE NULL END";

/// Upsert one `core_utxos` row; `spent` marks spent-only synthetic rows.
///
/// # Errors
///
/// [`WalletStorageError::EmptyUtxoScript`] when the script is empty. This
/// writes materialized `core_utxos.script` values, so refusing here is what
/// keeps the reader's `Address::from_script` reachable only for scripts
/// that can exist — a stored empty one fails the load of the whole file.
fn execute_upsert_utxo(
    stmt: &mut rusqlite::CachedStatement<'_>,
    wallet_id: &WalletId,
    utxo: &Utxo,
    spent: bool,
) -> Result<(), WalletStorageError> {
    if utxo.txout.script_pubkey.as_bytes().is_empty() {
        return Err(WalletStorageError::EmptyUtxoScript {
            outpoint: utxo.outpoint,
        });
    }
    let op = blob::encode_outpoint(&utxo.outpoint)?;
    stmt.execute(params![
        wallet_id.as_slice(),
        &op[..],
        crate::sqlite::util::safe_cast::u64_to_i64("core_utxos.value", utxo.value())?,
        utxo.txout.script_pubkey.as_bytes(),
        spent,
    ])?;
    Ok(())
}

fn upsert_sync_state(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
    last_processed: Option<u32>,
    synced: Option<u32>,
    chain_lock_bytes: Option<Vec<u8>>,
    chainlock: Option<u32>,
) -> Result<(), WalletStorageError> {
    // Read current row for monotonic-max height merge + to carry forward any
    // existing chain lock when the changeset doesn't include a new one.
    let current_raw: (Option<i64>, Option<i64>, Option<Vec<u8>>) = tx
        .query_row(
            "SELECT last_processed_height, synced_height, last_applied_chain_lock \
             FROM core_sync_state WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .unwrap_or((None, None, None));
    // Monotonic-max semantics for sync watermarks.
    let current = (
        height_column_u32("core_sync_state.last_processed_height", current_raw.0)?,
        height_column_u32("core_sync_state.synced_height", current_raw.1)?,
    );
    let lp = match (current.0, last_processed) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    let sy = match (current.1, synced) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    };
    // Chain lock: monotonic-max by height like the sync watermarks above.
    // A new chain lock replaces the stored one only when its height is >=
    // the stored height, so an out-of-order lower-height update can't
    // regress the finalized checkpoint. `None` (no update) keeps existing.
    let cl_final = match (chain_lock_bytes, current_raw.2) {
        (Some(new_bytes), Some(existing_bytes)) => {
            if chain_lock_height(&new_bytes) >= chain_lock_height(&existing_bytes) {
                Some(new_bytes)
            } else {
                Some(existing_bytes)
            }
        }
        (Some(new_bytes), None) => Some(new_bytes),
        (None, existing) => existing,
    };
    let existing_height = read_sync_heights(tx, wallet_id)?.2;
    let cl = existing_height.into_iter().chain(chainlock).max();
    tx.execute(
        "INSERT INTO core_sync_state \
            (wallet_id, last_processed_height, synced_height, last_applied_chain_lock, chainlock_height) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(wallet_id) DO UPDATE SET \
            last_processed_height = excluded.last_processed_height, \
            synced_height = excluded.synced_height, \
            last_applied_chain_lock = excluded.last_applied_chain_lock, \
            chainlock_height = excluded.chainlock_height",
        params![
            wallet_id.as_slice(),
            lp.map(i64::from),
            sy.map(i64::from),
            cl_final,
            cl.map(i64::from),
        ],
    )?;
    Ok(())
}

/// Bulk-reconstruct the keyless [`CoreChangeSet`] projection for one wallet
/// from the `core_*` tables, plus the per-outpoint owning-account side channel.
/// PUBLIC material only; mints no `Wallet`. `network` (from `wallets`) turns a
/// persisted `script` back into an `Address`.
///
/// [`CoreChangeSet::new_utxos`] cannot carry each UTXO's owning account (it is a
/// bare `Vec<Utxo>`), so the returned map surfaces, per unspent outpoint, the
/// funds account that owns it — resolved by matching the UTXO's script against
/// `core_address_pool`. [`apply_persisted_core_state`](crate::sqlite::rehydrate::apply_persisted_core_state)
/// consumes it to route each UTXO to its true account. An outpoint whose script
/// matches no pool row is absent from the map and falls back to the first funds
/// account (the one-way historical-attribution default; re-warms on next sync).
///
/// # Reconstructed (safety-critical-correct)
///
/// - **Unspent UTXOs** (`new_utxos`): every `spent = 0` row — the balance
///   source (no-silent-zero); confirmation height comes from the matching
///   `core_transactions` row. A missing row or height loads as unconfirmed.
/// - **Transaction records**: height-only rows supply UTXO confirmation
///   metadata but are not emitted as records. Blob-bearing rows are decoded
///   and checked against their typed txid and height columns.
/// - **IS-locks** / **sync watermarks**: decoded bit-exact, fail-hard on a
///   corrupt blob.
///
/// # Deferred to the first post-load `sync` (safe re-warm)
///
/// - **`is_coinbase` / `is_instantlocked` / `is_trusted` / `used` flags**: not
///   carried by `core_utxos`; defaulted and refreshed on the next scan.
pub fn load_state(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
    ctx: &LoadCtx,
) -> Result<
    (
        CoreChangeSet,
        std::collections::HashMap<dashcore::OutPoint, OwningAccount>,
    ),
    WalletStorageError,
> {
    let mut cs = CoreChangeSet::default();
    let mut utxo_accounts: HashMap<dashcore::OutPoint, OwningAccount> = HashMap::new();

    let mut transaction_heights: HashMap<dashcore::Txid, Option<u32>> = HashMap::new();
    let mut blob_backed_transaction_heights = HashSet::new();
    {
        use dashcore::hashes::Hash;

        // Pre-read length gates keep fixed-width txids and record blobs from
        // being materialized before their stored sizes are validated.
        let mut stmt = conn.prepare(
            "SELECT length(txid), txid, height, length(record_blob), record_blob \
             FROM core_transactions WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            blob::check_fixed_width(row.get::<_, i64>(0)?, 32, "core_transactions.txid")?;
            let txid_bytes: Vec<u8> = row.get(1)?;
            let txid = dashcore::Txid::from_slice(&txid_bytes)?;
            let height =
                height_column_u32("core_transactions.height", row.get::<_, Option<i64>>(2)?)?;
            let mut effective_txid = txid;
            let mut effective_height = height;
            if let Some(record_blob_len) = row.get::<_, Option<i64>>(3)? {
                blob::check_size(record_blob_len)?;
                let payload: Vec<u8> = row.get(4)?;
                let record = blob::decode::<TransactionRecord>(&payload)?;
                effective_txid = record.txid;
                effective_height = record.block_info().map(|block_info| block_info.height());
                if let Err(mismatch) =
                    ensure_transaction_record_matches_columns(&txid, height, &record)
                {
                    // The blob is authoritative, so the projection keeps
                    // using it; the typed columns are left exactly as found.
                    ctx.tolerate(LoadSite::CoreTransactionColumnDrift, mismatch)?;
                }
                cs.records.push(record);
                transaction_heights.insert(effective_txid, effective_height);
                blob_backed_transaction_heights.insert(effective_txid);
            } else if !blob_backed_transaction_heights.contains(&effective_txid) {
                transaction_heights
                    .entry(effective_txid)
                    .or_insert(effective_height);
            }
        }
    }

    // Unspent UTXOs → new_utxos (the balance source).
    // Pre-read `length()` gates on `outpoint` and `script` before materializing
    // the Vec so tampered oversize values are caught before heap allocation.
    // Uses `prepare + query + while let` (not `query_map`) so the typed
    // `BlobTooLarge` error can be returned from the loop body directly.
    {
        let mut stmt = conn.prepare(
            "SELECT length(outpoint), outpoint, value, length(script), script \
             FROM core_utxos WHERE wallet_id = ?1 AND spent = 0",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            // col 0: length(outpoint) — gate before materializing
            blob::check_size(row.get::<_, i64>(0)?)?;
            let op_bytes: Vec<u8> = row.get(1)?;
            let value: i64 = row.get(2)?;
            // col 3: length(script) — gate before materializing
            blob::check_size(row.get::<_, i64>(3)?)?;
            let script_bytes: Vec<u8> = row.get(4)?;
            let outpoint = blob::decode_outpoint(&op_bytes)?;
            let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
            let height = transaction_heights.get(&outpoint.txid).copied().flatten();
            let script = dashcore::ScriptBuf::from_bytes(script_bytes);
            if let Some(owner) = owning_account_for_script(conn, wallet_id, script.as_bytes())? {
                utxo_accounts.insert(outpoint, owner);
            }
            // TODO(unspent-script-recovery-tolerance): Recovery tolerance
            // for an undecodable unspent script is deliberately deferred.
            // This stays fail-hard because it is the balance source —
            // tolerating a failed decode drops a UTXO and silently
            // under-reports the balance. The cost of deferring is severe
            // and measured: `load()` builds every healthy wallet in the
            // file, then discards all of it when a later wallet hits this
            // line, because the per-wallet loop returns `Ok(state)` only
            // after it completes. Under Recovery — the mode whose purpose
            // is to hand back whatever it can — one bad row still costs
            // the user every wallet in the file.
            let address = dashcore::Address::from_script(&script, network)?;
            let utxo = Utxo {
                outpoint,
                txout: dashcore::TxOut {
                    value,
                    script_pubkey: script,
                },
                address,
                height: height.unwrap_or(0),
                is_coinbase: false,
                is_confirmed: height.is_some(),
                is_instantlocked: false,
                is_locked: false,
                is_trusted: false,
            };
            cs.new_utxos.push(utxo);
        }
    }

    {
        // Same pre-read length gate as `record_blob` above. `txid` is a raw
        // 32-byte hash, so its width is gated fixed before materializing —
        // an oversize column raises `BlobTooLarge` ahead of the `Vec` alloc
        // rather than materializing then failing in `Txid::from_slice`.
        let mut stmt = conn.prepare(
            "SELECT length(txid), txid, length(islock_blob), islock_blob \
             FROM core_instant_locks WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        while let Some(row) = rows.next()? {
            use dashcore::hashes::Hash;
            blob::check_fixed_width(row.get::<_, i64>(0)?, 32, "core_instant_locks.txid")?;
            let txid_bytes: Vec<u8> = row.get(1)?;
            blob::check_size(row.get::<_, i64>(2)?)?;
            let blob_bytes: Vec<u8> = row.get(3)?;
            let txid = dashcore::Txid::from_slice(&txid_bytes)?;
            let islock: dashcore::ephemerealdata::instant_lock::InstantLock =
                blob::decode(&blob_bytes)?;
            cs.instant_locks_for_non_final_records.insert(txid, islock);
        }
    }

    // Sync watermarks + persisted chain lock. Read `length()` first so an
    // oversize chain-lock blob is rejected before the Vec is allocated.
    {
        let mut stmt = conn.prepare(
            "SELECT last_processed_height, synced_height, \
                    length(last_applied_chain_lock), last_applied_chain_lock \
             FROM core_sync_state WHERE wallet_id = ?1",
        )?;
        let mut rows = stmt.query(params![wallet_id.as_slice()])?;
        if let Some(row) = rows.next()? {
            let lp: Option<i64> = row.get(0)?;
            let sy: Option<i64> = row.get(1)?;
            // Gate before materializing: NULL length means no chain lock.
            if let Some(n) = row.get::<_, Option<i64>>(2)? {
                blob::check_size(n)?;
            }
            let cl_bytes: Option<Vec<u8>> = row.get(3)?;
            // Fail-hard on an out-of-range watermark (corruption never skipped).
            cs.last_processed_height =
                height_column_u32("core_sync_state.last_processed_height", lp)?;
            cs.synced_height = height_column_u32("core_sync_state.synced_height", sy)?;
            // Policy decides: strict aborts on a corrupt chain-lock blob,
            // recovery leaves the field None for the next ChainLock event.
            if let Some(bytes) = cl_bytes {
                cs.last_applied_chain_lock = decode_chain_lock(&bytes, ctx)?;
            }
        }
    }

    Ok((cs, utxo_accounts))
}

/// Every address that has ever held a `core_utxos` row for this wallet —
/// spent **and** unspent — deduplicated, each paired with its resolved
/// owning account. The rehydration address-reuse guard: an address whose
/// UTXO was since spent must still be marked used so it's never handed back
/// out as a fresh receive address.
///
/// `core_utxos` carries no unambiguous account attribution, so ownership is
/// resolved per script via [`owning_account_for_script`]; the result is
/// `None` when the script matches no pool row (the caller then routes to the
/// first funds account). `network` turns each persisted `script` back into an
/// [`Address`](dashcore::Address). An invalid script is fatal under Strict;
/// Recovery counts and skips that address while continuing with the rest.
/// This compatibility entry point uses Strict; rehydration calls
/// [`load_used_addresses_with_ctx`] with its policy context.
pub fn load_used_addresses(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
) -> Result<Vec<(dashcore::Address, Option<OwningAccount>)>, WalletStorageError> {
    load_used_addresses_with_ctx(conn, wallet_id, network, &LoadCtx::strict())
}

/// [`load_used_addresses`] under an explicit load policy.
pub fn load_used_addresses_with_ctx(
    conn: &Connection,
    wallet_id: &WalletId,
    network: dashcore::Network,
    ctx: &LoadCtx,
) -> Result<Vec<(dashcore::Address, Option<OwningAccount>)>, WalletStorageError> {
    // Gate the largest stored `script` with a cheap aggregate BEFORE the
    // `DISTINCT ... ORDER BY script` read materializes or sorts any blob, so a
    // corrupt/oversize column raises a typed `BlobTooLarge` (the crate's 16 MiB
    // cap) rather than SQLite's own `TooBig` mid-sort, and never OOMs the host.
    // `core_utxos` has no `(wallet_id, script)` index, so the read would sort
    // the blob; the aggregate gate fires first regardless of query plan.
    blob::check_max_column_len(
        conn,
        "SELECT MAX(length(script)) FROM core_utxos WHERE wallet_id = ?1",
        wallet_id,
    )?;
    // Materialize the scripts before resolving ownership: `owning_account_for_script`
    // prepares its own statement on `conn`, so the reader statement must be
    // finished first.
    let scripts: Vec<Vec<u8>> = {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT script FROM core_utxos WHERE wallet_id = ?1 AND is_sweep_placeholder = 0 ORDER BY script",
        )?;
        let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
            row.get::<_, Vec<u8>>(0)
        })?;
        rows.collect::<Result<_, _>>()?
    };
    let mut out = Vec::with_capacity(scripts.len());
    for raw in scripts {
        let owner = owning_account_for_script(conn, wallet_id, &raw)?;
        let address = match blob::decode_script_to_address(raw, network) {
            Ok(address) => address,
            Err(error) => {
                ctx.tolerate(LoadSite::UndecodableAddressScript, error)?;
                continue;
            }
        };
        out.push((address, owner));
    }
    Ok(out)
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
        height_column_u32("core_sync_state.last_processed_height", raw.0)?,
        height_column_u32("core_sync_state.synced_height", raw.1)?,
        height_column_u32("core_sync_state.chainlock_height", raw.2)?,
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
/// One pass, narrowed to `is_sweep_placeholder = 1` (only the tombstone insert
/// sets the placeholder flag, so the set is exactly the never-materialised
/// rows, served by the partial index): held rows whose winner height is
/// at or below the boundary are collected. There is no released-leftover
/// shape to sweep up — a release deletes a never-materialised row in-line
/// (see the release pass in [`apply`]), and the loser loop's transient
/// `spent = 0` on a placeholder is always followed by that pass in the
/// same transaction.
///
/// Like upstream, a no-op until a chainlock height has been persisted —
/// without a finality boundary nothing can be proven final.
fn collect_finalized_tombstones(
    tx: &Transaction<'_>,
    wallet_id: &WalletId,
) -> Result<(), WalletStorageError> {
    let (_, sy, cl) = read_sync_heights(tx, wallet_id)?;
    let (Some(sy), Some(cl)) = (sy, cl) else {
        return Ok(());
    };
    let boundary = cl.min(sy);
    let mut stmt = tx.prepare_cached(
        "DELETE FROM core_utxos \
         WHERE wallet_id = ?1 AND is_sweep_placeholder = 1 AND spent = 1 \
           AND winner_mined_height <= ?2",
    )?;
    stmt.execute(params![wallet_id.as_slice(), i64::from(boundary)])?;
    Ok(())
}

/// Convert a stored sync-height column to `u32`, erroring on overflow
/// rather than silently truncating a corrupt/out-of-range value.
fn height_column_u32(
    field: &'static str,
    value: Option<i64>,
) -> Result<Option<u32>, WalletStorageError> {
    value
        .map(|v| crate::sqlite::util::safe_cast::i64_to_u32(field, v))
        .transpose()
}

/// Fetch a single transaction record by txid.
///
/// Returns `Ok(None)` when the row is absent or carries only confirmation
/// height metadata.
/// A height-only row is not synthesized because UTXO height is not attested
/// block context and must not masquerade as a `BlockInfo`.
pub fn get_tx_record(
    conn: &Connection,
    wallet_id: &WalletId,
    txid: &dashcore::Txid,
    ctx: &LoadCtx,
) -> Result<Option<TransactionRecord>, WalletStorageError> {
    // Pre-read `length()` gate before materializing, consistent with the
    // bulk load_state path above.
    let mut stmt = conn.prepare_cached(
        "SELECT height, length(record_blob), record_blob FROM core_transactions \
         WHERE wallet_id = ?1 AND txid = ?2",
    )?;
    let mut rows = stmt.query(params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(txid)])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    let height = height_column_u32("core_transactions.height", row.get::<_, Option<i64>>(0)?)?;
    let Some(record_blob_len) = row.get::<_, Option<i64>>(1)? else {
        return Ok(None);
    };
    blob::check_size(record_blob_len)?;
    let payload: Vec<u8> = row.get(2)?;
    let record = blob::decode(&payload)?;
    drop(rows);
    drop(stmt);
    if let Err(mismatch) = ensure_transaction_record_matches_columns(txid, height, &record) {
        // Captured before `mismatch` moves: only a txid disagreement means
        // the row is a different transaction from the one asked for.
        let names_another_transaction = record.txid != *txid;
        ctx.tolerate(LoadSite::CoreTransactionColumnDrift, mismatch)?;
        // Recovery tolerated the drift, which licenses continuing the load —
        // not answering a point read with someone else's transaction. Height
        // drift keeps the blob authoritative and still serves the record.
        if names_another_transaction {
            return Ok(None);
        }
    }
    Ok(Some(record))
}

/// Diagnose typed txid/height columns that disagree with the authoritative blob.
fn ensure_transaction_record_matches_columns(
    typed_txid: &dashcore::Txid,
    typed_height: Option<u32>,
    record: &TransactionRecord,
) -> Result<(), WalletStorageError> {
    let blob_height = record.block_info().map(|block_info| block_info.height());
    if record.txid != *typed_txid || blob_height != typed_height {
        return Err(WalletStorageError::CoreTransactionEntryMismatch {
            typed_txid: typed_txid.to_string(),
            blob_txid: record.txid.to_string(),
            typed_height,
            blob_height,
        });
    }
    Ok(())
}

/// Row representing one unspent UTXO. Used by tests that probe the
/// `core_utxos` table without going through full `Wallet` reconstruction.
#[cfg(any(test, feature = "__test-helpers"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnspentRow {
    pub outpoint: dashcore::OutPoint,
    pub value: u64,
    pub script: Vec<u8>,
    pub account_index: u32,
}

/// All UTXOs for a wallet that have not been spent yet, bucketed by the
/// account index resolved from `core_address_pool` during the read.
#[cfg(any(test, feature = "__test-helpers"))]
pub fn list_unspent_utxos(
    conn: &Connection,
    wallet_id: &WalletId,
) -> Result<BTreeMap<u32, Vec<UnspentRow>>, WalletStorageError> {
    let mut stmt = conn.prepare_cached(
        "SELECT outpoint, value, script \
         FROM core_utxos WHERE wallet_id = ?1 AND spent = 0",
    )?;
    let rows = stmt.query_map(params![wallet_id.as_slice()], |row| {
        let op_bytes: Vec<u8> = row.get(0)?;
        let value: i64 = row.get(1)?;
        let script: Vec<u8> = row.get(2)?;
        Ok((op_bytes, value, script))
    })?;
    let mut by_account: BTreeMap<u32, Vec<UnspentRow>> = BTreeMap::new();
    for r in rows {
        let (op_bytes, value, script_bytes) = r?;
        let outpoint = blob::decode_outpoint(&op_bytes)?;
        let value = crate::sqlite::util::safe_cast::i64_to_u64("core_utxos.value", value)?;
        let account_index = owning_account_for_script(conn, wallet_id, &script_bytes)?
            .map(|owner| owner.account_index)
            .unwrap_or(0);
        let row = UnspentRow {
            outpoint,
            value,
            script: script_bytes,
            account_index,
        };
        by_account.entry(account_index).or_default().push(row);
    }
    Ok(by_account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::address::Payload;
    use dashcore::hashes::Hash;
    use dashcore::{BlockHash, OutPoint, PubkeyHash, Transaction, TxOut, Txid};
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};

    fn transaction_record(txid: Txid, context: TransactionContext) -> TransactionRecord {
        let mut record = TransactionRecord::new(
            Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Incoming,
            Vec::new(),
            Vec::new(),
            100,
        );
        record.txid = txid;
        record
    }

    fn sample_utxo(txid: Txid, height: u32, is_confirmed: bool) -> Utxo {
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0x23u8; 20])),
        );
        Utxo {
            outpoint: OutPoint { txid, vout: 0 },
            txout: TxOut {
                value: 150_000,
                script_pubkey: address.script_pubkey(),
            },
            address,
            height,
            is_coinbase: false,
            is_confirmed,
            is_instantlocked: false,
            is_locked: false,
            is_trusted: false,
        }
    }

    fn sample_chain_lock(height: u32) -> ChainLock {
        ChainLock {
            block_height: height,
            block_hash: BlockHash::from_byte_array([0x11u8; 32]),
            signature: [0x22u8; 96].into(),
        }
    }

    /// A tampered `core_instant_locks.txid` that overflows the blob cap must
    /// raise `BlobTooLarge` from the fixed-width gate BEFORE the oversize `Vec`
    /// is materialized — not `BlobDecode` after `Txid::from_slice` on a
    /// multi-megabyte allocation.
    #[test]
    fn load_state_rejects_oversize_instant_lock_txid() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let w = [0xABu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();

        // Plant a txid one byte past the 16 MiB cap; islock_blob content is
        // irrelevant — the txid gate fires before it is read.
        let oversize_txid = vec![0u8; crate::SIZE_LIMIT_BYTES + 1];
        conn.execute(
            "INSERT INTO core_instant_locks (wallet_id, txid, islock_blob) VALUES (?1, ?2, ?3)",
            params![&w[..], oversize_txid.as_slice(), &[0u8; 4][..]],
        )
        .unwrap();

        let err = load_state(&conn, &w, dashcore::Network::Testnet, &LoadCtx::strict())
            .expect_err("load_state must reject an oversize instant-lock txid");
        assert!(
            matches!(err, WalletStorageError::BlobTooLarge { .. }),
            "expected BlobTooLarge from the pre-materialization gate, got {err:?}"
        );
    }

    #[test]
    fn load_state_reconciles_utxo_height_from_confirmed_transaction_record() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x42u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x7Eu8; 32]);
        let utxo = sample_utxo(txid, 123, true);
        let outpoint = utxo.outpoint;

        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    records: vec![transaction_record(txid, TransactionContext::Mempool)],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let confirmed_height = 321;
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(
                        txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            confirmed_height,
                            BlockHash::from_byte_array([0x34u8; 32]),
                            1_735_689_600,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let loaded = state
            .new_utxos
            .iter()
            .find(|candidate| candidate.outpoint == outpoint)
            .expect("matching UTXO must be loaded");
        assert_eq!(loaded.height, confirmed_height);
        assert!(loaded.is_confirmed);
    }

    #[test]
    fn load_state_restores_confirmed_recordless_utxo_height() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x44u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x80u8; 32]);
        let utxo = sample_utxo(txid, 456, true);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let loaded = state.new_utxos.first().expect("recordless UTXO must load");
        assert_eq!(loaded.height, 456);
        assert!(loaded.is_confirmed);
        assert!(state.records.is_empty());
        assert!(get_tx_record(&conn, &wallet_id, &txid, &LoadCtx::strict())
            .unwrap()
            .is_none());
    }

    #[test]
    fn height_only_placeholder_does_not_regress() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x49u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x86u8; 32]);
        let mut utxo = sample_utxo(txid, 500, true);
        for stale in [
            CoreChangeSet {
                new_utxos: vec![utxo.clone()],
                ..Default::default()
            },
            {
                utxo.height = 400;
                CoreChangeSet {
                    new_utxos: vec![utxo.clone()],
                    ..Default::default()
                }
            },
            {
                utxo.height = 0;
                utxo.is_confirmed = false;
                CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                }
            },
        ] {
            let tx = conn.transaction().unwrap();
            apply(&tx, &wallet_id, &stale).unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, 500);
        assert!(state.new_utxos[0].is_confirmed);
    }

    #[test]
    fn load_state_treats_height_zero_as_confirmed() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x4Au8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x87u8; 32]);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                new_utxos: vec![sample_utxo(txid, 0, true)],
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, 0);
        assert!(state.new_utxos[0].is_confirmed);
    }

    #[test]
    fn transaction_record_always_overrides_height_only_placeholder() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x45u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x81u8; 32]);
        let mut utxo = sample_utxo(txid, 456, true);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo.clone()],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(txid, TransactionContext::Mempool)],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        utxo.height = 789;
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, 0);
        assert!(!state.new_utxos[0].is_confirmed);
        assert_eq!(state.records.len(), 1);

        let confirmed_height = 900;
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(
                        txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            confirmed_height,
                            BlockHash::from_byte_array([0x35u8; 32]),
                            1_735_689_700,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        assert_eq!(state.new_utxos[0].height, confirmed_height);
        assert!(state.new_utxos[0].is_confirmed);
    }

    #[test]
    fn load_state_defaults_utxo_without_transaction_record_to_unconfirmed() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x43u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x7Fu8; 32]);
        let address = dashcore::Address::new(
            dashcore::Network::Testnet,
            Payload::PubkeyHash(PubkeyHash::from_byte_array([0x24u8; 20])),
        );
        let outpoint = OutPoint { txid, vout: 0 };
        let utxo = Utxo::new(
            outpoint,
            TxOut {
                value: 175_000,
                script_pubkey: address.script_pubkey(),
            },
            address,
            777,
            false,
        );

        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::strict(),
        )
        .unwrap();
        let loaded = state
            .new_utxos
            .iter()
            .find(|candidate| candidate.outpoint == outpoint)
            .expect("matching UTXO must be loaded");
        assert_eq!(loaded.height, 0);
        assert!(!loaded.is_confirmed);
    }

    #[test]
    fn load_state_tolerates_transaction_blob_txid_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x46u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let blob_txid = Txid::from_byte_array([0x82u8; 32]);
        let typed_txid = Txid::from_byte_array([0x83u8; 32]);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(blob_txid, TransactionContext::Mempool)],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }
        conn.execute(
            "UPDATE core_transactions SET txid = ?1 WHERE wallet_id = ?2",
            params![AsRef::<[u8]>::as_ref(&typed_txid), wallet_id.as_slice()],
        )
        .unwrap();

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::recovery(),
        )
        .expect("recovery mode must reconstruct from the authoritative blob");
        assert_eq!(state.records[0].txid, blob_txid);
        let on_disk: Vec<u8> = conn
            .query_row(
                "SELECT txid FROM core_transactions WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            on_disk,
            AsRef::<[u8]>::as_ref(&typed_txid),
            "a read must never rewrite the row it read"
        );
    }

    #[test]
    fn load_state_blob_height_wins_over_drifted_typed_column_in_either_scan_order() {
        for (case, typed_byte) in [0x10, 0xF0].into_iter().enumerate() {
            let mut conn = rusqlite::Connection::open_in_memory().unwrap();
            crate::sqlite::migrations::run(&mut conn).unwrap();
            let wallet_id = [0x50 + case as u8; 32];
            conn.execute(
                "INSERT INTO wallets (wallet_id, network, birth_height) \
                 VALUES (?1, 'testnet', 0)",
                params![&wallet_id[..]],
            )
            .unwrap();

            let blob_txid = Txid::from_byte_array([0x80; 32]);
            let typed_txid = Txid::from_byte_array([typed_byte; 32]);
            let confirmed_utxo = sample_utxo(blob_txid, 500, true);
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![confirmed_utxo.clone()],
                    records: vec![transaction_record(
                        blob_txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            500,
                            BlockHash::from_byte_array([0x38; 32]),
                            1_735_690_000,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
            conn.execute(
                "UPDATE core_transactions SET txid = ?1 WHERE wallet_id = ?2",
                params![AsRef::<[u8]>::as_ref(&typed_txid), wallet_id.as_slice()],
            )
            .unwrap();

            let mut stale_utxo = confirmed_utxo;
            stale_utxo.height = 100;
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    new_utxos: vec![stale_utxo],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();

            let (state, _) = load_state(
                &conn,
                &wallet_id,
                dashcore::Network::Testnet,
                &LoadCtx::recovery(),
            )
            .expect("recovery mode must still reconstruct blob-authoritative state");
            let loaded = state.new_utxos.first().expect("UTXO must load");
            assert_eq!(loaded.height, 500, "failed scan-order case {case}");
            assert!(loaded.is_confirmed);
        }
    }

    #[test]
    fn load_state_tolerates_transaction_blob_height_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x47u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x84u8; 32]);
        {
            let tx = conn.transaction().unwrap();
            apply(
                &tx,
                &wallet_id,
                &CoreChangeSet {
                    records: vec![transaction_record(
                        txid,
                        TransactionContext::InChainLockedBlock(BlockInfo::new(
                            500,
                            BlockHash::from_byte_array([0x36u8; 32]),
                            1_735_689_800,
                        )),
                    )],
                    ..Default::default()
                },
            )
            .unwrap();
            tx.commit().unwrap();
        }
        conn.execute(
            "UPDATE core_transactions SET height = 501 WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
        )
        .unwrap();

        let (state, _) = load_state(
            &conn,
            &wallet_id,
            dashcore::Network::Testnet,
            &LoadCtx::recovery(),
        )
        .expect("recovery mode must reconstruct from the authoritative blob");
        assert_eq!(state.records[0].height(), Some(500));
        let on_disk: Option<i64> = conn
            .query_row(
                "SELECT height FROM core_transactions WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            on_disk,
            Some(501),
            "a read must never rewrite the row it read"
        );
    }

    /// A txid-drifted row names a DIFFERENT transaction than the one asked
    /// for, so recovery mode must decline to answer rather than hand back
    /// the wrong record. Contrast the height-drift sibling below, where the
    /// blob stays authoritative and the record is still served.
    #[test]
    fn get_tx_record_declines_a_txid_drifted_row_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x4Bu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let blob_txid = Txid::from_byte_array([0x88u8; 32]);
        let typed_txid = Txid::from_byte_array([0x89u8; 32]);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                records: vec![transaction_record(blob_txid, TransactionContext::Mempool)],
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();
        conn.execute(
            "UPDATE core_transactions SET txid = ?1 WHERE wallet_id = ?2",
            params![AsRef::<[u8]>::as_ref(&typed_txid), wallet_id.as_slice()],
        )
        .unwrap();

        assert!(
            get_tx_record(&conn, &wallet_id, &typed_txid, &LoadCtx::recovery())
                .expect("recovery mode tolerates the drift instead of erroring")
                .is_none(),
            "the row's blob names a different transaction than the one asked \
             for; recovery must decline rather than serve the wrong record"
        );
        // The row was NOT repaired, so the blob txid still matches no row.
        assert!(
            get_tx_record(&conn, &wallet_id, &blob_txid, &LoadCtx::recovery())
                .unwrap()
                .is_none(),
            "a read must never rewrite the row it read"
        );
        // Declining is not deleting: the drifted row is still on disk under
        // its typed txid, for an operator or a later repair pass to find.
        let still_present: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM core_transactions WHERE wallet_id = ?1 AND txid = ?2",
                params![wallet_id.as_slice(), AsRef::<[u8]>::as_ref(&typed_txid)],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(still_present, 1, "a read must never delete the row it read");
    }

    #[test]
    fn get_tx_record_tolerates_blob_height_drift_in_recovery_without_repairing() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x4Cu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();

        let txid = Txid::from_byte_array([0x8Au8; 32]);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                records: vec![transaction_record(
                    txid,
                    TransactionContext::InChainLockedBlock(BlockInfo::new(
                        600,
                        BlockHash::from_byte_array([0x37u8; 32]),
                        1_735_689_900,
                    )),
                )],
                ..Default::default()
            },
        )
        .unwrap();
        tx.commit().unwrap();
        conn.execute(
            "UPDATE core_transactions SET height = 601 WHERE wallet_id = ?1",
            params![wallet_id.as_slice()],
        )
        .unwrap();

        let record = get_tx_record(&conn, &wallet_id, &txid, &LoadCtx::recovery())
            .expect("recovery mode must still serve the point read")
            .expect("blob-bearing row must return its record");
        assert_eq!(record.height(), Some(600));
        let on_disk: Option<i64> = conn
            .query_row(
                "SELECT height FROM core_transactions WHERE wallet_id = ?1",
                params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            on_disk,
            Some(601),
            "a read must never rewrite the row it read"
        );
    }

    /// `load_used_addresses` (the address-reuse-guard rehydration path called
    /// from `persister.rs`) must surface `AddressDecode` — carrying the
    /// upstream `dashcore::address::Error` — when a stored `core_utxos.script`
    /// parses as bytes but not as an address, not the context-free `BlobDecode`.
    #[test]
    fn load_used_addresses_wraps_address_error_as_address_decode() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let w = [0x99u8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&w[..]],
        )
        .unwrap();
        // A bare OP_RETURN script is well-formed bytes but not any address
        // type, so `Address::from_script` returns `UnrecognizedScript`.
        let bad_script = [0x6au8];
        conn.execute(
            "INSERT INTO core_utxos \
                (wallet_id, outpoint, value, script, spent) \
             VALUES (?1, ?2, 0, ?3, 0)",
            params![&w[..], &[0u8; 36][..], &bad_script[..]],
        )
        .unwrap();

        let err =
            load_used_addresses_with_ctx(&conn, &w, dashcore::Network::Testnet, &LoadCtx::strict())
                .expect_err("an unparseable script must be a hard error");
        assert!(
            matches!(err, WalletStorageError::AddressDecode { .. }),
            "expected AddressDecode carrying the upstream error, got {err:?}"
        );
    }

    /// An empty `script` must be refused by the WRITER, not discovered by
    /// the reader. `load()` turns every stored script back into an address,
    /// so one such row rejects the load of the entire database file — the
    /// shape migration V015 had to purge. `execute_upsert_utxo` is the only
    /// writer of `core_utxos.script`, so guarding it closes the producer.
    #[test]
    fn apply_refuses_an_empty_script_on_a_new_utxo() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x5Bu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();
        let mut utxo = sample_utxo(Txid::from_byte_array([0x5Bu8; 32]), 10, true);
        utxo.txout.script_pubkey = dashcore::ScriptBuf::from_bytes(Vec::new());
        let outpoint = utxo.outpoint;
        let cs = CoreChangeSet {
            new_utxos: vec![utxo],
            ..Default::default()
        };

        let tx = conn.transaction().unwrap();
        let err = apply(&tx, &wallet_id, &cs).expect_err("an empty script must be refused");
        match err {
            WalletStorageError::EmptyUtxoScript { outpoint: got } => {
                assert_eq!(got, outpoint, "the error must name the offending outpoint");
            }
            other => panic!("expected EmptyUtxoScript, got {other:?}"),
        }
        let rows: i64 = tx
            .query_row("SELECT COUNT(*) FROM core_utxos", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0, "the guard must refuse before binding the row");
    }

    /// The spend path synthesises a `spent = 1` row when the UTXO has no
    /// existing row, which is exactly the shape V015 had to delete. It runs
    /// through the same writer, so it must be refused on the same terms.
    #[test]
    fn apply_refuses_an_empty_script_on_a_synthetic_spent_row() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x5Cu8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();
        let mut utxo = sample_utxo(Txid::from_byte_array([0x5Cu8; 32]), 10, true);
        utxo.txout.script_pubkey = dashcore::ScriptBuf::from_bytes(Vec::new());
        let cs = CoreChangeSet {
            spent_utxos: vec![utxo],
            ..Default::default()
        };

        let tx = conn.transaction().unwrap();
        let err = apply(&tx, &wallet_id, &cs).expect_err("an empty script must be refused");
        assert!(
            matches!(err, WalletStorageError::EmptyUtxoScript { .. }),
            "the synthetic spent-row insert must be guarded too, got {err:?}"
        );
        let rows: i64 = tx
            .query_row("SELECT COUNT(*) FROM core_utxos", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 0, "the guard must refuse before binding the row");
    }

    /// Marking an EXISTING row spent never rewrites `script`, so a healthy
    /// row must still be spendable — the guard must not turn a legitimate
    /// spend into a write failure.
    #[test]
    fn apply_still_marks_an_existing_utxo_spent() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::sqlite::migrations::run(&mut conn).unwrap();
        let wallet_id = [0x5Du8; 32];
        conn.execute(
            "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
            params![&wallet_id[..]],
        )
        .unwrap();
        let utxo = sample_utxo(Txid::from_byte_array([0x5Du8; 32]), 10, true);
        let tx = conn.transaction().unwrap();
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                new_utxos: vec![utxo.clone()],
                ..Default::default()
            },
        )
        .expect("a well-formed script must still be accepted");
        apply(
            &tx,
            &wallet_id,
            &CoreChangeSet {
                spent_utxos: vec![utxo],
                ..Default::default()
            },
        )
        .expect("spending an existing row must still be accepted");
        let spent: bool = tx
            .query_row("SELECT spent FROM core_utxos", [], |row| row.get(0))
            .unwrap();
        assert!(spent, "the existing row must be marked spent");
    }

    #[test]
    fn chain_lock_height_rejects_trailing_bytes() {
        let bytes = encode_chain_lock(&sample_chain_lock(100_000)).expect("encode");
        assert_eq!(chain_lock_height(&bytes), Some(100_000));

        // A corrupt blob (valid prefix + trailing garbage) must not yield a
        // height, else it stays stuck atop later valid lower-height updates.
        let mut corrupt = bytes.clone();
        corrupt.extend_from_slice(&[0xFFu8; 4]);
        assert_eq!(chain_lock_height(&corrupt), None);
    }
}
