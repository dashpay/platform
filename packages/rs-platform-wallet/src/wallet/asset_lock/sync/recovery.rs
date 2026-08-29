//! Crash recovery and resume for asset locks.
//!
//! Contains methods for recovering asset locks from persisted state,
//! resolving status from wallet info, resuming interrupted locks,
//! and re-deriving private keys.

use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use std::collections::BTreeSet;
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::{OutPoint, Txid};
use dpp::prelude::CoreBlockHeight;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;
use crate::wallet::platform_wallet::PlatformWalletInfo;

use super::super::manager::AssetLockManager;
use super::super::orchestration::UNCONFIRMED_BROADCAST_PROOF_TIMEOUT;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

// ---------------------------------------------------------------------------
// Blocking accessor (for synchronous / evo-tool contexts)
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Blocking version of [`recover_asset_lock`](Self::recover_asset_lock).
    ///
    /// Uses `tokio::sync::RwLock::blocking_write` / `blocking_read` — must NOT
    /// be called from within a tokio async context.
    ///
    /// When `proof` is `None`, the method looks up the transaction's actual
    /// on-chain context from `ManagedWalletInfo` to determine the correct
    /// status (and constructs a `ChainAssetLockProof` if the TX is in a
    /// chain-locked block).
    #[allow(clippy::too_many_arguments)]
    pub fn recover_asset_lock_blocking(
        &self,
        tx: dashcore::Transaction,
        amount: u64,
        account_index: u32,
        funding_type: AssetLockFundingType,
        identity_index: u32,
        out_point: OutPoint,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) {
        // Phase 1 (lock held): claim the tracked-asset-lock slot and
        // pull the in-memory record out so the lookup work is
        // bounded to a single hashmap fetch.
        let in_memory_record = {
            let mut wm = self.wallet_manager.blocking_write();
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            if info.tracked_asset_locks.contains_key(&out_point) {
                return;
            }
            // Only fetch the in-memory record when we actually need
            // it (no proof was provided). Otherwise the proof we
            // already have determines the status without a lookup.
            if proof.is_none() {
                super::proof::funding_tx_record(
                    &info.core_wallet.accounts,
                    account_index,
                    &out_point.txid,
                )
            } else {
                None
            }
            // wm dropped here — release before persister I/O.
        };

        // Phase 2 (no lock held): resolve status. The persister
        // fallback's I/O (synchronous lookup, possibly an FFI
        // callback into a SwiftData query) is no longer serialized
        // behind the wallet-manager write lock.
        let (status, proof) = match proof {
            Some(ref p) => {
                let status = match p {
                    dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
                    dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
                };
                (status, proof)
            }
            None => self.resolve_status_with_in_memory(in_memory_record, account_index, &out_point),
        };

        // Phase 3 (lock held): commit the tracked-asset-lock entry.
        // We re-check `tracked_asset_locks.contains_key` because
        // another caller could have raced in during phase 2 — first
        // writer wins.
        let cs = {
            let mut wm = self.wallet_manager.blocking_write();
            let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) else {
                return;
            };
            if info.tracked_asset_locks.contains_key(&out_point) {
                return;
            }
            let lock = TrackedAssetLock {
                out_point,
                transaction: tx,
                account_index,
                funding_type,
                identity_index,
                amount,
                status,
                proof,
            };
            let mut cs = AssetLockChangeSet::default();
            cs.asset_locks.insert(out_point, (&lock).into());
            info.tracked_asset_locks.insert(out_point, lock);
            cs
            // wm dropped here — must release before queue_asset_lock_changeset
            // since the persister flush may need the wallet-manager lock
            // for other sub-changesets.
        };
        self.queue_asset_lock_changeset(cs);
    }

    /// Determine asset lock status from a pre-snapshotted in-memory
    /// record, falling back to the persister if the snapshot was
    /// `None`. The caller is responsible for dropping the
    /// wallet-manager lock between the snapshot and this call so the
    /// persister fallback's I/O isn't serialized behind it.
    ///
    /// If the TX is in a chain-locked block, returns `ChainLocked` with a
    /// constructed `ChainAssetLockProof`. If the TX has an InstantSend
    /// context, returns `InstantSendLocked` (without a proof, since we lack
    /// the IS-lock data). Otherwise defaults to `Broadcast`.
    ///
    /// Persister errors are logged at `error` and treated the same as
    /// "not found" — we'd rather classify as `Broadcast` (and let
    /// `resume_asset_lock` re-derive the proof via SPV) than abort
    /// recovery entirely. The `error` log surfaces the failure to
    /// operators since this is a one-shot path: a silently-degraded
    /// `Broadcast` classification for a genuinely chain-locked tx
    /// makes `resume_asset_lock` take the wasteful `wait_for_proof`
    /// path instead of constructing a `ChainAssetLockProof`
    /// directly, with no other signal that anything went wrong.
    fn resolve_status_with_in_memory(
        &self,
        in_memory: Option<key_wallet::managed_account::transaction_record::TransactionRecord>,
        account_index: u32,
        out_point: &OutPoint,
    ) -> (AssetLockStatus, Option<dpp::prelude::AssetLockProof>) {
        use super::proof::record_or_persister;
        use key_wallet::transaction_checking::TransactionContext;

        let record = match record_or_persister(in_memory, &self.persister, &out_point.txid) {
            Ok(opt) => opt,
            Err(e) => {
                tracing::error!(
                    txid = %out_point.txid,
                    account_index,
                    error = %e,
                    "Persister fallback failed during asset-lock status \
                     recovery; classifying as Broadcast (resume will \
                     re-derive the proof via SPV)"
                );
                None
            }
        };

        match record {
            Some(record) => match &record.context {
                TransactionContext::InChainLockedBlock(_) => {
                    if let Some(height) = record.height() {
                        use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
                        let proof = dpp::prelude::AssetLockProof::Chain(ChainAssetLockProof {
                            core_chain_locked_height: height,
                            out_point: *out_point,
                        });
                        (AssetLockStatus::ChainLocked, Some(proof))
                    } else {
                        (AssetLockStatus::ChainLocked, None)
                    }
                }
                TransactionContext::InstantSend(_) => (AssetLockStatus::InstantSendLocked, None),
                _ => (AssetLockStatus::Broadcast, None),
            },
            None => (AssetLockStatus::Broadcast, None),
        }
    }
}

// ---------------------------------------------------------------------------
// Resumable asset lock
// ---------------------------------------------------------------------------

/// Find the first outpoint of `lock`'s transaction that some **other,
/// confirmed** transaction of this wallet already spent, returning
/// `(conflicting_input, spending_txid, spender_height)`.
///
/// A hit means the asset lock is a double spend of a settled outpoint.
/// Peers reject such a transaction at the mempool boundary and relay
/// nothing back — Core has not sent BIP61 `reject` messages by default
/// since 0.17 — so while the spender stands the lock can neither be mined
/// nor IS-locked, and an unbounded proof wait on it never terminates.
/// Callers use a hit to BOUND that wait and, if it expires with the
/// conflict still standing, to report
/// [`PlatformWalletError::AssetLockInputContested`].
///
/// **A hit does not refuse the resume**, and must not be made to. Part of
/// the history this reads is rebuilt at load from persisted rows, and such
/// a record is never checked against the active chain: a wallet offline
/// while the spender's block was reorganized out restores the sighting all
/// the same, and nothing repairs it — key-wallet demotes a record only
/// when that transaction is re-observed, and a transaction absent from
/// both the replacement chain and every mempool never is. Short-circuiting
/// ahead of the (re-)broadcast and the proof wait on that evidence would
/// hand back the same verdict on every resume and every launch for a lock
/// that can in fact still confirm. Running the wait keeps the recovery
/// path open and gives live synchronization the window in which it can
/// retract the sighting.
///
/// **The gate is `is_confirmed()`, deliberately not `is_chain_locked()`.**
/// Under the default `keep-finalized-transactions = OFF` build,
/// `apply_chain_lock` evicts a record the moment a chainlock buries it and
/// retains only the txid, so a chainlocked spender essentially never
/// appears in `transaction_history()` at all: demanding ChainLock finality
/// here would make the whole screen dead code in production while leaving
/// the very failure it exists for — an old, long-settled spender — reported
/// as an unbounded proof wait.
///
/// **The verdict is always provisional, so no finality travels out of
/// here.** Reporting the conflict on a confirmed sibling is fund-safe:
/// that sibling is necessarily one of this wallet's own transactions
/// (nobody else can sign this wallet's outpoints), so the value it
/// carries is already the wallet's, and bounding the doomed wait costs
/// nothing — the lock is unrelayable for as long as the sibling stands.
/// What no evidence reachable here justifies is *refusing* the resume
/// outright (see above) or *discarding* the tracked lock: the sibling's
/// block can still reorg out, at which point a peer can replay the
/// already-broadcast lock and it can confirm — with its tracking state
/// gone, the confirmed lock's credits would be stranded.
///
/// A terminal verdict would need proof that the spender's block is an
/// ancestor of the FINALIZED chain, and the wallet layer cannot produce
/// one. A record's `is_chain_locked()` context and the wallet's applied
/// chainlock height are both promotion artifacts, not ancestry proofs:
/// promotion is height-based, and the pinned SPV chainlock manager
/// counts a missing header as a passing block-hash check, so a chainlock
/// arriving on a replacement branch ahead of its headers promotes
/// records that sit on the losing branch. Provenance does not rescue the
/// check either — under `keep-finalized-transactions` the key wallet
/// height-mutates a stale RESTORED `InBlock` record straight to
/// `InChainLockedBlock`, so a restored-record guard is bypassed by the
/// same flaw. Until SPV exposes a finalized-ancestry predicate, this
/// helper reports the conflict and nothing about its finality.
///
/// **Best-effort in one direction only.** A hit is conclusive: the
/// spender is a confirmed transaction sitting in this wallet's own
/// history, and confirmed spends of an outpoint are mutually exclusive.
/// A miss proves nothing — for the same eviction reason above, precisely
/// the oldest and therefore most likely conflicts are invisible here. A
/// lock that clears this scan may still be a double spend, and the
/// existing timeout path remains its only backstop. Do not restructure
/// callers to treat "no conflict" as proof of liveness.
///
/// Confirmation is required rather than mere presence: an unconfirmed
/// sibling that spends the same outpoint is a competing candidate, not a
/// verdict. Either transaction can still win, and the tracked lock is
/// often the one the user actually wants to push through, so a mempool
/// record must not condemn it.
fn first_confirmed_input_conflict(
    info: &PlatformWalletInfo,
    lock: &TrackedAssetLock,
) -> Option<(OutPoint, Txid, Option<CoreBlockHeight>)> {
    let lock_txid = lock.transaction.txid();
    let lock_inputs: BTreeSet<OutPoint> = lock
        .transaction
        .input
        .iter()
        .map(|input| input.previous_output)
        .collect();

    let history = info.core_wallet.transaction_history();

    // The source of truth: live transaction history. The load path restores
    // the relevant spender records into it (see the unresolved-record
    // restore in the FFI persister), so the same records serve app-launch
    // catch-up and the live session — and the same machinery keeps them
    // honest: `apply_chain_lock` promotes them when a chainlock buries
    // their block, and a reorg re-observation demotes them. An earlier
    // revision carried a separate load-time snapshot map instead; it could
    // neither promote nor demote, so its verdicts could not resolve.
    if let Some(hit) = history
        .iter()
        .filter(|record| record.txid != lock_txid && record.is_confirmed())
        .find_map(|record| {
            let conflicting_input = record
                .transaction
                .input
                .iter()
                .map(|input| input.previous_output)
                .find(|outpoint| lock_inputs.contains(outpoint))?;
            Some((conflicting_input, record.txid, record.height()))
        })
    {
        // Remember the observation before returning it. Promotion is also
        // EVICTION under the default `keep-finalized-transactions = OFF`
        // build: the moment a chainlock buries the spender's block,
        // `apply_chain_lock` removes the record this scan just read, and a
        // retry would otherwise find nothing at all and fall back into the
        // proof wait. The session memory below keeps the conflict visible
        // across that disappearance. A poisoned mutex degrades to no
        // memory, never a failure.
        let (input, spender, height) = hit;
        if let (Some(h), Ok(mut cache)) = (height, info.observed_input_conflicts.lock()) {
            cache.insert(
                input,
                crate::wallet::platform_wallet::ObservedInputConflict { spender, height: h },
            );
        }
        return Some((input, spender, height));
    }

    // No live record — consult the session memory. Two cases per
    // remembered input:
    //  * the remembered spender is back in history UNCONFIRMED: its block
    //    was reorged away and the record demoted in place — the memory is
    //    stale, retract it;
    //  * the spender has LEFT history: promotion-eviction is the only path
    //    that removes a record (a reorg demotes, nothing deletes), so the
    //    remembered in-block spend still stands and still makes the proof
    //    wait pointless. The eviction attests a height-based promotion,
    //    not finalized ancestry, so the verdict it feeds stays the
    //    provisional one — as it does everywhere else here.
    let Ok(mut cache) = info.observed_input_conflicts.lock() else {
        return None;
    };
    for input in &lock_inputs {
        let Some(observed) = cache.get(input).copied() else {
            continue;
        };
        // Same invariant as the live scan's `record.txid != lock_txid`:
        // a lock's own spend of its input is not a conflict with itself,
        // full stop. Two tracked locks sharing an input can cross-remember
        // each other, and after the winner's record is promotion-evicted a
        // resume of the WINNER must not read the memory as evidence
        // against it.
        if observed.spender == lock_txid {
            continue;
        }
        if let Some(record) = history
            .iter()
            .find(|record| record.txid == observed.spender)
        {
            if !record.is_confirmed() {
                cache.remove(input);
            }
            // A confirmed record for this spender would have been the
            // scan's hit above; nothing to add here either way.
            continue;
        }
        return Some((*input, observed.spender, Some(observed.height)));
    }
    None
}

/// The deadline a proof wait runs under once
/// [`first_confirmed_input_conflict`] has reported a sighting.
///
/// The sighting cannot refuse the wait (see that function), but it does cap
/// it at [`UNCONFIRMED_BROADCAST_PROOF_TIMEOUT`] — shortening a caller's
/// longer budget as well as replacing an unbounded one. While the spender
/// stands the lock is unrelayable, so a caller's extra minutes only delay
/// the verdict a host needs in order to explain the stalled funding
/// attempt. Nothing is given up on the recovery path the cap exists to keep
/// open: a proof that has already arrived resolves on `wait_for_proof`'s
/// first pass, straight from the record, before any deadline is consulted.
fn conflict_capped_proof_wait(timeout: Option<Duration>) -> Option<Duration> {
    Some(
        timeout.map_or(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT, |caller| {
            caller.min(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT)
        }),
    )
}

