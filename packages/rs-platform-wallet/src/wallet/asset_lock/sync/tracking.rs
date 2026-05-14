//! Asset lock state management: tracking, removing, and advancing status.

use crate::broadcaster::TransactionBroadcaster;
use dashcore::OutPoint;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
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
    ///   lock was tracked and is now `Consumed`. The changeset upserts
    ///   the persisted row with `statusRaw=4` for historical lookup.
    /// - `Ok(empty changeset)` — the lock was not tracked (already
    ///   consumed by a prior call, or never present). Idempotent.
    /// - `Err(WalletNotFound)` — the wallet id is unknown to the
    ///   manager. Always a programmer error / stale handle.
    pub(crate) async fn consume_asset_lock(
        &self,
        out_point: &OutPoint,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = wm
            .get_wallet_info_mut(&self.wallet_id)
            .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(self.wallet_id)))?;
        let mut cs = AssetLockChangeSet::default();
        if let Some(mut entry) = info.tracked_asset_locks.remove(out_point) {
            entry.status = AssetLockStatus::Consumed;
            entry.proof = None; // one-shot — never relevant after consumption
            cs.asset_locks.insert(*out_point, (&entry).into());
        } else {
            tracing::warn!(
                outpoint = %out_point,
                "consume_asset_lock: outpoint not tracked — already consumed or never present"
            );
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
