//! Asset lock state management: tracking, removing, and advancing status.

use crate::broadcaster::TransactionBroadcaster;
use dashcore::OutPoint;

use crate::changeset::changeset::AssetLockChangeSet;
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
    /// `pub(super)` to the `asset_lock` module, so the callers of
    /// `consume_asset_lock` outside it — the identity registration
    /// and top-up success paths in
    /// `wallet/identity/network/registration.rs` (the failure paths
    /// go through [`settle_reported_consumed`](Self::settle_reported_consumed))
    /// — can't queue the changeset themselves. The other mutators are
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

    /// Classify a Platform rejection of a funded submission that carried
    /// `submitted`'s proof, settling the tracked lock when — and only
    /// when — Platform's "already completely used" verdict names that
    /// same outpoint.
    ///
    /// The verdict arrives as an unauthenticated consensus error relayed
    /// by a single DAPI node, so it is trusted no further than the lock
    /// whose proof this very submission carried: a verdict naming any
    /// *other* outpoint is passed through as a plain
    /// [`Sdk`](PlatformWalletError::Sdk) error instead of tombstoning an
    /// unrelated lock (which would wipe its proof and strand real burned
    /// funds on a fabricated answer).
    ///
    /// On a bound verdict the credits the lock paid for already landed —
    /// an earlier attempt succeeded and the client never learned. Marking
    /// the lock [`Consumed`](AssetLockStatus::Consumed) takes it out of
    /// the resumable set, which is what stops a recovery worker retrying
    /// it against the same deterministic rejection on every pass (and,
    /// for clients that block new funding while a lock is unresolved,
    /// unblocks the next purchase). Returns
    /// [`AssetLockAlreadyConsumed`](PlatformWalletError::AssetLockAlreadyConsumed)
    /// — the same typed error a resume of a consumed lock raises, so
    /// callers need one terminal case, not a Platform error-string match.
    /// A bookkeeping failure here can only be `WalletNotFound`; it is
    /// logged rather than returned, because the verdict itself is what
    /// the caller must act on.
    pub(crate) async fn settle_reported_consumed(
        &self,
        error: dash_sdk::Error,
        submitted: &OutPoint,
    ) -> PlatformWalletError {
        match crate::error::asset_lock_already_consumed_out_point(&error) {
            Some(reported) if reported == *submitted => {
                tracing::info!(
                    outpoint = %reported,
                    "Platform rejected the asset lock as already completely used — its \
                     credits landed on an earlier attempt; marking the lock consumed"
                );
                if let Err(e) = self.consume_asset_lock(&reported).await {
                    tracing::warn!(
                        outpoint = %reported,
                        error = %e,
                        "consume_asset_lock failed after Platform's already-used rejection"
                    );
                }
                PlatformWalletError::AssetLockAlreadyConsumed(reported)
            }
            Some(reported) => {
                tracing::warn!(
                    reported = %reported,
                    submitted = %submitted,
                    "Platform's already-used verdict names a different outpoint than the \
                     one submitted; not settling any local lock"
                );
                PlatformWalletError::Sdk(error)
            }
            None => PlatformWalletError::Sdk(error),
        }
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