/// Seed the double-spend screen's session memory from freshly restored
/// state, before any resume runs.
///
/// The screen normally learns conflicts by reading them from history — but
/// SPV's chainlock dispatcher can win the race to the wallet lock and
/// promotion-evict a restored spender record before the first catch-up
/// resume ever reads it, leaving neither a record nor a memory: the silent
/// proof-wait hang all of this exists to prevent. Seeding at load closes
/// that window. Seeding decides only whether the screen fires at all:
/// entries seeded here surface as the same provisional verdict every
/// other sighting does.
pub(crate) fn seed_observed_input_conflicts(info: &PlatformWalletInfo) {
    let Ok(mut cache) = info.observed_input_conflicts.lock() else {
        return;
    };
    let history = info.core_wallet.transaction_history();
    for lock in info.tracked_asset_locks.values() {
        if !matches!(
            lock.status,
            AssetLockStatus::Built | AssetLockStatus::Broadcast
        ) {
            continue;
        }
        let lock_txid = lock.transaction.txid();
        for input in lock.transaction.input.iter().map(|i| i.previous_output) {
            let Some(record) = history.iter().find(|record| {
                record.txid != lock_txid
                    && record.is_confirmed()
                    && record
                        .transaction
                        .input
                        .iter()
                        .any(|i| i.previous_output == input)
            }) else {
                continue;
            };
            let Some(height) = record.height() else {
                continue;
            };
            cache.insert(
                input,
                crate::wallet::platform_wallet::ObservedInputConflict {
                    spender: record.txid,
                    height,
                },
            );
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Re-run the double-spend screen once a proof wait has expired, and
    /// render a conflict that still stands as the verdict explaining that
    /// expiry.
    ///
    /// Reading the screen AFTER the wait rather than before it is what
    /// keeps a restored sighting from condemning a lock forever. Restored
    /// block records enter history from persisted rows without ever being
    /// checked against the active chain, and no event demotes one whose
    /// block was reorganized out while the wallet was offline; a resume
    /// that refused to broadcast or wait on that evidence would report the
    /// same conflict on every launch for the rest of the lock's life.
    /// Running the wait first gives live synchronization its window: a
    /// proof that arrives settles the lock and this is never reached, and
    /// a sighting live history has retracted meanwhile leaves the caller's
    /// pre-existing outcome alone.
    ///
    /// The verdict is always the provisional
    /// [`PlatformWalletError::AssetLockInputContested`] — see
    /// [`first_confirmed_input_conflict`] for why nothing reachable here
    /// can prove the spender's block is on the finalized branch.
    ///
    /// A proof still outranks the sighting at this point, exactly as it
    /// does during the wait. The wait's expiry is a deadline race, not a
    /// statement about the lock: `wait_for_proof` re-reads the record at
    /// the top of each iteration and then selects between the notification
    /// and the deadline, so finality becoming visible while the deadline
    /// branch wins arrives one instant too late to be seen there — and a
    /// concurrent resume under a longer budget can equally have attached
    /// the proof and advanced the row while this one was expiring. Either
    /// way a sibling is still sitting in history, so the scan alone would
    /// answer "contested" for a lock that is already settled. A
    /// zero-duration proof probe runs first (a single local
    /// record/persister check — the same one the rejected-re-broadcast
    /// paths use, and no network wait), because only it can reach the
    /// persister for a record the in-memory map has evicted.
    ///
    /// Everything the verdict is then built from is read from ONE wallet
    /// snapshot: the funding transaction's own finality — its record, or the
    /// finalized-txid set a promotion that evicted the record leaves behind —
    /// the tracked row's proof and status, and the sibling scan. Splitting
    /// those reads is what let the race back in — finality landing after the
    /// probe's own lookup but before a later read left a locally final lock
    /// reported as contested, because the later read consulted only the row,
    /// which a record-only finality never advances. Holding one guard makes
    /// the three answers describe the same instant.
    ///
    /// Every suppression leaves the caller's error alone rather than
    /// replacing it: the row is left where it was, so the next resume
    /// returns the proof from the record on `wait_for_proof`'s first pass.
    async fn input_conflict_verdict(&self, out_point: &OutPoint) -> Option<PlatformWalletError> {
        if self
            .wait_for_proof(out_point, Some(Duration::ZERO))
            .await
            .is_ok()
        {
            tracing::info!(
                outpoint = %out_point,
                "resume_asset_lock: the proof wait expired, but the local record \
                 already holds finality — the input conflict is not the \
                 explanation and no contested verdict is reported"
            );
            return None;
        }

        let wm = self.wallet_manager.read().await;
        let info = wm.get_wallet_info(&self.wallet_id)?;
        let lock = info.tracked_asset_locks.get(out_point)?;
        // Finality that landed during the probe. The record can carry it
        // while the row still says `Broadcast` with no proof attached —
        // `LockNotifyHandler` wakes waiters without advancing the status —
        // so the row check below cannot stand in for this one.
        //
        // Asked of the record AND of the account's finalized-txid set,
        // because the promotion that grants finality is also what takes the
        // record away: under the default `keep-finalized-transactions`
        // configuration a chainlocked record is evicted and only its txid
        // retained, so a chainlock landing between the probe and this
        // snapshot leaves nothing for a record lookup to find. Reading only
        // the record there condemned a locally final lock on the strength of
        // a sibling the promotion had not touched.
        {
            use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
            let wallet_chain_lock_height = info
                .core_wallet
                .metadata
                .last_applied_chain_lock
                .as_ref()
                .map(|chain_lock| chain_lock.block_height);
            let networks_match = info.network() == self.sdk.network;
            let record_is_final = super::proof::funding_tx_record(
                &info.core_wallet.accounts,
                lock.account_index,
                &out_point.txid,
            )
            .is_some_and(|record| {
                super::proof::record_holds_local_finality(
                    &record,
                    wallet_chain_lock_height,
                    networks_match,
                )
            });
            if record_is_final
                || super::proof::funding_tx_is_finalized(
                    &info.core_wallet.accounts,
                    lock.account_index,
                    &out_point.txid,
                )
            {
                tracing::info!(
                    outpoint = %out_point,
                    "resume_asset_lock: the proof wait expired, but the funding \
                     transaction reached finality while it was being read — the \
                     input conflict is not the explanation and no contested \
                     verdict is reported"
                );
                return None;
            }
        }
        // The screen speaks only for the two proof-less statuses
        // (`resume_asset_lock` screens on exactly those), and a row carrying
        // a proof is settled by evidence this scan cannot outrank — see the
        // status match in `resume_asset_lock`.
        if lock.proof.is_some()
            || !matches!(
                lock.status,
                AssetLockStatus::Built | AssetLockStatus::Broadcast
            )
        {
            tracing::info!(
                outpoint = %out_point,
                status = ?lock.status,
                has_proof = lock.proof.is_some(),
                "resume_asset_lock: the proof wait expired, but the tracked lock \
                 has since been settled — no contested verdict is reported"
            );
            return None;
        }
        let (input, spent_by, height) = first_confirmed_input_conflict(info, lock)?;
        tracing::warn!(
            outpoint = %out_point,
            %input,
            %spent_by,
            ?height,
            "resume_asset_lock: the proof wait expired with the input conflict \
             still standing; reporting it as the provisional verdict"
        );
        Some(PlatformWalletError::AssetLockInputContested {
            out_point: *out_point,
            input,
            spent_by,
            height,
        })
    }

    /// Resume a tracked asset lock from whatever stage it's at.
    ///
    /// Looks up the tracked lock by `txid`, then:
    ///
    /// - **`Built`**: re-broadcasts the transaction and waits for a proof.
    /// - **`Broadcast`**: waits for a proof.
    /// - **`InstantSendLocked` / `ChainLocked`**: uses the existing proof
    ///   (upgrading a stale IS-lock to a ChainLock proof if necessary).
    ///
    /// After obtaining the proof, advances the tracked lock status and
    /// re-derives the one-time credit-output derivation path from the
    /// wallet's funding-account address pool.
    ///
    /// Returns `(proof, derivation_path)` ready for use in identity
    /// registration or top-up via the `_with_signer` SDK methods. The
    /// caller passes `derivation_path` to the same signer used for the
    /// build phase when the credit output is later consumed on Platform.
    ///
    /// `timeout` is `Option<Duration>` and is only consulted when the lock
    /// still needs a proof (`Built` / `Broadcast`, or the defensive
    /// proof-less `RecoveredFromChain` fallback). For `InstantSendLocked` /
    /// `ChainLocked` the proof already exists and no wait happens, so the
    /// value is moot.
    ///
    /// `None` requests an unbounded wait, and gets one **only** where this
    /// call obtained positive evidence the transaction is on the network:
    /// the `Built` arm whose re-broadcast returned `Ok`. Every other
    /// proof-waiting path substitutes
    /// [`UNCONFIRMED_BROADCAST_PROOF_TIMEOUT`], because the alternative is a
    /// `Notify` loop that never terminates under the FFI's
    /// `runtime().block_on(...)` — a permanently pinned host thread rather
    /// than a late answer. Expiry leaves the tracked row untouched, so the
    /// next resume picks up a proof that arrives later straight from the
    /// record; on the `Broadcast` arm it is reported as
    /// [`PlatformWalletError::TransactionBroadcastUnconfirmed`].
    ///
    /// A `Built` / `Broadcast` lock is screened by
    /// [`first_confirmed_input_conflict`], and a hit never refuses the
    /// resume. It withdraws the unbounded wait — a double spend no peer
    /// relays is not evidence the transaction is on the network — and the
    /// verdict is read afterwards by [`Self::input_conflict_verdict`]: a
    /// proof that arrives during the bounded wait settles the lock
    /// normally, and a conflict the wait did not clear is reported as the
    /// provisional [`PlatformWalletError::AssetLockInputContested`], which
    /// keeps the lock tracked for a later retry. Blocking the
    /// broadcast-and-wait outright is what this evidence does NOT support:
    /// the screen also reads records the load path rebuilt from persisted
    /// rows, which no event can demote once their block has been
    /// reorganized out behind an offline wallet, so a pre-emptive refusal
    /// would return the same verdict on every launch for a lock that is
    /// free to confirm. Proving the spender's block is on the finalized
    /// branch would take an ancestry predicate the wallet does not have —
    /// chainlock contexts and applied chainlock heights are promotion
    /// artifacts, not ancestry proofs — so the terminal
    /// [`PlatformWalletError::AssetLockInputConflict`] is never
    /// constructed here. A conflict that persists across sessions is in
    /// practice permanent, but acting on that (discarding the tracked
    /// lock) is host and user policy; the SDK does not license it
    /// unilaterally on this evidence. The screen is one-sided — read its
    /// docs before treating a clean pass as evidence the lock is alive.
    pub async fn resume_asset_lock(
        &self,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath), PlatformWalletError> {
        tracing::info!(outpoint = %out_point, ?timeout, "resume_asset_lock: entered");

        // 1. Look up the tracked lock — snapshot the fields we need.
        let (tx, status, existing_proof, account_index, input_conflict) = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let tracked_count = info.tracked_asset_locks.len();
            let lock = info.tracked_asset_locks.get(out_point).ok_or_else(|| {
                tracing::warn!(
                    outpoint = %out_point,
                    tracked_count,
                    "resume_asset_lock: asset lock not in tracked_asset_locks map"
                );
                PlatformWalletError::AssetLockNotTracked(*out_point)
            })?;
            tracing::info!(
                outpoint = %out_point,
                status = ?lock.status,
                has_proof = lock.proof.is_some(),
                account_index = lock.account_index,
                "resume_asset_lock: lock looked up"
            );
            // Only the two proof-less statuses are candidates. A lock
            // carrying an IS/Chain proof, a `RecoveredFromChain` entry
            // (reconstructed from a record the chain itself accepted), and
            // a `Consumed` tombstone are all settled by evidence stronger
            // than this scan; re-classifying one of them as a double spend
            // on the strength of an unrelated history record would
            // invalidate a lock the network already honoured.
            let input_conflict = match lock.status {
                AssetLockStatus::Built | AssetLockStatus::Broadcast => {
                    first_confirmed_input_conflict(info, lock)
                }
                AssetLockStatus::InstantSendLocked
                | AssetLockStatus::ChainLocked
                | AssetLockStatus::RecoveredFromChain
                | AssetLockStatus::Consumed => None,
            };
            (
                lock.transaction.clone(),
                lock.status.clone(),
                lock.proof.clone(),
                lock.account_index,
                input_conflict,
            )
        };

        // A sighting does NOT stop the resume. It cannot: the screen reads
        // records the load path rebuilt from persisted rows, and a block
        // record restored that way has never been checked against the
        // active chain. A wallet that was offline while the spender's block
        // was reorganized out restores the sighting anyway, and nothing
        // repairs it — key-wallet demotes a record only when that same
        // transaction is re-observed, and a transaction absent from both
        // the replacement chain and every mempool is never observed again.
        // Refusing the (re-)broadcast and the proof wait on that evidence
        // would return the same verdict on every resume and every launch
        // for a lock that is in fact free to confirm.
        //
        // So the sighting only bounds the wait (below): it withdraws the
        // unbounded one, because it is not evidence that the transaction is
        // on the network. The verdict is read afterwards, from whatever
        // live synchronization left behind while the wait ran — a proof
        // that arrives settles the lock outright, and a conflict the wait
        // did not clear becomes the error explaining the expiry.
        if let Some((input, spent_by, height)) = input_conflict {
            tracing::warn!(
                outpoint = %out_point,
                %input,
                %spent_by,
                ?height,
                "resume_asset_lock: asset lock double-spends an outpoint a \
                 confirmed transaction of this wallet already consumed; \
                 resuming under a bounded wait rather than refusing, since \
                 the sighting may be restored evidence no live event can \
                 retract"
            );
        }

        // 2. Resume from the current status.
        let proof = match status {
            AssetLockStatus::Built => {
                // Re-broadcast and wait for proof.
                //
                // No verdict this broadcaster can return ends the resume by
                // itself. `MaybeSent`
                // means the outcome is unknown — and for a lock stuck at
                // `Built` that is the expected answer when the app died
                // between a successful broadcast and this status advance:
                // the tx is in a mempool (or mined) and every re-broadcast
                // reports the same ambiguity. Failing on it left the lock at
                // `Built` forever, so each recovery pass repeated the same
                // broadcast and the same abort, and the top-up never
                // completed. Advancing to `Broadcast` and waiting matches
                // what the `Broadcast` arm below already does with the
                // identical signal.
                //
                // `MaybeSent` is however ALSO what the broadcaster reports
                // for a genuinely rejected transaction: `DapiBroadcaster`
                // classifies every failure that way by construction, and the
                // SPV broadcaster only reaches `Rejected` on `NotConnected`.
                // So the advance above cannot be read as evidence the tx is
                // live, and the proof wait that follows it must not be the
                // unbounded one — at the `resume_asset_lock(.., None)`
                // production call sites that would turn a prompt broadcast
                // failure into a permanent hang. Bound it, and translate the
                // expiry back into the `TransactionBroadcastUnconfirmed` the
                // caller used to get immediately.
                //
                // A DEFINITE `Rejected` is scoped to the attempt that
                // produced it, exactly as on the `Broadcast` arm below: with
                // the production `SpvBroadcaster` it is reachable only from
                // an unstarted client and dash-spv's zero-connected-peers
                // check, so it means "*this* send never left the device". It
                // is not a statement about the row. A lock sits at `Built`
                // after a SUCCESSFUL broadcast too — the app killed between
                // the send and the status advance is the very case this arm
                // exists for — so the original may be in a mempool or already
                // mined, and the record may already carry its proof. The
                // rejection is therefore handled here rather than returned:
                // probe the local record once without waiting, and where a
                // conflict was sighted go on to the bounded wait, which is
                // the only path that can produce the sighting's verdict.
                //
                // What must NOT happen is the raw conversion. `Rejected`
                // becomes `TransactionBroadcast`, the FFI's definite-
                // rejection code 26, whose contract is that Core rejected the
                // transaction, the inputs' reservation was released, and a
                // rebuild is safe. Only the initial build path performs that
                // untrack-and-release; the resume keeps both the row and its
                // reservation, so a host honouring code 26 here would rebuild
                // from other UTXOs and create a SECOND asset lock beside a
                // possibly-live one. The non-terminal
                // `TransactionBroadcastUnconfirmed` is the contract that
                // matches what this arm actually knows: outcome unknown,
                // inputs still reserved, do not retry.
                let mut local_proof = None;
                let mut maybe_sent_reason = None;
                let mut undispatched = None;
                match self.broadcaster.broadcast(&tx).await {
                    Ok(_) => {}
                    Err(BroadcastError::MaybeSent { reason }) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            reason = %reason,
                            "resume_asset_lock: re-broadcast of a Built lock returned an \
                             unknown outcome (the network may already hold this tx, or may \
                             have rejected it — the broadcaster cannot tell); advancing to \
                             Broadcast and waiting for proof under a bounded timeout"
                        );
                        maybe_sent_reason = Some(reason);
                    }
                    Err(rejected @ BroadcastError::Rejected { .. }) => {
                        match self.wait_for_proof(out_point, Some(Duration::ZERO)).await {
                            Ok(proof) => {
                                tracing::info!(
                                    outpoint = %out_point,
                                    error = %rejected,
                                    "resume_asset_lock: re-broadcast of a Built lock was \
                                     rejected before dispatch, but the local record already \
                                     holds finality — completing the resume from the local \
                                     proof"
                                );
                                local_proof = Some(proof);
                            }
                            Err(probe_err) => {
                                // No proof, and this attempt never left the
                                // device. Without a sighting there is nothing
                                // this call can still learn — the `Broadcast`
                                // arm returns the same unknown-outcome error
                                // here, and for the same reason.
                                if input_conflict.is_none() {
                                    tracing::warn!(
                                        outpoint = %out_point,
                                        error = %rejected,
                                        probe = %probe_err,
                                        "resume_asset_lock: re-broadcast of a Built lock \
                                         was rejected before dispatch and no local proof \
                                         exists — this attempt proves nothing about an \
                                         earlier send; leaving the row tracked at Built \
                                         and failing the resume as an unknown outcome"
                                    );
                                    return Err(
                                        PlatformWalletError::TransactionBroadcastUnconfirmed(
                                            format!(
                                                "asset lock {out_point} remains tracked at \
                                                 Built after the re-broadcast was rejected \
                                                 before dispatch; an earlier broadcast may \
                                                 still be on the network: {rejected}"
                                            ),
                                        ),
                                    );
                                }
                                tracing::warn!(
                                    outpoint = %out_point,
                                    error = %rejected,
                                    probe = %probe_err,
                                    "resume_asset_lock: re-broadcast of a Built lock was \
                                     rejected before dispatch with an input conflict \
                                     sighted; entering the bounded proof wait, since the \
                                     sighting bounds the wait rather than replacing it and \
                                     its verdict is only readable afterwards"
                                );
                                undispatched = Some(rejected.to_string());
                            }
                        }
                    }
                }
                let proof = if let Some(proof) = local_proof {
                    proof
                } else {
                    // The status advance belongs to a send that actually
                    // dispatched. An attempt rejected before dispatch leaves
                    // the row exactly where it was, so the next resume
                    // re-sends the transaction instead of dropping into the
                    // `Broadcast` arm's wait for a send that never happened.
                    if undispatched.is_none() {
                        let cs = self
                            .advance_asset_lock_status(out_point, AssetLockStatus::Broadcast, None)
                            .await?;
                        self.queue_asset_lock_changeset(cs);
                    }
                    // An ambiguous re-broadcast and a conflict sighting both
                    // deny this call positive evidence that the transaction is
                    // on the network, and an unbounded wait without that
                    // evidence pins the host thread rather than merely delaying
                    // an answer. A clean broadcast with no sighting keeps the
                    // caller's `None` exactly as before.
                    let bounded = if input_conflict.is_some() {
                        conflict_capped_proof_wait(timeout)
                    } else if maybe_sent_reason.is_some() {
                        timeout.or(Some(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT))
                    } else {
                        timeout
                    };
                    match self.wait_for_proof(out_point, bounded).await {
                        Ok(proof) => proof,
                        Err(expiry @ PlatformWalletError::FinalityTimeout(_)) => {
                            // The wait has now given live synchronization its
                            // window, so the screen is re-read and what it says
                            // NOW decides. A conflict it still reports is the
                            // honest explanation for the expiry; one it has
                            // retracted meanwhile leaves the pre-existing
                            // outcome untouched.
                            if let Some(contested) = self.input_conflict_verdict(out_point).await {
                                return Err(contested);
                            }
                            // A caller who chose its own bound is left exactly
                            // as it was, `FinalityTimeout` and all. Every
                            // re-typing below exists to keep an UNBOUNDED wait
                            // from hanging on a signal that cannot arrive, and
                            // that reason is absent the moment the caller
                            // named a deadline: the shielded seed pool reads
                            // `FinalityTimeout` as a pacing signal and resumes
                            // the lock later, so substituting a do-not-retry
                            // error for the bound it asked for would break a
                            // working flow to fix an unrelated one. The check
                            // comes FIRST because both translations below are
                            // reachable under an explicit timeout.
                            if timeout.is_some() {
                                return Err(expiry);
                            }
                            // The wait ran on a send that never dispatched, so
                            // the outcome of any earlier one is still unknown
                            // and the row is still tracked and reserved. That
                            // is the unknown-outcome contract, never the
                            // definite-rejection code the raw conversion would
                            // have produced.
                            if let Some(rejection) = undispatched {
                                return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                                    format!(
                                        "asset lock {} remains tracked at Built after the \
                                         re-broadcast was rejected before dispatch and no \
                                         InstantSend/ChainLock proof arrived within {:?}; an \
                                         earlier broadcast may still be on the network: {}",
                                        out_point, UNCONFIRMED_BROADCAST_PROOF_TIMEOUT, rejection
                                    ),
                                ));
                            }
                            // Ambiguous re-broadcast AND an unbounded wait: the
                            // only combination that can hang forever. Its expiry
                            // is translated back into the broadcast error the
                            // caller used to get immediately.
                            return Err(match &maybe_sent_reason {
                                Some(reason) => {
                                    PlatformWalletError::TransactionBroadcastUnconfirmed(format!(
                                        "asset lock {} was re-broadcast with an unknown \
                                         outcome and no InstantSend/ChainLock proof arrived \
                                         within {:?}: {}",
                                        out_point, UNCONFIRMED_BROADCAST_PROOF_TIMEOUT, reason
                                    ))
                                }
                                None => expiry,
                            });
                        }
                        Err(e) => return Err(e),
                    }
                };
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::Broadcast => {
                // Defensive re-broadcast, then wait for proof. A lock can
                // sit at `Broadcast` across app restarts long enough for
                // its funding tx to be evicted from every mempool (Core's
                // default `-mempoolexpiry` is two weeks), or the original
                // broadcast may have reached no peers at all (SPV
                // connectivity gap). Once no node holds the tx, no IS/CL
                // proof can ever arrive and the wait below can only run out
                // its bound. A re-broadcast revives an evicted/undelivered
                // tx, so it is worth attempting before every wait.
                //
                // Best-effort for the AMBIGUOUS verdict only: unlike the
                // `Built` arm, this tx was already broadcast once (that's
                // what `Broadcast` means), so it may still be in a mempool or
                // already mined — in which case the network reports "already
                // known" / "already in block chain", which the broadcaster
                // cannot distinguish from a real rejection and reports as
                // `MaybeSent`. We log that and proceed to `wait_for_proof`
                // rather than failing the resume on a tx that is actually
                // fine. If the tx really was mined, `wait_for_proof` resolves
                // immediately from the SPV/persisted record.
                //
                // A DEFINITE `Rejected` ends the resume early — but it says
                // NOTHING about the row, and must not be read as one. In
                // fact the row's RECORD may already hold the answer: a lock
                // can sit at `Broadcast` while its transaction record
                // carries an IS lock or a chain-locked context, because
                // finality that arrives with no waiter active enriches the
                // record without advancing the tracked status
                // (`LockNotifyHandler` only wakes waiters, and
                // `enrich_from_record` upgrades only chain-locked records
                // on scan paths). So before surfacing the rejection, probe
                // the record once, without waiting — `wait_for_proof` with
                // a zero bound performs exactly one local record/persister
                // check and expires before touching the network. On the
                // canonical trigger (`catchUpStuckAssetLocks` resuming
                // rows at launch before SPV connects) that probe is the
                // difference between completing an already-final lock
                // entirely offline and failing it every launch until
                // connectivity returns.
                //
                // `Rejected` is scoped to the attempt that produced it. With
                // the production `SpvBroadcaster` it is reachable from
                // exactly two places, an unstarted client and dash-spv's
                // zero-connected-peers check (`spv/runtime.rs`), so it means
                // "*this* send never left the device" — not "the transaction
                // is not on the network". The ORIGINAL broadcast that put
                // this row at `Broadcast` happened in an earlier process,
                // possibly days ago, and its outcome is untouched by a
                // re-broadcast that never dispatched.
                //
                // So there is no untrack here. `catchUpStuckAssetLocks` runs
                // on every wallet load, selects `statusRaw < 2` (which
                // includes `Broadcast` = 1) and has no SPV-connected gate, so
                // dropping the row on this verdict deleted tracking for a
                // possibly-mined asset lock on an ordinary offline relaunch —
                // and reconstruction only re-inserts on a FRESH detection
                // event, which an already-recorded mined transaction never
                // produces again. The row stays exactly as it was; a later
                // resume, once SPV is connected, re-broadcasts and resolves
                // it normally.
                //
                // The error type follows the same logic. `Rejected` converts
                // to `TransactionBroadcast`, which the FFI surfaces as the
                // definite-rejection code: it promises the host that Core
                // rejected the transaction, that its inputs' reservation was
                // released, and that rebuilding is therefore safe. None of
                // that holds here — the row and its reservation are kept
                // precisely because the original may still confirm, so a host
                // honouring that contract would rebuild from other UTXOs and
                // create a SECOND asset lock beside a live one. The resume
                // fails as `TransactionBroadcastUnconfirmed` instead: outcome
                // unknown, inputs still reserved, do not retry.
                //
                // Nor is there a state on this path where non-dispatch of the
                // original send IS provable: a row can sit at `Built` after a
                // successful broadcast too (app killed between the send and
                // the status advance), which is precisely why the `Built` arm
                // above also only surfaces the error and leaves its row alone.
                let mut local_proof = None;
                if let Err(e) = self.broadcaster.broadcast(&tx).await {
                    if matches!(e, BroadcastError::Rejected { .. }) {
                        match self.wait_for_proof(out_point, Some(Duration::ZERO)).await {
                            Ok(proof) => {
                                tracing::info!(
                                    outpoint = %out_point,
                                    error = %e,
                                    "resume_asset_lock: defensive re-broadcast of a \
                                     Broadcast-status lock was rejected, but the \
                                     local record already holds finality — \
                                     completing the resume from the local proof"
                                );
                                local_proof = Some(proof);
                            }
                            Err(probe_err) => {
                                tracing::warn!(
                                    outpoint = %out_point,
                                    error = %e,
                                    probe = %probe_err,
                                    "resume_asset_lock: defensive re-broadcast of a \
                                     Broadcast-status lock was rejected before \
                                     dispatch and no local proof exists — this \
                                     attempt never left the device, which proves \
                                     nothing about the original broadcast; leaving \
                                     the row tracked at Broadcast and failing the \
                                     resume as an unknown outcome"
                                );
                                return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                                    format!(
                                        "asset lock {out_point} remains tracked after the \
                                         defensive re-broadcast was rejected before \
                                         dispatch; the original broadcast may still be on \
                                         the network: {e}"
                                    ),
                                ));
                            }
                        }
                    } else {
                        tracing::debug!(
                            outpoint = %out_point,
                            error = %e,
                            "resume_asset_lock: defensive re-broadcast of a \
                             Broadcast-status lock returned an unknown outcome (likely \
                             already in a mempool or mined); proceeding to wait \
                             for proof"
                        );
                    }
                }
                // Bounded like the `Built` arm, and for the same reason. This
                // arm is only entered on a RESUME, i.e. for a transaction
                // whose earlier broadcast window already failed to produce a
                // proof — so "finality is only a matter of time", the premise
                // that justifies the unbounded `wait_for_proof(None)` on the
                // initial funding path, does not hold here. The `None` this
                // receives from `resume_asset_lock(.., None)` (and from the
                // FFI's `timeout_secs == 0`) drove an unbounded `Notify` loop
                // under `runtime().block_on(...)`, which pins the calling host
                // thread permanently rather than merely delaying a result.
                //
                // The bound costs nothing: the row is left at `Broadcast`, so
                // a proof that lands after the expiry is picked up by the very
                // next resume — `wait_for_proof` returns it on its first
                // iteration, straight from the record, without waiting at all.
                //
                // Callers that supplied their own timeout keep their exact
                // semantics, `FinalityTimeout` and all: `or` is the identity
                // on `Some`, and the re-typing below is gated on the caller
                // having asked for an unbounded wait. The shielded seed pool
                // reads `FinalityTimeout` as a pacing signal, so re-typing it
                // for everyone would break a working flow to fix another.
                let proof = if let Some(proof) = local_proof {
                    proof
                } else {
                    let bounded = if input_conflict.is_some() {
                        conflict_capped_proof_wait(timeout)
                    } else {
                        timeout.or(Some(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT))
                    };
                    match self.wait_for_proof(out_point, bounded).await {
                        Ok(proof) => proof,
                        Err(expiry @ PlatformWalletError::FinalityTimeout(_)) => {
                            // Same reading as the `Built` arm: the wait gave
                            // live synchronization its window, so the screen
                            // is re-read afterwards and a conflict that still
                            // stands explains the expiry.
                            if let Some(contested) = self.input_conflict_verdict(out_point).await {
                                return Err(contested);
                            }
                            if timeout.is_some() {
                                return Err(expiry);
                            }
                            let reason = format!(
                                "asset lock {} is tracked as broadcast but no \
                                 InstantSend/ChainLock proof arrived within {:?}; the \
                                 lock remains tracked and resumable",
                                out_point, UNCONFIRMED_BROADCAST_PROOF_TIMEOUT
                            );
                            return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                                reason,
                            ));
                        }
                        Err(e) => return Err(e),
                    }
                };
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::InstantSendLocked | AssetLockStatus::ChainLocked => {
                // Already have a proof — validate / upgrade if stale.
                let proof = existing_proof.ok_or_else(|| {
                    PlatformWalletError::AssetLockProofWait(format!(
                        "Asset lock {} is marked as {:?} but has no proof attached",
                        out_point, status
                    ))
                })?;
                self.validate_or_upgrade_proof(proof, account_index, out_point)
                    .await?
            }
            AssetLockStatus::RecoveredFromChain => {
                // Reconstructed from a chain-locked record after a
                // restore — Platform-side consumption is unknown. An
                // explicit resume is allowed to try consuming it:
                // Platform rejects an already-spent outpoint with a
                // typed error, and a genuinely unspent lock is real
                // recoverable value. The reconstruction path only
                // assigns this status alongside a chain proof — at
                // creation for records already finalized, or via
                // `enrich_from_record` when finality arrives later
                // (non-final detections enter as
                // `Broadcast`/`InstantSendLocked` and take those arms,
                // re-broadcast included, until then) — so the proof is
                // present by construction; the `None` arm is a
                // defensive fallback for a row whose persisted proof
                // was lost, and its wait resolves from the already
                // chain-locked record rather than blocking on new
                // network events.
                //
                // That last part is a "by construction" argument, and it
                // holds only while the chain-locked record is still
                // reachable — the same lost-state accident that produced a
                // proof-less `RecoveredFromChain` row could equally have
                // taken the record with it, and then the wait has nothing
                // to resolve from and blocks forever on `Notify`. The bound
                // is free where the argument holds (`wait_for_proof` returns
                // from the record on its first iteration, before any deadline
                // is consulted) and closes the case where it doesn't, so it
                // is applied for uniformity with the two arms above. No
                // re-typing here: nothing was broadcast on this path, so
                // `FinalityTimeout` is the honest verdict.
                match existing_proof {
                    Some(proof) => {
                        self.validate_or_upgrade_proof(proof, account_index, out_point)
                            .await?
                    }
                    None => {
                        let proof = self
                            .wait_for_proof(
                                out_point,
                                timeout.or(Some(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT)),
                            )
                            .await?;
                        self.validate_or_upgrade_proof(proof, account_index, out_point)
                            .await?
                    }
                }
            }
            AssetLockStatus::Consumed => {
                // Terminal tombstone — the asset lock was already
                // burned by a successful identity registration / top-up.
                // Retaining this state makes the typed distinction from
                // an unknown outpoint available before and after restart.
                return Err(PlatformWalletError::AssetLockAlreadyConsumed(*out_point));
            }
        };

        // 3. Advance status and attach proof. A `RecoveredFromChain`
        // entry keeps its status: the resume proved (or refreshed)
        // Core-side finality, which that status already asserts — it
        // proved nothing new about Platform-side consumption, so
        // advancing into `InstantSendLocked`/`ChainLocked` (which
        // consumers read as "in flight") would silently re-enter the
        // pending window and resurrect the false-"Pending" rendering
        // on every restored lock whose resume didn't end in a spend.
        // Consumption is recorded separately (`consume_asset_lock`)
        // when the credit output actually lands on Platform.
        let new_status = if status == AssetLockStatus::RecoveredFromChain {
            AssetLockStatus::RecoveredFromChain
        } else {
            match &proof {
                dpp::prelude::AssetLockProof::Instant(_) => AssetLockStatus::InstantSendLocked,
                dpp::prelude::AssetLockProof::Chain(_) => AssetLockStatus::ChainLocked,
            }
        };
        let cs = self
            .advance_asset_lock_status(out_point, new_status, Some(proof.clone()))
            .await?;
        self.queue_asset_lock_changeset(cs);

        // 4. Re-derive the one-time credit-output derivation path.
        let path = {
            let wm = self.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let lock = info
                .tracked_asset_locks
                .get(out_point)
                .ok_or_else(|| PlatformWalletError::AssetLockNotTracked(*out_point))?;
            self.rederive_credit_output_path(lock).await?
        };

        Ok((proof, path))
    }

    /// Re-derive the one-time credit-output **derivation path** for a
    /// tracked asset lock.
    ///
    /// The credit output address was generated from a funding account
    /// (identity registration, top-up, etc.). This method finds that
    /// address in the funding account's address pool and retrieves
    /// its derivation path — the path is what the caller hands to a
    /// `key_wallet::signer::Signer` when later consuming the credit
    /// output on Platform.
    ///
    /// Previously this method derived the actual private key from the
    /// wallet's root xpriv; that path is no longer reachable for
    /// `ExternalSignable` wallets (the root key isn't in-process) and
    /// the signer-based architecture doesn't need it — the signer
    /// owns derivation end-to-end.
    async fn rederive_credit_output_path(
        &self,
        lock: &TrackedAssetLock,
    ) -> Result<DerivationPath, PlatformWalletError> {
        use dashcore::blockdata::transaction::special_transaction::TransactionPayload;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        // 1. Extract the credit output from the AssetLockPayload.
        let payload = lock
            .transaction
            .special_transaction_payload
            .as_ref()
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(
                    "Transaction has no special transaction payload".to_string(),
                )
            })?;
        let asset_lock_payload = match payload {
            TransactionPayload::AssetLockPayloadType(p) => p,
            _ => {
                return Err(PlatformWalletError::AssetLockTransaction(
                    "Transaction payload is not an AssetLockPayload".to_string(),
                ));
            }
        };
        let credit_output = asset_lock_payload.credit_outputs.first().ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(
                "AssetLockPayload has no credit outputs".to_string(),
            )
        })?;

        // 2. Get the address from the credit output's script_pubkey.
        let address = DashAddress::from_script(&credit_output.script_pubkey, self.sdk.network)
            .map_err(|e| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Failed to derive address from credit output script: {}",
                    e
                ))
            })?;

        // 3. Find the derivation path in the funding account and derive key under a single lock.
        let wm = self.wallet_manager.read().await;
        let info = wm
            .get_wallet_info(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let wi = &info.core_wallet;
        let funding_account = match lock.funding_type {
            AssetLockFundingType::IdentityRegistration => {
                wi.accounts.identity_registration.as_ref()
            }
            AssetLockFundingType::IdentityTopUp => {
                wi.accounts.identity_topup.get(&lock.identity_index)
            }
            AssetLockFundingType::IdentityTopUpNotBound => {
                wi.accounts.identity_topup_not_bound.as_ref()
            }
            AssetLockFundingType::IdentityInvitation => wi.accounts.identity_invitation.as_ref(),
            AssetLockFundingType::AssetLockAddressTopUp => {
                wi.accounts.asset_lock_address_topup.as_ref()
            }
            AssetLockFundingType::AssetLockShieldedAddressTopUp => {
                wi.accounts.asset_lock_shielded_address_topup.as_ref()
            }
        };

        let funding_account = funding_account.ok_or_else(|| {
            PlatformWalletError::AssetLockTransaction(format!(
                "Funding account {:?} not found for re-derivation",
                lock.funding_type
            ))
        })?;

        let derivation_path = funding_account
            .address_derivation_path(&address)
            .ok_or_else(|| {
                PlatformWalletError::AssetLockTransaction(format!(
                    "Address {} not found in funding account {:?}",
                    address, lock.funding_type
                ))
            })?;

        Ok(derivation_path)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use dashcore::bls_sig_utils::BLSSignature;
    use dashcore::ephemerealdata::chain_lock::ChainLock;
    use dashcore::hashes::Hash;
    use dashcore::prelude::CoreBlockHeight;
    use dashcore::{BlockHash, Network, OutPoint, Transaction, TxIn, Txid};
    use key_wallet::account::account_collection::AccountCollection;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::{Account, AccountType};
    use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
    use key_wallet::managed_account::transaction_record::{
        TransactionDirection, TransactionRecord,
    };
    use key_wallet::transaction_checking::{BlockInfo, TransactionContext, TransactionType};
    use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
    use key_wallet::wallet::Wallet;
    use key_wallet_manager::WalletManager;
    use tokio::sync::{Notify, RwLock};

    use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
    use crate::changeset::{
        ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    };
    use crate::error::PlatformWalletError;
    use crate::test_support::{
        funded_wallet_manager, AlwaysMaybeSentBroadcaster, AlwaysRejectedBroadcaster,
    };
    use crate::wallet::asset_lock::manager::AssetLockManager;
    use crate::wallet::asset_lock::tracked::{AssetLockStatus, TrackedAssetLock};
    use crate::wallet::core::WalletGeneration;
    use crate::wallet::identity::IdentityManager;
    use crate::wallet::persister::WalletPersister;
    use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
    use crate::AssetLockFundingType;

    /// Persistence stub that records every stored changeset so the test
    /// can replay the registration rounds the way the FFI load path does.
    #[derive(Default)]
    struct RecordingPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    /// Captures the exact transaction passed to the resumed `Built` branch.
    /// Recovery must never rebuild a replacement transaction/outpoint.
    ///
    /// `reject` reproduces the production `SpvBroadcaster` before it is
    /// connected: the send is recorded (it was attempted) and then refused
    /// with the DEFINITE `Rejected`, which on that broadcaster means only
    /// that this attempt never left the device.
    #[derive(Default)]
    struct RecordingBroadcaster {
        transactions: Mutex<Vec<Transaction>>,
        reject: bool,
    }

    #[async_trait]
    impl TransactionBroadcaster for RecordingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.transactions
                .lock()
                .expect("recording broadcaster mutex")
                .push(transaction.clone());
            if self.reject {
                return Err(BroadcastError::Rejected {
                    reason: "simulated pre-send rejection".to_string(),
                });
            }
            Ok(transaction.txid())
        }
    }

    impl PlatformWalletPersistence for RecordingPersistence {
        fn store(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            self.stored
                .lock()
                .expect("recording persistence mutex")
                .push(changeset);
            Ok(())
        }

        fn flush(
            &self,
            _wallet_id: crate::wallet::platform_wallet::WalletId,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    /// Persistence stub that mutates the wallet from inside the N-th
    /// persister-backed record lookup, placing a change at an interleaving
    /// no test can otherwise reach.
    ///
    /// `wait_for_proof` reads the in-memory record under the wallet lock,
    /// DROPS that guard, and only then falls back to the persister — so a
    /// mutation applied here lands strictly after the probe's own read and
    /// strictly before whatever the caller reads next. That is the exact gap
    /// the verdict has to survive: finality (or a retraction) that becomes
    /// visible between the probe and the snapshot the verdict is built from.
    ///
    /// The lookup is synchronous, so the wallet is taken with `try_write` —
    /// sound precisely because no wallet guard is held across it.
    struct InterleavedPersistence {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        /// Zero-based index of the persister lookup to mutate on.
        target_lookup: usize,
        lookups: std::sync::atomic::AtomicUsize,
        #[allow(clippy::type_complexity)]
        mutate: Mutex<Option<Box<dyn FnOnce(&mut PlatformWalletInfo) + Send>>>,
    }

    impl InterleavedPersistence {
        fn new(
            wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
            wallet_id: WalletId,
            target_lookup: usize,
            mutate: impl FnOnce(&mut PlatformWalletInfo) + Send + 'static,
        ) -> Self {
            Self {
                wallet_manager,
                wallet_id,
                target_lookup,
                lookups: std::sync::atomic::AtomicUsize::new(0),
                mutate: Mutex::new(Some(Box::new(mutate))),
            }
        }

        fn fired(&self) -> bool {
            self.mutate.lock().expect("interleave mutex").is_none()
        }
    }

    impl PlatformWalletPersistence for InterleavedPersistence {
        fn store(
            &self,
            _wallet_id: WalletId,
            _changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            Ok(())
        }

        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }

        fn get_core_tx_record(
            &self,
            _wallet_id: WalletId,
            _txid: &Txid,
        ) -> Result<Option<TransactionRecord>, PersistenceError> {
            let lookup = self
                .lookups
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if lookup == self.target_lookup {
                if let Some(mutate) = self.mutate.lock().expect("interleave mutex").take() {
                    let mut wm = loop {
                        if let Ok(guard) = self.wallet_manager.try_write() {
                            break guard;
                        }
                        std::thread::yield_now();
                    };
                    mutate(
                        wm.get_wallet_info_mut(&self.wallet_id)
                            .expect("wallet must remain registered"),
                    );
                }
            }
            // This backend keeps no records of its own; the mutation above is
            // its whole purpose.
            Ok(None)
        }
    }

    /// File `record` into the wallet's BIP44 account 0 — the synchronous
    /// twin of `ConflictFixture::file_record`, for use from inside
    /// [`InterleavedPersistence`].
    fn insert_record(info: &mut PlatformWalletInfo, record: TransactionRecord) {
        info.core_wallet
            .accounts
            .standard_bip44_accounts
            .get_mut(&0)
            .expect("funded fixture has BIP44 account 0")
            .transactions_mut()
            .insert(record.txid, record);
    }

    #[tokio::test]
    async fn built_resume_rebroadcasts_original_and_typed_failures_do_not_broadcast() {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(RecordingPersistence::default());
        let broadcaster = Arc::new(RecordingBroadcaster::default());
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::clone(&broadcaster),
            WalletPersister::new(wallet_id, persistence),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityRegistration,
                4,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction: transaction.clone(),
                    account_index: 0,
                    funding_type: AssetLockFundingType::IdentityRegistration,
                    identity_index: 4,
                    amount: 1_000_000,
                    status: AssetLockStatus::Built,
                    proof: None,
                },
            );
        }

        let timed_out = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof event should arrive");
        assert!(matches!(
            timed_out,
            PlatformWalletError::FinalityTimeout(actual) if actual == out_point
        ));
        {
            let broadcast = broadcaster
                .transactions
                .lock()
                .expect("recording broadcaster mutex");
            assert_eq!(broadcast.as_slice(), std::slice::from_ref(&transaction));
            assert_eq!(broadcast[0].txid(), out_point.txid);
        }

        // The real consume path leaves a terminal tombstone. That gives a
        // same-process retry a truthful typed error, while a foreign outpoint
        // remains distinguishable as never tracked.
        let consumed_changeset = manager
            .consume_asset_lock(&out_point)
            .await
            .expect("consume tracked lock");
        assert_eq!(
            consumed_changeset
                .asset_locks
                .get(&out_point)
                .expect("consumed snapshot")
                .status,
            AssetLockStatus::Consumed
        );
        {
            let wm = wallet_manager.read().await;
            assert_eq!(
                wm.get_wallet_info(&wallet_id)
                    .expect("wallet")
                    .tracked_asset_locks
                    .get(&out_point)
                    .expect("consumed tombstone")
                    .status,
                AssetLockStatus::Consumed
            );
        }
        let consumed = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("consumed lock must fail");
        assert!(matches!(
            consumed,
            PlatformWalletError::AssetLockAlreadyConsumed(actual) if actual == out_point
        ));

        let foreign = OutPoint::new(Txid::all_zeros(), 7);
        let unknown = manager
            .resume_asset_lock(&foreign, Some(Duration::from_millis(10)))
            .await
            .expect_err("foreign outpoint must fail");
        assert!(matches!(
            unknown,
            PlatformWalletError::AssetLockNotTracked(actual) if actual == foreign
        ));
        assert_eq!(
            broadcaster
                .transactions
                .lock()
                .expect("recording broadcaster mutex")
                .len(),
            1,
            "terminal/foreign failures must not trigger another broadcast"
        );
    }

    /// Builds a tracked `Built`-status lock on a funded wallet and resumes it
    /// through `broadcaster`, returning the resume error and the lock's status
    /// afterwards. Shared by the two ambiguity/rejection cases below.
    async fn resume_built_lock_with(
        broadcaster: Arc<dyn TransactionBroadcaster>,
    ) -> (PlatformWalletError, AssetLockStatus) {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(wallet_id, Arc::new(RecordingPersistence::default())),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityTopUp,
                4,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered")
                .tracked_asset_locks
                .insert(
                    out_point,
                    TrackedAssetLock {
                        out_point,
                        transaction,
                        account_index: 0,
                        funding_type: AssetLockFundingType::IdentityTopUp,
                        identity_index: 4,
                        amount: 1_000_000,
                        status: AssetLockStatus::Built,
                        proof: None,
                    },
                );
        }

        let error = manager
            .resume_asset_lock(&out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof event should arrive in either case");
        let status = wallet_manager
            .read()
            .await
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&out_point)
            .expect("lock stays tracked")
            .status
            .clone();
        (error, status)
    }

    /// An AMBIGUOUS re-broadcast must not end the resume. A lock sitting at
    /// `Built` whose transaction was in fact already broadcast (app killed
    /// between the send and the status advance) draws `MaybeSent` on every
    /// retry, so failing on it pinned the lock at `Built` forever and the
    /// top-up could never complete. It must advance to `Broadcast` and go on
    /// to wait for the proof — here, until the 10ms test timeout.
    #[tokio::test]
    async fn built_resume_survives_an_ambiguous_rebroadcast_and_advances() {
        let (error, status) = resume_built_lock_with(Arc::new(AlwaysMaybeSentBroadcaster)).await;

        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "resume must reach the proof wait, not fail on the broadcast: {error:?}"
        );
        assert_eq!(
            status,
            AssetLockStatus::Broadcast,
            "an ambiguous re-broadcast must still advance the lock, or every \
             later pass repeats the same broadcast and the same failure"
        );
    }

    /// A DEFINITE rejection also fails the resume, but as an UNKNOWN
    /// outcome — never as the definite-rejection contract.
    ///
    /// `Rejected` is scoped to the attempt that produced it: the production
    /// `SpvBroadcaster` reaches it only from an unstarted client and the
    /// zero-connected-peers check, so it means "this send never left the
    /// device". A row sits at `Built` after a SUCCESSFUL broadcast too (the
    /// app killed between the send and the status advance), so an earlier
    /// send may be in a mempool or already mined. `TransactionBroadcast` —
    /// the FFI's code 26 — would tell the host that Core rejected the
    /// transaction, that its inputs' reservation was released and that a
    /// rebuild is safe; the resume releases nothing and keeps the row, so a
    /// host honouring that would build a SECOND asset lock beside a
    /// possibly-live one. Only the initial build path, which does untrack
    /// and release, may emit 26.
    #[tokio::test]
    async fn built_resume_of_a_rejected_rebroadcast_reports_an_unknown_outcome() {
        let (error, status) = resume_built_lock_with(Arc::new(AlwaysRejectedBroadcaster)).await;

        assert!(
            !matches!(error, PlatformWalletError::TransactionBroadcast(_)),
            "a re-broadcast that never left the device is not evidence that an earlier \
             send was rejected, so it must not claim the definite-rejection contract \
             while the row and its reservation are kept: {error:?}"
        );
        assert!(
            matches!(
                error,
                PlatformWalletError::TransactionBroadcastUnconfirmed(_)
            ),
            "a rejected re-broadcast must fail the resume as an unknown outcome: {error:?}"
        );
        assert_eq!(
            status,
            AssetLockStatus::Built,
            "a send that never dispatched must leave the row resumable at Built"
        );
    }

    /// A lazily-created `IdentityTopUp` funding account must survive a
    /// restart. Its persisted registration round (account xpub + pool
    /// snapshot) is the ONLY record the load path can rebuild the account
    /// from — the `account_registrations` / `account_address_pools`
    /// changeset fields are not replayed by `apply_changeset` — so
    /// without it, `resume_asset_lock` on a relaunched wallet fails at
    /// `rederive_credit_output_path` ("Funding account IdentityTopUp not
    /// found for re-derivation") and the already-broadcast top-up is
    /// stranded.
    #[tokio::test]
    async fn topup_credit_output_path_rederives_after_restart() {
        const TOPUP_INDEX: u32 = 7;

        // --- Session 1: build a top-up asset lock. This lazily creates
        // the IdentityTopUp{7} account and must persist its registration.
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let persistence = Arc::new(RecordingPersistence::default());
        // The mock SDK network must match the testnet wallet fixture:
        // `rederive_credit_output_path` decodes the credit-output address
        // against `sdk.network`.
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            Arc::clone(&sdk),
            wallet_manager,
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(
                wallet_id,
                Arc::clone(&persistence) as Arc<dyn PlatformWalletPersistence>,
            ),
        );
        let (tx, path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::IdentityTopUp,
                TOPUP_INDEX,
                &signer,
            )
            .await
            .expect("build top-up asset lock");

        let topup_account_type = AccountType::IdentityTopUp {
            registration_index: TOPUP_INDEX,
        };
        let registrations: Vec<crate::changeset::AccountRegistrationEntry> = {
            let stored = persistence.stored.lock().expect("recording mutex");
            assert!(
                stored.iter().any(|cs| cs
                    .account_address_pools
                    .iter()
                    .any(|p| p.account_type == topup_account_type && !p.addresses.is_empty())),
                "the lazily-created top-up account must persist a non-empty \
                 initial address-pool snapshot"
            );
            stored
                .iter()
                .flat_map(|cs| cs.account_registrations.iter().cloned())
                .collect()
        };
        assert!(
            registrations
                .iter()
                .any(|r| r.account_type == topup_account_type),
            "the lazily-created top-up account must persist an \
             AccountRegistrationEntry, got {registrations:?}"
        );

        // --- "Restart": rebuild the wallet the way the FFI load path
        // does (`build_wallet_start_state`) — an external-signable
        // Wallet holding ONLY the persisted account registrations (pool
        // windows regenerate deterministically from each account xpub) —
        // and re-track the lock from its persisted row.
        let mut accounts = AccountCollection::new();
        for reg in &registrations {
            let account = Account::from_xpub(
                Some(wallet_id),
                reg.account_type,
                reg.account_xpub,
                Network::Testnet,
            )
            .expect("Account::from_xpub");
            accounts.insert(account).expect("insert restored account");
        }
        let restored_wallet = Wallet::new_external_signable(Network::Testnet, wallet_id, accounts);
        let mut restored_info = PlatformWalletInfo {
            observed_input_conflicts: Default::default(),
            core_wallet: ManagedWalletInfo::from_wallet(&restored_wallet, 0),
            generation: Arc::new(WalletGeneration::new()),
            identity_manager: IdentityManager::new(),
            tracked_asset_locks: BTreeMap::new(),
            dpns_name_states: BTreeMap::new(),
        };
        let out_point = OutPoint::new(tx.txid(), 0);
        let lock = TrackedAssetLock {
            out_point,
            transaction: tx,
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityTopUp,
            identity_index: TOPUP_INDEX,
            amount: 1_000_000,
            status: AssetLockStatus::Built,
            proof: None,
        };
        restored_info
            .tracked_asset_locks
            .insert(out_point, lock.clone());

        let mut wm = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let restored_id = wm
            .insert_wallet(restored_wallet, restored_info)
            .expect("insert restored wallet");
        assert_eq!(
            restored_id, wallet_id,
            "external-signable restore must keep the persisted wallet id"
        );

        let restored_manager = AssetLockManager::new(
            sdk,
            Arc::new(RwLock::new(wm)),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(wallet_id, persistence as Arc<dyn PlatformWalletPersistence>),
        );
        let rederived = restored_manager
            .rederive_credit_output_path(&lock)
            .await
            .expect(
                "credit-output path must re-derive from the persisted \
                 IdentityTopUp account after a restart",
            );
        assert_eq!(
            rederived, path,
            "re-derived credit-output path must match the build-time path"
        );
    }

    // -----------------------------------------------------------------
    // Input-conflict screen (double-spent asset locks)
    // -----------------------------------------------------------------

    /// Everything the input-conflict tests need: a funded wallet, a built
    /// asset-lock transaction over its spendable UTXO, its outpoint, and a
    /// manager whose broadcaster records every send so a test can prove
    /// the screen fired *before* the (re-)broadcast rather than after it.
    struct ConflictFixture {
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        manager: AssetLockManager<RecordingBroadcaster>,
        broadcaster: Arc<RecordingBroadcaster>,
        transaction: Transaction,
        out_point: OutPoint,
        /// Kept so a test can attempt a REBUILD: the wallet's whole balance
        /// rides on the one UTXO this fixture's transaction spends, so a
        /// rebuild that fails at input selection is direct proof the funding
        /// reservation is still held.
        signer: crate::test_support::WalletSigner,
    }

    impl ConflictFixture {
        async fn new() -> Self {
            Self::with_broadcaster(RecordingBroadcaster::default()).await
        }

        /// The same fixture whose (re-)broadcast is refused before dispatch,
        /// the way an app-launch catch-up resume meets an SPV client that
        /// has not connected yet.
        async fn rejecting() -> Self {
            Self::with_broadcaster(RecordingBroadcaster {
                reject: true,
                ..Default::default()
            })
            .await
        }

        async fn with_broadcaster(broadcaster: RecordingBroadcaster) -> Self {
            Self::with_broadcaster_and_persistence(broadcaster, |_, _| {
                Arc::new(RecordingPersistence::default())
            })
            .await
        }

        /// The fixture wired to a caller-supplied persistence backend, built
        /// from the wallet handle so an interleaving stub can reach back into
        /// the wallet it is going to mutate.
        async fn with_broadcaster_and_persistence(
            broadcaster: RecordingBroadcaster,
            persistence: impl FnOnce(
                Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
                WalletId,
            ) -> Arc<dyn PlatformWalletPersistence>,
        ) -> Self {
            let (wallet_manager, wallet_id, _generation, signer) =
                funded_wallet_manager(StandardAccountType::BIP44Account).await;
            let broadcaster = Arc::new(broadcaster);
            let sdk = Arc::new(
                dash_sdk::SdkBuilder::new_mock()
                    .with_network(Network::Testnet)
                    .build()
                    .expect("mock sdk"),
            );
            let manager = AssetLockManager::new(
                sdk,
                Arc::clone(&wallet_manager),
                wallet_id,
                Arc::new(Notify::new()),
                Arc::clone(&broadcaster),
                WalletPersister::new(
                    wallet_id,
                    persistence(Arc::clone(&wallet_manager), wallet_id),
                ),
            );
            let (transaction, _path) = manager
                .build_asset_lock_transaction(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    4,
                    &signer,
                )
                .await
                .expect("build asset lock");
            let out_point = OutPoint::new(transaction.txid(), 0);
            Self {
                wallet_manager,
                wallet_id,
                manager,
                broadcaster,
                transaction,
                out_point,
                signer,
            }
        }

        /// Attempt a fresh asset-lock build over the same wallet. The funded
        /// fixture holds exactly one spendable UTXO, so this can only succeed
        /// once that UTXO's reservation has been released.
        async fn rebuild(&self) -> Result<(), PlatformWalletError> {
            self.manager
                .build_asset_lock_transaction(
                    1_000_000,
                    0,
                    AssetLockFundingType::IdentityRegistration,
                    5,
                    &self.signer,
                )
                .await
                .map(|_| ())
        }

        /// The single outpoint the asset-lock transaction spends — the one
        /// a rescan-resurrected UTXO would have handed it a second time.
        fn funded_input(&self) -> OutPoint {
            self.transaction
                .input
                .first()
                .expect("asset lock spends at least one input")
                .previous_output
        }

        async fn track(
            &self,
            status: AssetLockStatus,
            proof: Option<dpp::prelude::AssetLockProof>,
        ) {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet must remain registered");
            info.tracked_asset_locks.insert(
                self.out_point,
                TrackedAssetLock {
                    out_point: self.out_point,
                    transaction: self.transaction.clone(),
                    account_index: 0,
                    funding_type: AssetLockFundingType::IdentityRegistration,
                    identity_index: 4,
                    amount: 1_000_000,
                    status,
                    proof,
                },
            );
        }

        /// Park the wallet's applied-chainlock watermark at `height`
        /// without running the promotion pass, so restored rows keep the
        /// pre-chainlock context they were persisted with.
        async fn set_chain_lock_boundary(&self, height: CoreBlockHeight) {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet must remain registered");
            info.core_wallet.metadata.last_applied_chain_lock = Some(ChainLock {
                block_height: height,
                block_hash: BlockHash::all_zeros(),
                signature: BLSSignature::from([0u8; 96]),
            });
        }

        /// File `record` in the wallet's BIP44 account by direct map
        /// insertion. Going through the detection pipeline instead would
        /// route the record by relevance and, for a chainlocked context,
        /// evict it again under the default `keep-finalized-transactions`
        /// build — the scan under test reads `transaction_history()`, so
        /// the record has to actually be there.
        async fn file_record(&self, record: TransactionRecord) {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet must remain registered");
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded fixture has BIP44 account 0")
                .transactions_mut()
                .insert(record.txid, record);
        }

        /// Prime the screen's session memory directly, the way the load
        /// seeder or a prior resume would.
        async fn remember_conflict(&self, spender: Txid, height: u32) {
            let wm = self.manager.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&self.wallet_id)
                .expect("wallet must remain registered");
            info.observed_input_conflicts
                .lock()
                .expect("test cache")
                .insert(
                    self.funded_input(),
                    crate::wallet::platform_wallet::ObservedInputConflict { spender, height },
                );
        }

        /// Remove `txid`'s record from the wallet's BIP44 account, the way
        /// `apply_chain_lock`'s promotion-eviction does under the default
        /// `keep-finalized-transactions = OFF` build.
        async fn evict_record(&self, txid: Txid) {
            let mut wm = self.manager.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .expect("wallet must remain registered");
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded fixture has BIP44 account 0")
                .transactions_mut()
                .remove(&txid);
        }

        fn broadcast_count(&self) -> usize {
            self.broadcaster
                .transactions
                .lock()
                .expect("recording broadcaster mutex")
                .len()
        }
    }

    /// Wrap `transaction` as a history record filed against BIP44 account 0.
    fn record_for(transaction: Transaction, context: TransactionContext) -> TransactionRecord {
        TransactionRecord::new(
            transaction,
            AccountType::Standard {
                index: 0,
                standard_account_type: StandardAccountType::BIP44Account,
            },
            context,
            TransactionType::Standard,
            TransactionDirection::Outgoing,
            Vec::new(),
            Vec::new(),
            0,
        )
    }

    /// A distinct transaction that spends `spends`. Its txid falls out of
    /// the inputs, so it never collides with the asset lock's own.
    fn transaction_spending(spends: OutPoint) -> Transaction {
        Transaction {
            version: 1,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: spends,
                ..Default::default()
            }],
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    fn confirmed_at(height: u32) -> TransactionContext {
        TransactionContext::InBlock(BlockInfo::new(
            height,
            BlockHash::all_zeros(),
            1_700_000_000,
        ))
    }

    fn chain_locked_at(height: u32) -> TransactionContext {
        TransactionContext::InChainLockedBlock(BlockInfo::new(
            height,
            BlockHash::all_zeros(),
            1_700_000_000,
        ))
    }

    /// The base case: a confirmed sibling spending the lock's input makes
    /// the resume end in the contested variant — the screen's one verdict,
    /// which carries no licence to discard the tracked lock. It arrives
    /// after the resume has run its course, not instead of it.
    #[tokio::test]
    async fn broadcast_resume_reports_a_contested_input_for_a_merely_in_block_spender() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a currently double-spent asset lock must fail, not wait");
        match error {
            PlatformWalletError::AssetLockInputContested {
                out_point,
                input,
                spent_by,
                height,
            } => {
                assert_eq!(out_point, fixture.out_point);
                assert_eq!(input, fixture.funded_input());
                assert_eq!(spent_by, spender_txid);
                assert_eq!(height, Some(1_234));
            }
            other => panic!("expected AssetLockInputContested, got {other:?}"),
        }
        assert_eq!(
            fixture.broadcast_count(),
            1,
            "the screen must not refuse the defensive re-broadcast — the \
             sighting may be restored evidence nothing can retract"
        );
    }

    /// Regression: a conflict sighting must never cost the lock a proof
    /// that has already arrived.
    ///
    /// The screen reads history the load path rebuilt from persisted rows,
    /// and such a record is never checked against the active chain — a
    /// wallet offline while the spender's block was reorganized out
    /// restores the sighting all the same, and no later event demotes a
    /// transaction that is absent from both the replacement chain and every
    /// mempool. Refusing to broadcast or wait on that evidence returned the
    /// contested verdict on every resume and every launch for a lock whose
    /// own funding transaction was sitting in history chain-locked, ready
    /// to settle. The resume must run and take the proof.
    #[tokio::test]
    async fn a_standing_conflict_never_costs_the_lock_a_proof_that_has_arrived() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        // The unreconciled sighting.
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;
        // ... and the lock's own funding transaction, chain-locked: the
        // proof `wait_for_proof` resolves from without touching the network.
        fixture
            .file_record(record_for(
                fixture.transaction.clone(),
                chain_locked_at(1_500),
            ))
            .await;

        let (proof, _path) = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect("a lock whose own record is chain-locked must resume despite the sighting");
        match proof {
            dpp::prelude::AssetLockProof::Chain(chain) => {
                assert_eq!(chain.out_point, fixture.out_point);
                assert_eq!(chain.core_chain_locked_height, 1_500);
            }
            other => panic!("expected a ChainAssetLockProof, got {other:?}"),
        }
    }

    /// Regression: a `Built` row whose re-broadcast is refused before it
    /// dispatches must still take a proof that has already arrived.
    ///
    /// This is the launch catch-up shape: `catchUpStuckAssetLocks` resumes
    /// a restored row before SPV connects, so the re-broadcast draws the
    /// DEFINITE `Rejected` (unstarted client / zero peers), while history
    /// carries both a restored spender of the lock's input and the lock's
    /// own chain-locked funding record. Returning the rejection there
    /// skipped the record entirely — an already-final lock failed on every
    /// launch until connectivity returned, and it failed as the FFI's code
    /// 26, whose released-reservation contract this path does not honour.
    #[tokio::test]
    async fn a_rejected_rebroadcast_of_a_conflicted_built_lock_still_takes_an_arrived_proof() {
        let fixture = ConflictFixture::rejecting().await;
        fixture.track(AssetLockStatus::Built, None).await;

        // The unreconciled sighting...
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;
        // ... and the lock's own funding transaction, chain-locked.
        fixture
            .file_record(record_for(
                fixture.transaction.clone(),
                chain_locked_at(1_500),
            ))
            .await;

        let (proof, _path) = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect(
                "a locally-proven lock must resume despite a rejected re-broadcast and a \
                 standing sighting",
            );
        match proof {
            dpp::prelude::AssetLockProof::Chain(chain) => {
                assert_eq!(chain.core_chain_locked_height, 1_500);
            }
            other => panic!("expected a ChainAssetLockProof, got {other:?}"),
        }
        assert_eq!(
            fixture
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&fixture.wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&fixture.out_point)
                .expect("lock stays tracked")
                .status,
            AssetLockStatus::ChainLocked,
            "the resume must advance the row exactly as a waited-for proof would"
        );
        // Completing from a local proof releases nothing either: the lock is
        // settled and its inputs stay spent by it, so a rebuild must still
        // find no candidates.
        let rebuild = fixture.rebuild().await;
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "a settled lock must keep its funding reservation, got {rebuild:?}"
        );
    }

    /// The same shape without a proof: the rejection must not pre-empt the
    /// bounded wait the sighting exists to bound, and its expiry must be
    /// reported as the provisional contested verdict — never as the
    /// definite-rejection code 26, which promises a released reservation
    /// this path does not release.
    #[tokio::test]
    async fn a_rejected_rebroadcast_of_a_conflicted_built_lock_reports_the_contested_verdict() {
        let fixture = ConflictFixture::rejecting().await;
        fixture.track(AssetLockStatus::Built, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof exists, so the bounded wait must expire");
        match error {
            PlatformWalletError::AssetLockInputContested {
                out_point,
                input,
                spent_by,
                height,
            } => {
                assert_eq!(out_point, fixture.out_point);
                assert_eq!(input, fixture.funded_input());
                assert_eq!(spent_by, spender_txid);
                assert_eq!(height, Some(1_234));
            }
            other => panic!(
                "a rejected re-broadcast must not pre-empt the sighting's bounded wait, \
                 and must never surface the released-reservation contract, got {other:?}"
            ),
        }
        assert_eq!(
            fixture.broadcast_count(),
            1,
            "the re-broadcast is still attempted — the sighting bounds the resume, it \
             never refuses it"
        );
        assert_eq!(
            fixture
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&fixture.wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&fixture.out_point)
                .expect("lock stays tracked")
                .status,
            AssetLockStatus::Built,
            "a send that never dispatched must not advance the row — the next resume \
             re-sends rather than waiting on a broadcast that never happened"
        );
        // The retained status is only half the invariant. A row that is
        // resumable while its inputs are re-spendable is exactly the state
        // the release gate exists to prevent, and only a rebuild attempt can
        // prove the reservation is still held: the fixture's whole balance
        // rides on the one UTXO this lock spends, so a released reservation
        // would let this build succeed and put a second asset lock on the
        // wire beside the first.
        let rebuild = fixture.rebuild().await;
        assert!(
            matches!(
                rebuild,
                Err(PlatformWalletError::AssetLockInsufficientFunds { available: 0, .. })
            ),
            "the funding reservation must still be held after a rejected re-broadcast, \
             leaving a rebuild with zero spendable candidates, got {rebuild:?}"
        );
    }

    /// Regression: finality that lands BETWEEN the verdict's proof probe and
    /// the snapshot the verdict is built from must still outrank the
    /// standing sighting.
    ///
    /// This is the narrowest interleaving the decision has to survive, and
    /// the whole resume is driven through the public entry point to reach
    /// it. The funding record is filed from inside the probe's own persister
    /// lookup — after that probe has already read the in-memory map and
    /// missed, before anything else is read. Finality arriving that way
    /// never touches the tracked row (`LockNotifyHandler` wakes waiters
    /// without advancing a status), so the row still says `Broadcast` with
    /// no proof and the sibling is still sitting in history: reading the
    /// record and the row in two separate snapshots reported a locally final
    /// lock as contested.
    #[tokio::test]
    async fn finality_landing_between_the_probe_and_the_snapshot_outranks_the_conflict() {
        let funding_tx = Mutex::new(None);
        let interleave = Mutex::new(None);
        let fixture = ConflictFixture::with_broadcaster_and_persistence(
            RecordingBroadcaster::default(),
            |wallet_manager, wallet_id| {
                // The lock's own transaction is only known once the fixture
                // has built it, so the stub reads it back out of the shared
                // slot the fixture fills in below.
                let built = Arc::new(Mutex::new(None::<Transaction>));
                let handle = Arc::clone(&built);
                let stub = Arc::new(InterleavedPersistence::new(
                    wallet_manager,
                    wallet_id,
                    // Lookup 0 is the expiring proof wait's own miss; lookup
                    // 1 is the verdict's probe, the gap under test.
                    1,
                    move |info| {
                        let transaction = handle
                            .lock()
                            .expect("built transaction slot")
                            .clone()
                            .expect("fixture files the transaction before resuming");
                        insert_record(info, record_for(transaction, chain_locked_at(1_500)));
                    },
                ));
                *funding_tx.lock().expect("slot") = Some(built);
                *interleave.lock().expect("slot") = Some(Arc::clone(&stub));
                stub as Arc<dyn PlatformWalletPersistence>
            },
        )
        .await;
        let funding_tx = funding_tx.lock().expect("slot").take().expect("slot set");
        let interleave = interleave.lock().expect("slot").take().expect("slot set");
        *funding_tx.lock().expect("built transaction slot") = Some(fixture.transaction.clone());

        fixture.track(AssetLockStatus::Broadcast, None).await;
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("the proof arrives too late for this wait to return it");
        assert!(
            interleave.fired(),
            "the test proves nothing unless the finality actually landed inside the \
             verdict's probe"
        );
        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a lock whose own record reached finality during the probe must not be \
             reported as contested on the strength of a sibling still sitting in \
             history — the caller keeps its expiry and the next resume returns the \
             proof, got {error:?}"
        );
        assert_eq!(
            fixture
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&fixture.wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&fixture.out_point)
                .expect("lock stays tracked")
                .status,
            AssetLockStatus::Broadcast,
            "suppressing the verdict must leave the row exactly where it was"
        );
    }

    /// Regression: a ChainLock that lands in that same gap must outrank the
    /// sighting even though the promotion it performs takes the funding
    /// record away.
    ///
    /// Promotion is EVICTION under the default
    /// `keep-finalized-transactions = OFF` build: `apply_chain_lock` drops
    /// the record it has just promoted and keeps only its txid in the
    /// account's finalized set. A snapshot that asked the record alone
    /// therefore questioned the one place finality no longer lives, and
    /// condemned a locally final lock on the strength of a sibling the same
    /// chainlock never buried. The chainlock here is applied for real —
    /// the funding transaction is filed in a block below the lock height and
    /// promoted by the wallet's own pass — so the eviction is the wallet's,
    /// not the test's. The sibling sits in a HIGHER block on purpose: the
    /// same pass must leave it standing, or there would be no conflict left
    /// to suppress and the test would pass on any code.
    #[tokio::test]
    async fn a_chainlock_evicting_the_funding_record_mid_verdict_outranks_the_conflict() {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

        let funding_tx = Mutex::new(None);
        let interleave = Mutex::new(None);
        let fixture = ConflictFixture::with_broadcaster_and_persistence(
            RecordingBroadcaster::default(),
            |wallet_manager, wallet_id| {
                let built = Arc::new(Mutex::new(None::<Transaction>));
                let handle = Arc::clone(&built);
                let stub = Arc::new(InterleavedPersistence::new(
                    wallet_manager,
                    wallet_id,
                    // Lookup 0 is the expiring proof wait's own miss; lookup
                    // 1 is the verdict's probe, the gap under test.
                    1,
                    move |info| {
                        let transaction = handle
                            .lock()
                            .expect("built transaction slot")
                            .clone()
                            .expect("fixture files the transaction before resuming");
                        insert_record(info, record_for(transaction, confirmed_at(1_200)));
                        info.apply_chain_lock(ChainLock {
                            block_height: 1_220,
                            block_hash: BlockHash::all_zeros(),
                            signature: BLSSignature::from([0u8; 96]),
                        });
                    },
                ));
                *funding_tx.lock().expect("slot") = Some(built);
                *interleave.lock().expect("slot") = Some(Arc::clone(&stub));
                stub as Arc<dyn PlatformWalletPersistence>
            },
        )
        .await;
        let funding_tx = funding_tx.lock().expect("slot").take().expect("slot set");
        let interleave = interleave.lock().expect("slot").take().expect("slot set");
        *funding_tx.lock().expect("built transaction slot") = Some(fixture.transaction.clone());

        fixture.track(AssetLockStatus::Broadcast, None).await;
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("the chainlock arrives too late for this wait to return a proof");
        assert!(
            interleave.fired(),
            "the test proves nothing unless the chainlock actually landed inside the \
             verdict's probe"
        );
        {
            let wm = fixture.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&fixture.wallet_id)
                .expect("wallet must remain registered");
            let account = info
                .core_wallet
                .accounts
                .standard_bip44_accounts
                .get(&0)
                .expect("funded fixture has BIP44 account 0");
            #[cfg(not(feature = "keep-finalized-transactions"))]
            assert!(
                !account.transactions().contains_key(&fixture.out_point.txid),
                "the interleaving under test is promotion-EVICTION: a record still \
                 sitting in the map would leave the resident-record check able to \
                 answer, and the eviction path untested"
            );
            assert!(
                account.transaction_is_finalized(&fixture.out_point.txid),
                "the finalized-txid set is where the promotion leaves the finality, \
                 and the only trace of it the verdict can still read"
            );
            assert!(
                info.core_wallet
                    .transaction_history()
                    .iter()
                    .any(|record| record.txid != fixture.out_point.txid && record.is_confirmed()),
                "the sibling must survive the same chainlock, or there is no \
                 contested verdict left for the finality to suppress"
            );
        }
        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a lock whose funding transaction was chainlocked during the probe must \
             not be reported as contested because the promotion evicted the record \
             that said so — the caller keeps its expiry and the next resume returns \
             the proof, got {error:?}"
        );
        assert_eq!(
            fixture
                .wallet_manager
                .read()
                .await
                .get_wallet_info(&fixture.wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&fixture.out_point)
                .expect("lock stays tracked")
                .status,
            AssetLockStatus::Broadcast,
            "suppressing the verdict must leave the row exactly where it was"
        );
    }

    /// The same precedence against a concurrent resume rather than against
    /// the record: two overlapping resumes run under different budgets, and
    /// the longer one attaches the proof and advances the row while the
    /// shorter one is expiring. Driven end to end, with the settling landing
    /// in the same probe-to-snapshot gap.
    #[tokio::test]
    async fn a_concurrent_resume_that_settled_the_lock_suppresses_the_contested_verdict() {
        let interleave = Mutex::new(None);
        let fixture = ConflictFixture::with_broadcaster_and_persistence(
            RecordingBroadcaster::default(),
            |wallet_manager, wallet_id| {
                let stub = Arc::new(InterleavedPersistence::new(
                    wallet_manager,
                    wallet_id,
                    1,
                    |info| {
                        let (out_point, lock) = info
                            .tracked_asset_locks
                            .iter_mut()
                            .next()
                            .map(|(out_point, lock)| (*out_point, lock))
                            .expect("the lock under resume is tracked");
                        lock.status = AssetLockStatus::ChainLocked;
                        lock.proof = Some(dpp::prelude::AssetLockProof::Chain(
                            dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof {
                                core_chain_locked_height: 1_500,
                                out_point,
                            },
                        ));
                    },
                ));
                *interleave.lock().expect("slot") = Some(Arc::clone(&stub));
                stub as Arc<dyn PlatformWalletPersistence>
            },
        )
        .await;
        let interleave = interleave.lock().expect("slot").take().expect("slot set");

        fixture.track(AssetLockStatus::Broadcast, None).await;
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("this resume's own wait found no proof before its deadline");
        assert!(
            interleave.fired(),
            "the concurrent resume must actually have settled the row mid-verdict"
        );
        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a row another resume already settled must not be re-condemned by the \
             screen, got {error:?}"
        );
    }

    /// Regression: a caller that named its own deadline keeps
    /// `FinalityTimeout` on the rejected-`Built` path too.
    ///
    /// The re-typings on that path exist to stop an UNBOUNDED wait hanging
    /// on a signal that cannot arrive; a caller that supplied a bound never
    /// had that problem. The shielded seed pool reads `FinalityTimeout` as a
    /// pacing signal and resumes the lock later, so handing it a
    /// do-not-retry error instead — and one quoting the 180-second policy
    /// cap rather than the bound it asked for — silently drops the lock out
    /// of that flow. The conflict retracts mid-verdict so the contested
    /// verdict is out of the way and the undispatched translation is the
    /// only thing left that could win.
    #[tokio::test]
    async fn a_rejected_built_rebroadcast_keeps_an_explicit_timeout_as_finality_timeout() {
        let spender = Mutex::new(None);
        let interleave = Mutex::new(None);
        let fixture = ConflictFixture::with_broadcaster_and_persistence(
            RecordingBroadcaster {
                reject: true,
                ..Default::default()
            },
            |wallet_manager, wallet_id| {
                let demoted = Arc::new(Mutex::new(None::<Transaction>));
                let handle = Arc::clone(&demoted);
                let stub = Arc::new(InterleavedPersistence::new(
                    wallet_manager,
                    wallet_id,
                    // Lookup 0 is the rejection's own local-proof probe,
                    // lookup 1 the expiring wait, lookup 2 the verdict's
                    // probe — the gap the retraction has to land in.
                    2,
                    move |info| {
                        let transaction = handle
                            .lock()
                            .expect("spender slot")
                            .clone()
                            .expect("fixture files the spender before resuming");
                        // A reorg drops the block; the record survives,
                        // demoted, which retracts the remembered sighting.
                        insert_record(info, record_for(transaction, TransactionContext::Mempool));
                    },
                ));
                *spender.lock().expect("slot") = Some(demoted);
                *interleave.lock().expect("slot") = Some(Arc::clone(&stub));
                stub as Arc<dyn PlatformWalletPersistence>
            },
        )
        .await;
        let spender_slot = spender.lock().expect("slot").take().expect("slot set");
        let interleave = interleave.lock().expect("slot").take().expect("slot set");

        fixture.track(AssetLockStatus::Built, None).await;
        let spender = transaction_spending(fixture.funded_input());
        *spender_slot.lock().expect("spender slot") = Some(spender.clone());
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof exists, so the caller's bounded wait must expire");
        assert!(
            interleave.fired(),
            "the conflict must actually have retracted mid-verdict, or the contested \
             verdict would be doing the work this test is about"
        );
        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a caller-selected timeout must expire as FinalityTimeout, not be retyped \
             into the unknown-outcome contract that quotes the policy cap it never \
             asked for, got {error:?}"
        );
    }

    /// The same on the memory path: a remembered sighting whose record has
    /// left history under a covering boundary still reports the
    /// provisional verdict, never the discard-licensing terminal one. The
    /// eviction attests a height-based promotion, not that the spender's
    /// block is on the finalized branch.
    #[tokio::test]
    async fn a_remembered_spender_evicted_under_the_boundary_stays_provisional() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender_txid = transaction_spending(fixture.funded_input()).txid();
        fixture.remember_conflict(spender_txid, 1_234).await;
        fixture.set_chain_lock_boundary(1_300).await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("the remembered conflict still stops the wait");
        assert!(
            matches!(error, PlatformWalletError::AssetLockInputContested { .. }),
            "a remembered sighting must not upgrade on the boundary, got {error:?}"
        );
    }

    /// The memory must never condemn a lock with its own txid: two tracked
    /// locks sharing an input cross-remember each other, and after the
    /// winner's record is promotion-evicted a resume of the WINNER must
    /// not read the memory as evidence against it — a lock's own spend is
    /// not a conflict with itself.
    #[tokio::test]
    async fn remembered_evidence_never_condemns_the_lock_itself() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        fixture
            .remember_conflict(fixture.transaction.txid(), 1_234)
            .await;
        fixture.set_chain_lock_boundary(1_300).await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof means the resume runs and then times out");
        assert!(
            !matches!(
                error,
                PlatformWalletError::AssetLockInputConflict { .. }
                    | PlatformWalletError::AssetLockInputContested { .. }
            ),
            "a lock's own remembered spend must never condemn it, got {error:?}"
        );
    }

    /// The load-time seeder primes the memory before any resume runs, so
    /// a chainlock dispatcher that promotion-evicts the restored spender
    /// before the first catch-up still leaves the screen with evidence —
    /// reported, like every other sighting, as the provisional verdict.
    #[tokio::test]
    async fn seeding_survives_a_pre_resume_promotion_eviction() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;
        // The load path's seeding pass, then the dispatcher's promotion
        // eviction — all before the first resume.
        {
            let wm = fixture.manager.wallet_manager.read().await;
            let info = wm
                .get_wallet_info(&fixture.wallet_id)
                .expect("wallet must remain registered");
            super::seed_observed_input_conflicts(info);
        }
        fixture.evict_record(spender_txid).await;
        fixture.set_chain_lock_boundary(1_300).await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("the seeded conflict must stop the wait");
        match error {
            PlatformWalletError::AssetLockInputContested { spent_by, .. } => {
                assert_eq!(spent_by, spender_txid, "the seeded spender, provisionally");
            }
            other => panic!("expected AssetLockInputContested, got {other:?}"),
        }
    }

    /// Promotion is eviction: once a chainlock buries the spender's block,
    /// `apply_chain_lock` removes its record from history. The screen's
    /// session memory must carry the conflict across that disappearance
    /// instead of letting the resume fall back into the proof wait — and it
    /// must carry it as the SAME provisional verdict, because a
    /// height-based promotion is not proof that the spender's block is on
    /// the finalized branch.
    #[tokio::test]
    async fn a_chainlock_evicted_spender_keeps_the_remembered_verdict_provisional() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;

        // First resume: the screen reports and remembers the sighting.
        let first = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a currently double-spent asset lock must fail, not wait");
        assert!(
            matches!(first, PlatformWalletError::AssetLockInputContested { .. }),
            "the screen's one verdict is provisional, got {first:?}"
        );

        // The chainlock lands: boundary moves past the spender's height and
        // the promotion evicts its record.
        fixture.evict_record(spender_txid).await;
        fixture.set_chain_lock_boundary(1_300).await;

        let second = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a chainlock-settled double spend must fail, not wait");
        match second {
            PlatformWalletError::AssetLockInputContested { spent_by, .. } => {
                assert_eq!(spent_by, spender_txid, "the remembered spender, unchanged");
            }
            other => panic!("expected AssetLockInputContested, got {other:?}"),
        }
    }

    /// The memory retracts: a reorg demotes the spender's record in place,
    /// and re-observing it unconfirmed must clear the remembered verdict —
    /// the lock is viable again and the resume takes its normal course.
    #[tokio::test]
    async fn a_reorg_demoted_spender_retracts_the_remembered_verdict() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        fixture
            .file_record(record_for(spender.clone(), confirmed_at(1_234)))
            .await;
        let first = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a currently double-spent asset lock must fail, not wait");
        assert!(matches!(
            first,
            PlatformWalletError::AssetLockInputContested { .. }
        ));

        // The reorg drops the block; the record survives, demoted.
        fixture
            .file_record(record_for(spender, TransactionContext::Mempool))
            .await;

        let second = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof means the resume runs and then times out");
        assert!(
            !matches!(
                second,
                PlatformWalletError::AssetLockInputConflict { .. }
                    | PlatformWalletError::AssetLockInputContested { .. }
            ),
            "a demoted spender must retract the remembered verdict, got {second:?}"
        );
    }

    /// The memory outlives the record with no applied chainlock in sight:
    /// the conflict is still reported, still provisionally. The boundary
    /// is not consulted at all — with or without one, the screen has the
    /// same evidence and gives the same answer.
    #[tokio::test]
    async fn an_evicted_spender_without_a_covering_boundary_stays_provisional() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;
        let _ = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await;
        fixture.evict_record(spender_txid).await;

        let second = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("the remembered conflict still stops the wait");
        assert!(
            matches!(second, PlatformWalletError::AssetLockInputContested { .. }),
            "no boundary, no terminal claim, got {second:?}"
        );
    }

    /// The applied chainlock watermark is NOT an ancestry proof, so a live
    /// in-block record sitting at or below it earns no promotion. The
    /// watermark is height-based, and the SPV chainlock manager counts a
    /// missing header as a passing block-hash check, so a chainlock landing
    /// on a replacement branch ahead of its headers can move it past a
    /// record that sits on the losing branch. The verdict stays contested.
    #[tokio::test]
    async fn a_live_spender_below_the_boundary_stays_contested() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, confirmed_at(1_234)))
            .await;
        fixture.set_chain_lock_boundary(1_300).await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a double-spent asset lock must fail, not wait");
        match error {
            PlatformWalletError::AssetLockInputContested { spent_by, .. } => {
                assert_eq!(spent_by, spender_txid);
            }
            other => panic!("expected AssetLockInputContested, got {other:?}"),
        }
    }

    /// The strongest evidence the wallet can hold — a spender whose own
    /// record carries a chain-locked context — still buys no upgrade. That
    /// context is set by the same height-based promotion, so it attests a
    /// chainlock at the record's height, not that the record's block is on
    /// the branch the chainlock covers. The terminal variant has no
    /// emitter; the resume reports the provisional one here too.
    #[tokio::test]
    async fn a_chain_locked_spender_still_reports_only_the_contested_verdict() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        let spender = transaction_spending(fixture.funded_input());
        let spender_txid = spender.txid();
        fixture
            .file_record(record_for(spender, chain_locked_at(1_234)))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a double-spent asset lock must fail, not wait");
        let rendered = error.to_string();
        match error {
            PlatformWalletError::AssetLockInputContested {
                spent_by, height, ..
            } => {
                assert_eq!(spent_by, spender_txid);
                assert_eq!(height, Some(1_234));
            }
            other => panic!("expected AssetLockInputContested, got {other:?}"),
        }
        assert!(
            rendered.contains("provisional"),
            "the rendered Display must say the verdict is provisional: {rendered}"
        );
        assert_eq!(
            fixture.broadcast_count(),
            1,
            "not even a chain-locked-looking spender may refuse the resume"
        );
    }

    /// An unconfirmed sibling spending the same outpoint is a competing
    /// candidate, not a verdict — either transaction can still win, and
    /// condemning the tracked lock on a mempool record would discard a
    /// perfectly live funding attempt. The resume must take its normal
    /// course (re-broadcast, then wait) instead.
    #[tokio::test]
    async fn broadcast_resume_ignores_an_unconfirmed_spend_of_the_same_input() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                TransactionContext::Mempool,
            ))
            .await;

        let error = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("no proof event should arrive within the deadline");
        assert!(
            !matches!(
                error,
                PlatformWalletError::AssetLockInputConflict { .. }
                    | PlatformWalletError::AssetLockInputContested { .. }
            ),
            "an unconfirmed conflict must not condemn the lock, got {error:?}"
        );
        assert_eq!(
            fixture.broadcast_count(),
            1,
            "the resume must still reach its defensive re-broadcast"
        );
    }

    /// The asset-lock transaction is itself filed in wallet history once
    /// it is seen on chain, and it necessarily spends every outpoint it
    /// spends. Matching on the outpoints alone would therefore make every
    /// confirmed lock report itself as its own double spend; the txid
    /// guard is what prevents that.
    #[tokio::test]
    async fn resume_does_not_treat_the_locks_own_confirmed_record_as_a_conflict() {
        let fixture = ConflictFixture::new().await;
        fixture.track(AssetLockStatus::Broadcast, None).await;

        fixture
            .file_record(record_for(fixture.transaction.clone(), confirmed_at(1_234)))
            .await;

        let outcome = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await;
        assert!(
            !matches!(
                outcome,
                Err(PlatformWalletError::AssetLockInputConflict { .. })
                    | Err(PlatformWalletError::AssetLockInputContested { .. })
            ),
            "a lock's own record must never condemn it under either variant, got {outcome:?}"
        );
    }

    /// Settled locks are decided by evidence the screen has no standing to
    /// overturn: a `Consumed` tombstone records a completed Platform spend,
    /// and a proof-carrying lock holds finality the network already granted.
    /// Both must return exactly what they returned before the screen
    /// existed, even with a confirmed conflicting record sitting in history
    /// — and neither may broadcast.
    #[tokio::test]
    async fn settled_locks_keep_their_outcome_despite_a_confirmed_conflicting_record() {
        let fixture = ConflictFixture::new().await;
        fixture
            .file_record(record_for(
                transaction_spending(fixture.funded_input()),
                confirmed_at(1_234),
            ))
            .await;

        let chain_proof = dpp::prelude::AssetLockProof::Chain(
            dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof {
                core_chain_locked_height: 1_234,
                out_point: fixture.out_point,
            },
        );
        fixture
            .track(AssetLockStatus::ChainLocked, Some(chain_proof.clone()))
            .await;
        let (resumed_proof, _path) = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect("a chain-locked lock resumes from its own proof");
        assert_eq!(resumed_proof, chain_proof);

        fixture.track(AssetLockStatus::Consumed, None).await;
        let consumed = fixture
            .manager
            .resume_asset_lock(&fixture.out_point, Some(Duration::from_millis(10)))
            .await
            .expect_err("a consumed lock must stay terminal");
        assert!(
            matches!(
                consumed,
                PlatformWalletError::AssetLockAlreadyConsumed(actual) if actual == fixture.out_point
            ),
            "expected AssetLockAlreadyConsumed, got {consumed:?}"
        );
        assert_eq!(
            fixture.broadcast_count(),
            0,
            "settled locks never re-enter the broadcast path"
        );
    }

    /// Builds a tracked lock at `status` on a funded wallet and resumes it
    /// through `broadcaster` with the given `timeout`, returning the resume
    /// error and the lock's tracked state afterwards (`None` = untracked).
    async fn resume_lock_at(
        broadcaster: Arc<dyn TransactionBroadcaster>,
        status: AssetLockStatus,
        timeout: Option<Duration>,
    ) -> (PlatformWalletError, Option<AssetLockStatus>) {
        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            broadcaster,
            WalletPersister::new(wallet_id, Arc::new(RecordingPersistence::default())),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::AssetLockAddressTopUp,
                4,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered")
                .tracked_asset_locks
                .insert(
                    out_point,
                    TrackedAssetLock {
                        out_point,
                        transaction,
                        account_index: 0,
                        funding_type: AssetLockFundingType::AssetLockAddressTopUp,
                        identity_index: 4,
                        amount: 1_000_000,
                        status,
                        proof: None,
                    },
                );
        }

        let error = manager
            .resume_asset_lock(&out_point, timeout)
            .await
            .expect_err("no proof event is ever delivered in these cases");
        let tracked = wallet_manager
            .read()
            .await
            .get_wallet_info(&wallet_id)
            .expect("wallet")
            .tracked_asset_locks
            .get(&out_point)
            .map(|lock| lock.status.clone());
        (error, tracked)
    }

    /// Regression: an ambiguous re-broadcast on the UNBOUNDED resume path
    /// must not hang.
    ///
    /// `MaybeSent` is the broadcaster's verdict for a genuinely rejected
    /// transaction as much as for an accepted one — `DapiBroadcaster`
    /// classifies every failure that way, and the SPV broadcaster reaches
    /// `Rejected` only on `NotConnected`. So advancing to `Broadcast` and
    /// then waiting with `wait_for_proof(None)` — which is what the three
    /// `resume_asset_lock(.., None)` production call sites do — turned a
    /// broadcast failure that used to surface in ~30s into a wait that never
    /// ends, because no proof can arrive for a tx that was never accepted.
    ///
    /// `start_paused` auto-advances the substituted bound, so this asserts
    /// termination *and* that the caller gets the pre-#4367 typed error back.
    #[tokio::test(start_paused = true)]
    async fn unbounded_resume_of_an_ambiguous_rebroadcast_terminates() {
        let (error, status) = resume_lock_at(
            Arc::new(AlwaysMaybeSentBroadcaster),
            AssetLockStatus::Built,
            None,
        )
        .await;

        assert!(
            matches!(
                error,
                PlatformWalletError::TransactionBroadcastUnconfirmed(_)
            ),
            "an unbounded resume whose re-broadcast was ambiguous must end as a \
             broadcast-unconfirmed failure rather than hang, got {error:?}"
        );
        assert_eq!(
            status,
            Some(AssetLockStatus::Broadcast),
            "the advance itself is still correct — the row stays resumable so a \
             later pass can pick up a proof that does eventually arrive"
        );
    }

    /// The bounded callers must be untouched by the fix above. The shielded
    /// seed pool passes its own timeout and treats `FinalityTimeout` as a
    /// pacing signal (pause, resume the lock later), so re-typing the error
    /// for every caller would have broken a working flow to fix a different
    /// one.
    #[tokio::test]
    async fn bounded_resume_of_an_ambiguous_rebroadcast_still_reports_finality_timeout() {
        let (error, status) = resume_lock_at(
            Arc::new(AlwaysMaybeSentBroadcaster),
            AssetLockStatus::Built,
            Some(Duration::from_millis(10)),
        )
        .await;

        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a caller-supplied timeout must keep its FinalityTimeout semantics: {error:?}"
        );
        assert_eq!(status, Some(AssetLockStatus::Broadcast));
    }

    /// A rejected defensive re-broadcast must not be reported as a rejection
    /// of the ORIGINAL transaction — and the row SURVIVES it.
    ///
    /// With the production `SpvBroadcaster`, `Rejected` means an unstarted
    /// client or zero connected peers: a fact about the re-broadcast attempt,
    /// not about the ORIGINAL broadcast that put the row at `Broadcast` in an
    /// earlier process. Two things followed from reading it as a verdict on
    /// the row.
    ///
    /// The first revision untracked the row here. `catchUpStuckAssetLocks`
    /// resumes every `statusRaw < 2` row on each wallet load with no
    /// SPV-connected gate, so that untrack deleted tracking for
    /// possibly-mined asset locks on ordinary offline relaunches, with no
    /// path back (reconstruction re-inserts only on a fresh detection event).
    ///
    /// The second was the error type. `TransactionBroadcast` is the FFI's
    /// code 26, which promises the host that Core rejected the transaction,
    /// its UTXO reservation was released and a rebuild is safe — while this
    /// arm deliberately keeps both the row and its reservation because the
    /// original may still confirm. A host honouring code 26 would rebuild
    /// from other UTXOs and create a SECOND asset lock alongside a live one.
    /// The non-terminal `TransactionBroadcastUnconfirmed` (code 20) is the
    /// contract that matches what this arm actually knows: outcome unknown,
    /// do not retry.
    #[tokio::test]
    async fn rejected_defensive_rebroadcast_is_not_a_rejection_of_the_original_transaction() {
        let (error, tracked) = resume_lock_at(
            Arc::new(AlwaysRejectedBroadcaster),
            AssetLockStatus::Broadcast,
            None,
        )
        .await;

        assert!(
            !matches!(error, PlatformWalletError::TransactionBroadcast(_)),
            "a re-broadcast that never left the device is not evidence the original \
             transaction was rejected, so it must not claim the definite-rejection \
             contract, got {error:?}"
        );
        assert!(
            matches!(
                error,
                PlatformWalletError::TransactionBroadcastUnconfirmed(_)
            ),
            "a rejected defensive re-broadcast must fail the resume as an unknown \
             outcome instead of falling through to a proof wait, got {error:?}"
        );
        assert_eq!(
            tracked,
            Some(AssetLockStatus::Broadcast),
            "a re-broadcast that never left the device says nothing about the \
             original send — the row must survive, unchanged, for a later resume"
        );
    }

    /// A definite rejection must consult the LOCAL record before failing
    /// the resume.
    ///
    /// A row can sit at `Broadcast` while its transaction record already
    /// carries finality: `LockNotifyHandler` only wakes waiters, so an
    /// IS/CL event that arrives with no waiter active enriches the record
    /// but never advances the tracked status, and `enrich_from_record`
    /// upgrades only `InChainLockedBlock` records on scan paths — an
    /// `InstantSend` context is invisible to it. On the next launch
    /// `catchUpStuckAssetLocks` resumes the row before SPV connects, the
    /// defensive re-broadcast draws `Rejected` (unstarted client / zero
    /// peers), and the pre-fix arm failed the resume even though
    /// `wait_for_proof` would have returned the proof on its first
    /// iteration, straight from the record, without any network at all.
    #[tokio::test]
    async fn definite_rejection_on_a_broadcast_lock_yields_the_local_proof() {
        use dashcore::ephemerealdata::instant_lock::InstantLock;
        use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
        use key_wallet::managed_account::transaction_record::{
            TransactionDirection, TransactionRecord,
        };
        use key_wallet::transaction_checking::{TransactionContext, TransactionType};

        let (wallet_manager, wallet_id, _balance, signer) =
            funded_wallet_manager(StandardAccountType::BIP44Account).await;
        let sdk = Arc::new(
            dash_sdk::SdkBuilder::new_mock()
                .with_network(Network::Testnet)
                .build()
                .expect("mock sdk"),
        );
        let manager = AssetLockManager::new(
            sdk,
            Arc::clone(&wallet_manager),
            wallet_id,
            Arc::new(Notify::new()),
            Arc::new(AlwaysRejectedBroadcaster),
            WalletPersister::new(wallet_id, Arc::new(RecordingPersistence::default())),
        );
        let (transaction, _path) = manager
            .build_asset_lock_transaction(
                1_000_000,
                0,
                AssetLockFundingType::AssetLockAddressTopUp,
                4,
                &signer,
            )
            .await
            .expect("build asset lock");
        let out_point = OutPoint::new(transaction.txid(), 0);
        {
            let mut wm = wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&wallet_id)
                .expect("wallet must remain registered");
            // The finality that arrived while nobody was waiting: an
            // IS-locked record for the funding tx, filed under the BIP44
            // account the lock was built from.
            let record = TransactionRecord::new(
                transaction.clone(),
                AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP44Account,
                },
                TransactionContext::InstantSend(InstantLock::default()),
                TransactionType::Standard,
                TransactionDirection::Outgoing,
                Vec::new(),
                Vec::new(),
                0,
            );
            info.core_wallet
                .accounts
                .standard_bip44_accounts
                .get_mut(&0)
                .expect("funded wallet has BIP44 account 0")
                .transactions_mut()
                .insert(record.txid, record);
            info.tracked_asset_locks.insert(
                out_point,
                TrackedAssetLock {
                    out_point,
                    transaction,
                    account_index: 0,
                    funding_type: AssetLockFundingType::AssetLockAddressTopUp,
                    identity_index: 4,
                    amount: 1_000_000,
                    status: AssetLockStatus::Broadcast,
                    proof: None,
                },
            );
        }

        let (proof, _path) = manager
            .resume_asset_lock(&out_point, None)
            .await
            .expect("a locally-proven lock must survive a rejected re-broadcast");
        assert!(
            matches!(proof, dpp::prelude::AssetLockProof::Instant(_)),
            "the proof must come from the record's InstantSend context: {proof:?}"
        );
        assert_eq!(
            wallet_manager
                .read()
                .await
                .get_wallet_info(&wallet_id)
                .expect("wallet")
                .tracked_asset_locks
                .get(&out_point)
                .expect("lock stays tracked")
                .status,
            AssetLockStatus::InstantSendLocked,
            "the resume must advance the row exactly as a waited-for proof would"
        );
    }

    /// Regression: the `Broadcast` arm's proof wait must terminate on the
    /// UNBOUNDED resume path too.
    ///
    /// The first revision of this fix bounded only the `Built` arm. Its own
    /// retained behavior — advance an ambiguous `Built` lock to `Broadcast`
    /// and leave the row there — routes exactly that lock into this arm on
    /// the next resume pass, where a bare `wait_for_proof(None)` waits on
    /// `Notify` forever. The hang was deferred by one pass, not removed, and
    /// under the FFI's `runtime().block_on(...)` it pins a host thread.
    ///
    /// `start_paused` auto-advances the substituted bound, so this asserts
    /// termination *and* the typed error the caller gets on expiry.
    #[tokio::test(start_paused = true)]
    async fn unbounded_resume_of_a_broadcast_row_terminates() {
        let (error, status) = resume_lock_at(
            Arc::new(AlwaysMaybeSentBroadcaster),
            AssetLockStatus::Broadcast,
            None,
        )
        .await;

        assert!(
            matches!(
                error,
                PlatformWalletError::TransactionBroadcastUnconfirmed(_)
            ),
            "an unbounded resume of a Broadcast row must end as a \
             broadcast-unconfirmed failure rather than hang, got {error:?}"
        );
        assert_eq!(
            status,
            Some(AssetLockStatus::Broadcast),
            "the row must stay exactly where it was so a proof arriving after the \
             bound is picked up by the next resume"
        );
    }

    /// The bounded callers of the `Broadcast` arm keep their semantics, the
    /// same way the `Built` arm's do: `or` is the identity on `Some`, and the
    /// re-typing is gated on the caller having asked for an unbounded wait.
    #[tokio::test]
    async fn bounded_resume_of_a_broadcast_row_still_reports_finality_timeout() {
        let (error, status) = resume_lock_at(
            Arc::new(AlwaysMaybeSentBroadcaster),
            AssetLockStatus::Broadcast,
            Some(Duration::from_millis(10)),
        )
        .await;

        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a caller-supplied timeout must keep its FinalityTimeout semantics: {error:?}"
        );
        assert_eq!(status, Some(AssetLockStatus::Broadcast));
    }

    /// The `RecoveredFromChain` proof-less fallback is bounded for the same
    /// reason, even though its wait resolves immediately whenever the
    /// chain-locked record it reads is still present. The accident that
    /// leaves a `RecoveredFromChain` row without its persisted proof can take
    /// the record too, and then the "resolves immediately by construction"
    /// argument yields an unbounded `Notify` loop. No re-typing: nothing is
    /// broadcast on this arm, so `FinalityTimeout` is the honest verdict.
    #[tokio::test(start_paused = true)]
    async fn unbounded_resume_of_a_proofless_recovered_row_terminates() {
        let (error, status) = resume_lock_at(
            Arc::new(AlwaysRejectedBroadcaster),
            AssetLockStatus::RecoveredFromChain,
            None,
        )
        .await;

        assert!(
            matches!(error, PlatformWalletError::FinalityTimeout(_)),
            "a proof-less RecoveredFromChain resume must terminate, got {error:?}"
        );
        assert_eq!(
            status,
            Some(AssetLockStatus::RecoveredFromChain),
            "the row keeps its status — the resume proved nothing new about it"
        );
    }
}
