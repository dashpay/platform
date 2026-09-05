//! Asset lock state management: tracking, removing, and advancing status.

use crate::broadcaster::TransactionBroadcaster;
use dashcore::OutPoint;
use std::collections::BTreeMap;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::PersistenceCapabilities;
use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

/// One resume's hold on an outpoint's cleanup-exclusion window.
///
/// Held from the moment the resume snapshots the tracked row until it has
/// sent the transaction and recorded that send by advancing the row — the
/// span in which the resume is committed to broadcasting a transaction it
/// has already read out of the map. While it stands,
/// [`untrack_asset_lock`](AssetLockManager::untrack_asset_lock) refuses to
/// remove the row, which is what keeps the initial build's rejection
/// cleanup from releasing the funding reservation and the in-broadcast
/// fence under a send that is still coming.
///
/// A claim releases on drop until the resume observes local finality or
/// enters a broadcast that may have side effects. From that point it becomes
/// sticky: cancellation cannot prove that releasing the inputs is safe, so
/// the exclusion survives until a status transition makes the `Built`
/// cleanup inapplicable. Exits proven to be pre-dispatch restore ordinary
/// RAII release.
pub(crate) struct ResumeDispatchClaim<'a> {
    claims: &'a std::sync::Mutex<BTreeMap<OutPoint, usize>>,
    out_point: OutPoint,
    release_on_drop: bool,
}

impl ResumeDispatchClaim<'_> {
    /// Preserve cleanup exclusion if this future is cancelled before it can
    /// record the transaction's send or proof.
    pub(crate) fn preserve_on_drop(&mut self) {
        self.release_on_drop = false;
    }

    /// Restore ordinary RAII release after the current attempt is proven not
    /// to have left the device and no local finality was observed.
    pub(crate) fn release_on_drop(&mut self) {
        self.release_on_drop = true;
    }
}

