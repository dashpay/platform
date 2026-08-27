//! Crash recovery and resume for asset locks.
//!
//! Contains methods for recovering asset locks from persisted state,
//! resolving status from wallet info, resuming interrupted locks,
//! and re-deriving private keys.

use crate::broadcaster::{BroadcastError, TransactionBroadcaster};
use std::time::Duration;

use dashcore::Address as DashAddress;
use dashcore::OutPoint;
use key_wallet::bip32::DerivationPath;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;

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

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
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
    pub async fn resume_asset_lock(
        &self,
        out_point: &OutPoint,
        timeout: Option<Duration>,
    ) -> Result<(dpp::prelude::AssetLockProof, DerivationPath), PlatformWalletError> {
        tracing::info!(outpoint = %out_point, ?timeout, "resume_asset_lock: entered");

        // 1. Look up the tracked lock — snapshot the fields we need.
        let (tx, status, existing_proof, account_index) = {
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
            (
                lock.transaction.clone(),
                lock.status.clone(),
                lock.proof.clone(),
                lock.account_index,
            )
        };

        // 2. Resume from the current status.
        let proof = match status {
            AssetLockStatus::Built => {
                // Re-broadcast and wait for proof.
                //
                // Only a DEFINITE rejection stops the resume. `MaybeSent`
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
                let maybe_sent_reason = match self.broadcaster.broadcast(&tx).await {
                    Ok(_) => None,
                    Err(BroadcastError::MaybeSent { reason }) => {
                        tracing::warn!(
                            outpoint = %out_point,
                            reason = %reason,
                            "resume_asset_lock: re-broadcast of a Built lock returned an \
                             unknown outcome (the network may already hold this tx, or may \
                             have rejected it — the broadcaster cannot tell); advancing to \
                             Broadcast and waiting for proof under a bounded timeout"
                        );
                        Some(reason)
                    }
                    Err(rejected) => return Err(rejected.into()),
                };
                let cs = self
                    .advance_asset_lock_status(out_point, AssetLockStatus::Broadcast, None)
                    .await?;
                self.queue_asset_lock_changeset(cs);
                let proof = match (&maybe_sent_reason, timeout) {
                    // Ambiguous re-broadcast AND an unbounded wait: the only
                    // combination that can hang forever. Substitute the bound
                    // and translate its expiry back into the broadcast error
                    // the caller used to get immediately.
                    //
                    // Callers that passed their own timeout are left exactly
                    // as they were, `FinalityTimeout` and all — the shielded
                    // seed pool treats that error as a pacing signal and
                    // resumes the lock later, so re-typing it would break a
                    // working flow to fix an unrelated one.
                    (Some(reason), None) => {
                        match self
                            .wait_for_proof(out_point, Some(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT))
                            .await
                        {
                            Ok(proof) => proof,
                            Err(PlatformWalletError::FinalityTimeout(_)) => {
                                return Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
                                    format!(
                                        "asset lock {} was re-broadcast with an unknown \
                                         outcome and no InstantSend/ChainLock proof arrived \
                                         within {:?}: {}",
                                        out_point, UNCONFIRMED_BROADCAST_PROOF_TIMEOUT, reason
                                    ),
                                ))
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    _ => self.wait_for_proof(out_point, timeout).await?,
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
                    let bounded = timeout.or(Some(UNCONFIRMED_BROADCAST_PROOF_TIMEOUT));
                    match self.wait_for_proof(out_point, bounded).await {
                        Ok(proof) => proof,
                        Err(PlatformWalletError::FinalityTimeout(_)) if timeout.is_none() => {
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
    use dashcore::hashes::Hash;
    use dashcore::{Network, OutPoint, Transaction, Txid};
    use key_wallet::account::account_collection::AccountCollection;
    use key_wallet::account::account_type::StandardAccountType;
    use key_wallet::account::{Account, AccountType};
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
    use crate::wallet::platform_wallet::PlatformWalletInfo;
    use crate::AssetLockFundingType;

    /// Persistence stub that records every stored changeset so the test
    /// can replay the registration rounds the way the FFI load path does.
    #[derive(Default)]
    struct RecordingPersistence {
        stored: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    /// Captures the exact transaction passed to the resumed `Built` branch.
    /// Recovery must never rebuild a replacement transaction/outpoint.
    #[derive(Default)]
    struct RecordingBroadcaster {
        transactions: Mutex<Vec<Transaction>>,
    }

    #[async_trait]
    impl TransactionBroadcaster for RecordingBroadcaster {
        async fn broadcast(&self, transaction: &Transaction) -> Result<Txid, BroadcastError> {
            self.transactions
                .lock()
                .expect("recording broadcaster mutex")
                .push(transaction.clone());
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

    /// A DEFINITE rejection is the opposite case and must keep failing the
    /// resume: nothing is on the network, so no proof can ever arrive, and
    /// the lock stays at `Built` for a later retry to re-send.
    #[tokio::test]
    async fn built_resume_still_fails_on_a_definite_rejection() {
        let (error, status) = resume_built_lock_with(Arc::new(AlwaysRejectedBroadcaster)).await;

        assert!(
            matches!(error, PlatformWalletError::TransactionBroadcast(_)),
            "a definite rejection must surface as a broadcast failure: {error:?}"
        );
        assert_eq!(
            status,
            AssetLockStatus::Built,
            "a tx that never entered the network must stay resumable at Built"
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
