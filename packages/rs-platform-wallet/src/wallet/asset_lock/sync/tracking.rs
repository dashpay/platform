//! Asset lock state management: tracking, removing, and advancing status.

use crate::broadcaster::TransactionBroadcaster;
use dashcore::OutPoint;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::PersistenceCapabilities;
use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
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
    /// tracked. Guarded on the row still being
    /// [`Built`](AssetLockStatus::Built): if a concurrent flow advanced it
    /// (e.g. a `resume_asset_lock` that re-broadcast in the window between
    /// the rejected broadcast and this cleanup), the progress is kept
    /// rather than clobbered. The caller queues the changeset (call sites
    /// live in `asset_lock/build.rs`, inside the module).
    pub(crate) async fn untrack_asset_lock(&self, out_point: &OutPoint) -> AssetLockChangeSet {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = AssetLockChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            match info.tracked_asset_locks.get(out_point) {
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

    /// Remove a tracked asset lock that is sitting at
    /// [`Broadcast`](AssetLockStatus::Broadcast) **without a proof** after a
    /// re-broadcast came back definitively [`Rejected`].
    ///
    /// Companion to [`untrack_asset_lock`](Self::untrack_asset_lock), kept
    /// separate rather than folded into it because the two have different
    /// safety obligations. `untrack_asset_lock` is the build-time rejection
    /// path, and its caller in `asset_lock/build.rs` uses "the row was
    /// removed" as the trigger to RELEASE the funding-input reservation.
    /// There, a row that advanced to `Broadcast` concurrently is treated as
    /// positive evidence the transaction reached the network, so the guard
    /// deliberately keeps it and holds the reservation. Widening that method
    /// to remove `Broadcast` rows would release reservations for inputs
    /// whose transaction may be live — a double-spend opening.
    ///
    /// This method is only reached from `resume_asset_lock` after the
    /// broadcaster returned `Rejected`, which it does only when the send
    /// provably did not happen. It releases no reservation.
    ///
    /// Guards, in addition to the status check:
    ///
    /// - `proof.is_none()` — a row carrying an IS/CL proof has authenticated
    ///   on-chain evidence that outranks any broadcast verdict.
    /// - Only `Broadcast` is removed. [`Consumed`](AssetLockStatus::Consumed)
    ///   is a terminal tombstone that must survive (#4347), and
    ///   `InstantSendLocked` / `ChainLocked` / `RecoveredFromChain` all imply
    ///   finality that a broadcast rejection cannot contradict.
    ///
    /// Idempotent: an empty changeset when the outpoint is untracked or
    /// fails a guard.
    pub(crate) async fn untrack_unproven_broadcast_asset_lock(
        &self,
        out_point: &OutPoint,
    ) -> AssetLockChangeSet {
        let mut wm = self.wallet_manager.write().await;
        let mut cs = AssetLockChangeSet::default();
        if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
            match info.tracked_asset_locks.get(out_point) {
                Some(entry)
                    if entry.status == AssetLockStatus::Broadcast && entry.proof.is_none() =>
                {
                    info.tracked_asset_locks.remove(out_point);
                    cs.removed.insert(*out_point);
                }
                Some(entry) => tracing::warn!(
                    outpoint = %out_point,
                    status = ?entry.status,
                    has_proof = entry.proof.is_some(),
                    "untrack_unproven_broadcast_asset_lock: lock is not an unproven \
                     Broadcast row — leaving it tracked"
                ),
                None => {}
            }
        }
        cs
    }

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
        Ok(cs)
    }
}