impl Drop for ResumeDispatchClaim<'_> {
    fn drop(&mut self) {
        if !self.release_on_drop {
            return;
        }
        // Recover from poisoning rather than skipping the release: a claim
        // that outlived its resume would block the rejection cleanup — and
        // with it the funding reservation's release — for the process's
        // remaining lifetime.
        let mut claims = self.claims.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = claims.get_mut(&self.out_point) {
            *count -= 1;
            if *count == 0 {
                claims.remove(&self.out_point);
            }
        }
    }
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Claim `out_point`'s dispatch window for a resume that is about to
    /// broadcast.
    ///
    /// MUST be called while the resume still holds the wallet read guard it
    /// snapshotted the tracked row under. That is where the serialization
    /// against the rejection cleanup comes from: the cleanup reads the claim
    /// under the wallet WRITE guard, so a claim taken under the read guard is
    /// either already visible to it — and the row is kept — or the cleanup
    /// went first, removed the row, and the snapshot the claim would have
    /// protected never happened.
    pub(crate) fn claim_resume_dispatch(&self, out_point: OutPoint) -> ResumeDispatchClaim<'_> {
        *self
            .resume_dispatch_claims
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(out_point)
            .or_insert(0) += 1;
        ResumeDispatchClaim {
            claims: &self.resume_dispatch_claims,
            out_point,
            release_on_drop: true,
        }
    }

    /// Whether active or sticky resume state excludes rejected-build cleanup.
    fn resume_cleanup_excluded(&self, out_point: &OutPoint) -> bool {
        self.resume_dispatch_claims
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(out_point)
    }

    /// Snapshot the funding role, bound index and status used to authorize an
    /// existing-lock resume. Taking all three under one read lock avoids a
    /// role/status time-of-check/time-of-use split in the resolver.
    ///
    /// [`AssetLockFundingType`]:
    /// key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType
    pub(crate) async fn tracked_resume_metadata(
        &self,
        out_point: &OutPoint,
    ) -> Option<(
        key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType,
        u32,
        AssetLockStatus,
    )> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.tracked_asset_locks.get(out_point))
            .map(|lock| (lock.funding_type, lock.identity_index, lock.status.clone()))
    }

    /// Track a new asset lock in memory, returning a changeset describing
    /// the inserted entry.
    ///
    /// If an entry already exists at `out_point`, it is overwritten.
    pub(crate) async fn track_asset_lock(&self, lock: TrackedAssetLock) -> AssetLockChangeSet {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = AssetLockChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            let out_point = lock.out_point;
            cs.asset_locks.insert(out_point, (&lock).into());
            info.tracked_asset_locks.insert(out_point, lock);
        }
        cs
    }

    /// Remove a tracked asset lock whose funding transaction was
    /// definitively rejected at broadcast (never reached the network).
    ///
    /// Unlike [`consume_asset_lock`](Self::consume_asset_lock), the
    /// persisted row is deleted too (via the changeset's `removed` set):
    /// a rejected lock's funding transaction never existed on the
    /// network, so the row has no historical value — and because the
    /// rejection released the funding UTXO reservation, leaving the
    /// `Built` row resumable would let a later `resume_asset_lock`
    /// re-broadcast a transaction whose inputs may have been re-spent.
    ///
    /// Idempotent: returns an empty changeset if the outpoint is not
    /// tracked. Guarded twice over, and both guards mean the same thing —
    /// that the transaction may be live or committed to dispatch, so the row
    /// and everything that pins its inputs must stay:
    ///
    /// 1. The row must still be [`Built`](AssetLockStatus::Built). A resume
    ///    that already re-broadcast advanced it, and that advance is
    ///    positive evidence the transaction reached the network.
    /// 2. No active or sticky cleanup exclusion may remain
    ///    ([`claim_resume_dispatch`](Self::claim_resume_dispatch)). A resume
    ///    that has snapshotted the row but not yet sent is still `Built`, and
    ///    cancellation after a possible send or observed proof can leave it
    ///    there. Guard 1 alone cannot distinguish either case from a clean
    ///    pre-dispatch exit.
    ///
    /// Refusing on either guard is what the caller reads back out of the
    /// changeset: an empty `removed` set keeps the funding reservation and
    /// the in-broadcast fence held and turns the definite-rejection verdict
    /// into the unknown outcome, whose contract — row tracked, inputs
    /// reserved, do not retry — is the one that actually holds. The caller
    /// queues the changeset (call sites live in `asset_lock/build.rs`,
    /// inside the module).
    pub(crate) async fn untrack_asset_lock(&self, out_point: &OutPoint) -> AssetLockChangeSet {
        let mut wm = self.wallet_manager.write().await;
        // Read under the write guard, which is what makes this atomic
        // against a resume claiming the window under its read guard.
        let cleanup_excluded = self.resume_cleanup_excluded(out_point);
        let mut cs = AssetLockChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            match info.tracked_asset_locks.get(out_point) {
                Some(_) if cleanup_excluded => tracing::warn!(
                    outpoint = %out_point,
                    "untrack_asset_lock: resume state excludes cleanup of this Built lock — \
                     leaving it tracked, since its transaction may already be live or \
                     committed to dispatch"
                ),
                Some(entry) if entry.status == AssetLockStatus::Built => {
                    info.tracked_asset_locks.remove(out_point);
                    cs.removed.insert(*out_point);
                }
                Some(entry) => tracing::warn!(
                    outpoint = %out_point,
                    status = ?entry.status,
                    "untrack_asset_lock: lock advanced past Built concurrently — leaving it tracked"
                ),
                None => {}
            }
        }
        cs
    }

    // NOTE: there is deliberately no `untrack_unproven_broadcast_asset_lock`
    // companion here. A `Rejected` verdict from a re-broadcast describes only
    // that attempt (with the production `SpvBroadcaster`: an unstarted client
    // or zero connected peers), never the ORIGINAL broadcast that moved the
    // row to `Broadcast` in an earlier process — so it is not evidence that
    // the transaction is absent from the network, and removing the row on it
    // would delete tracking for possibly-mined asset locks during ordinary
    // offline relaunches. `resume_asset_lock` surfaces the typed error and
    // leaves the row untouched.

    /// Mark a tracked asset lock as
    /// [`Consumed`](AssetLockStatus::Consumed) after a successful
    /// identity registration or top-up.
    ///
    /// Two-sided semantics:
    ///
    /// 1. **In-memory** (`tracked_asset_locks`) — the entry is
    ///    removed. The wallet has no further use for a consumed lock
    ///    (proof is one-shot, status will never advance again), so
    ///    keeping it in memory just costs heap.
    /// 2. **Persisted** (`PersistentAssetLock` on the Swift side) —
    ///    the row is retained with `status = Consumed`. Historical
    ///    UI lookups (e.g. the Transactions list mapping a funding
    ///    tx row back to its locked amount) need this row to
    ///    survive consumption.
    ///
    /// The changeset carries only `asset_locks` (with the consumed
    /// entry). It deliberately does NOT populate `removed`: that set
    /// drives the Swift persister to delete the row, which would
    /// undo (2). Apply-side replay in `apply.rs` detects the
    /// `Consumed` status and drops the in-memory entry on the
    /// Rust side, mirroring this method's runtime behavior.
    ///
    /// Distinguishes three outcomes via the result:
    ///
    /// - `Ok(changeset)` with a populated `asset_locks` entry — the
    ///   lock was tracked and is now `Consumed`. The changeset has
    ///   ALREADY been queued for persistence before return; the value
    ///   is surfaced for tests / future internal callers that want to
    ///   inspect the diff.
    /// - `Ok(empty changeset)` — the lock is already marked consumed,
    ///   or was never tracked. Idempotent; nothing queued.
    /// - `Err(WalletNotFound)` — the wallet id is unknown to the
    ///   manager. Always a programmer error / stale handle.
    ///
    /// **Why queue internally** (unlike `track_asset_lock` /
    /// `advance_asset_lock_status`, which return a changeset and let
    /// the caller queue it): `queue_asset_lock_changeset` is
    /// `pub(super)` to the `asset_lock` module, so the only callers
    /// of `consume_asset_lock` — the identity registration and
    /// top-up flows in `wallet/identity/network/registration.rs` —
    /// can't queue the changeset themselves. The other mutators are
    /// only called from inside `asset_lock/build.rs`, which IS in
    /// the module and queues at the call site. Queueing here closes
    /// the gap without widening the `pub(super)` visibility.
    pub(crate) async fn consume_asset_lock(
        &self,
        out_point: &OutPoint,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        // Build the changeset under the write lock, then release the
        // lock before queueing — `queue_asset_lock_changeset` calls
        // the persister synchronously and we don't want to hold the
        // wallet-manager write lock across that.
        let cs = {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let mut cs = AssetLockChangeSet::default();
            match info.tracked_asset_locks.get_mut(out_point) {
                Some(entry) if entry.status != AssetLockStatus::Consumed => {
                    entry.status = AssetLockStatus::Consumed;
                    entry.proof = None; // one-shot — never relevant after consumption
                    cs.asset_locks.insert(*out_point, (&*entry).into());
                }
                Some(_) => {
                    tracing::debug!(
                        outpoint = %out_point,
                        "consume_asset_lock: outpoint is already consumed"
                    );
                }
                None => {
                    tracing::warn!(
                        outpoint = %out_point,
                        "consume_asset_lock: outpoint was never tracked"
                    );
                }
            }
            cs
        };
        self.queue_asset_lock_changeset(cs.clone());
        Ok(cs)
    }

    /// Record that Platform reported an asset lock as already consumed without
    /// claiming authenticated Platform-side completion.
    ///
    /// The supplied proof must be a Core-chain-authenticated ChainLock proof.
    /// The row moves to [`RecoveredFromChain`](AssetLockStatus::RecoveredFromChain),
    /// whose contract is deliberately nonterminal: Core finality is known, but
    /// Platform consumption is not. The proof is retained so a future explicit
    /// recovery remains possible.
    ///
    /// Unlike the ordinary queued status updates, this user-visible recovery
    /// marker is stored and flushed synchronously. A failure rolls back the
    /// in-memory mutation only when the backend has not committed the store and
    /// did not retain a transient retry buffer. Before mutating, the backend
    /// must attest atomic tracked-asset-lock persistence and restart restore.
    pub(crate) async fn mark_asset_lock_consumption_unknown(
        &self,
        out_point: &OutPoint,
        chain_proof: dpp::prelude::AssetLockProof,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        if !matches!(&chain_proof, dpp::prelude::AssetLockProof::Chain(_)) {
            return Err(PlatformWalletError::AssetLockProofWait(format!(
                "Asset lock {} cannot enter consumption-unknown state without a ChainLock proof",
                out_point
            )));
        }

        let capabilities = self.persister.persistence_capabilities();
        let required = PersistenceCapabilities::ASSET_LOCK_RECONCILIATION;
        if !capabilities.contains(required) {
            let missing = capabilities.missing(required);
            return Err(PlatformWalletError::Persistence(format!(
                "asset-lock reconciliation requires persistence capabilities {:?} \
                 (missing mask 0x{:x})",
                missing.names(),
                missing.bits(),
            )));
        }

        let (previous, candidate, cs) = {
            let mut wm = self.wallet_manager.write().await;
            let info = wm
                .get_wallet_info_mut(&self.wallet_id)
                .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
            let entry = info
                .tracked_asset_locks
                .get_mut(out_point)
                .ok_or_else(|| PlatformWalletError::AssetLockNotTracked(*out_point))?;
            let previous = entry.clone();
            entry.status = AssetLockStatus::RecoveredFromChain;
            entry.proof = Some(chain_proof);
            let candidate = entry.clone();
            let mut cs = AssetLockChangeSet::default();
            cs.asset_locks.insert(*out_point, (&*entry).into());
            (previous, candidate, cs)
        };

        let store_commits_inline = self.persister.store_commits_inline();
        let persist_failure = match self.persister.store(PlatformWalletChangeSet {
            asset_locks: Some(cs.clone()),
            ..Default::default()
        }) {
            Err(error) => Some((error, true)),
            Ok(()) => self.persister.flush().err().map(|error| {
                let may_rollback = !store_commits_inline;
                (error, may_rollback)
            }),
        };
        if let Some((error, may_rollback)) = persist_failure {
            if error.is_transient() || !may_rollback {
                return Err(PlatformWalletError::Persistence(error.to_string()));
            }
            let mut wm = self.wallet_manager.write().await;
            if let Some(current) = wm
                .get_wallet_info_mut(&self.wallet_id)
                .and_then(|info| info.tracked_asset_locks.get_mut(out_point))
            {
                // Do not overwrite a concurrent lifecycle advance. Roll back
                // only while the exact candidate written above is still live.
                if current.status == candidate.status && current.proof == candidate.proof {
                    *current = previous;
                }
            }
            return Err(PlatformWalletError::Persistence(error.to_string()));
        }

        Ok(cs)
    }

    /// Advance the status of a tracked asset lock and optionally attach the proof.
    ///
    /// Returns an [`AssetLockChangeSet`] carrying a full snapshot of the
    /// updated entry.
    pub(crate) async fn advance_asset_lock_status(
        &self,
        out_point: &OutPoint,
        new_status: AssetLockStatus,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let entry = info.tracked_asset_locks.get_mut(out_point).ok_or_else(|| {
            PlatformWalletError::AssetLockProofWait(format!(
                "Asset lock {} is not tracked",
                out_point
            ))
        })?;
        entry.status = new_status;
        if proof.is_some() {
            entry.proof = proof;
        }

        let mut cs = AssetLockChangeSet::default();
        cs.asset_locks.insert(*out_point, (&*entry).into());
        if entry.status != AssetLockStatus::Built {
            // The status now excludes rejected-build cleanup on its own.
            // Clear both active claims and sticky claims left by cancelled
            // resumes; their eventual drops tolerate the absent entry.
            self.resume_dispatch_claims
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(out_point);
        }
        Ok(cs)
    }
}
