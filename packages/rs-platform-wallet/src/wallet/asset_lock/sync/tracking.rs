//! Asset lock state management: tracking, removing, and advancing status.

use crate::broadcaster::TransactionBroadcaster;
use dashcore::OutPoint;

use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;

use super::super::manager::AssetLockManager;
use super::super::tracked::{AssetLockStatus, TrackedAssetLock};

/// Outcome of the compare-and-set `Built` → `Broadcast` promotion in
/// [`AssetLockManager::promote_built_to_broadcast`].
#[derive(Debug)]
pub(crate) enum BuiltPromotion {
    /// The row was still `Built` and is now `Broadcast`. Carries the
    /// changeset, which has ALREADY been queued for persistence before
    /// return — the value is surfaced only so tests and callers can
    /// inspect the diff.
    Promoted(AssetLockChangeSet),
    /// A concurrent flow already advanced the row past `Built`. Nothing
    /// was mutated; the caller re-dispatches from these values rather
    /// than overwriting them.
    AlreadyAdvanced {
        current_status: AssetLockStatus,
        current_proof: Option<dpp::prelude::AssetLockProof>,
    },
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// The recorded [`AssetLockFundingType`] of a tracked lock, or `None`
    /// when the outpoint is not tracked. Used by the funding resolver to
    /// refuse consuming `IdentityInvitation` (bearer voucher) locks through
    /// generic resume/top-up paths.
    ///
    /// [`AssetLockFundingType`]:
    /// key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType
    pub(crate) async fn tracked_funding_type(
        &self,
        out_point: &OutPoint,
    ) -> Option<key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType>
    {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .and_then(|info| info.tracked_asset_locks.get(out_point))
            .map(|lock| lock.funding_type)
    }

    /// Track a new asset lock in memory, returning a changeset describing
    /// the inserted entry. The changeset has ALREADY been queued for
    /// persistence before return; the value is surfaced so callers and
    /// tests can inspect the diff.
    ///
    /// If an entry already exists at `out_point`, it is overwritten.
    ///
    /// # Ordering
    ///
    /// Mutation and enqueue happen as one unit under
    /// [`status_persist_serial`](AssetLockManager::status_persist_serial),
    /// for the same reason the promotion does. The insert is what makes
    /// the row visible to `resume_asset_lock`, so the moment the wallet
    /// lock drops a resume can promote it to `Broadcast` and enqueue
    /// that — and an unserialized `Built` enqueue landing afterwards
    /// would regress the durable row to the pre-broadcast state.
    pub(crate) async fn track_asset_lock(&self, lock: TrackedAssetLock) -> AssetLockChangeSet {
        let _serial = self.status_persist_serial.lock().await;

        let cs = {
            let mut wm = self.wallet_manager.write().await;
            let mut cs = AssetLockChangeSet::default();
            if let Some(info) = wm.get_wallet_info_mut(&self.wallet_id) {
                let out_point = lock.out_point;
                cs.asset_locks.insert(out_point, (&lock).into());
                info.tracked_asset_locks.insert(out_point, lock);
            }
            cs
        };

        self.queue_asset_lock_changeset(cs.clone());
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
    /// rather than clobbered.
    ///
    /// The changeset has ALREADY been queued for persistence before
    /// return; the value is surfaced so the caller can tell whether the
    /// row was actually removed (`removed` non-empty) — `build.rs` gates
    /// releasing the funding reservation on exactly that.
    ///
    /// # Ordering
    ///
    /// Mutation and enqueue happen as one unit under
    /// [`status_persist_serial`](AssetLockManager::status_persist_serial).
    /// The `Built` guard is itself a compare-and-set against the same
    /// concurrent finalizer, so it needs the same protection: were the
    /// row-deleting `removed` enqueue to land after a concurrent
    /// finalize's proof-bearing snapshot, Swift would delete the row
    /// that snapshot had just written.
    pub(crate) async fn untrack_asset_lock(&self, out_point: &OutPoint) -> AssetLockChangeSet {
        let _serial = self.status_persist_serial.lock().await;

        let cs = {
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
        };

        self.queue_asset_lock_changeset(cs.clone());
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
    /// - `Ok(empty changeset)` — the lock was not tracked (already
    ///   consumed by a prior call, or never present). Idempotent;
    ///   nothing queued.
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
        // Hold the ordering mutex across mutate → enqueue so the
        // terminal `Consumed` snapshot cannot be overtaken by a
        // concurrent status writer's older one (see
        // `status_persist_serial`). Acquired BEFORE `wallet_manager`.
        let _serial = self.status_persist_serial.lock().await;

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
            cs
        };
        self.queue_asset_lock_changeset(cs.clone());
        Ok(cs)
    }

    /// Compare-and-set the pre-broadcast
    /// [`Built`](AssetLockStatus::Built) →
    /// [`Broadcast`](AssetLockStatus::Broadcast) promotion, under the
    /// wallet write lock.
    ///
    /// Shared by BOTH `Built` → `Broadcast` writers, because both hold a
    /// stale view of the row across an unbounded `broadcast(&tx)` await:
    ///
    /// - `resume_asset_lock` snapshots the row under a *read* lock and
    ///   drops it before promoting, so two resumes can both observe
    ///   `Built`.
    /// - `broadcast_funded_asset_lock` tracks the row as `Built`, then
    ///   awaits its own broadcast before promoting — during which a
    ///   concurrent resume can pick the same outpoint up.
    ///
    /// Either way, if the other flow goes on to broadcast, obtain a proof
    /// and store `InstantSendLocked` / `ChainLocked` with that proof
    /// attached, an unconditional write from the delayed caller would
    /// downgrade the finalized row back to `Broadcast` while leaving the
    /// proof attached — an inconsistent `Broadcast + Some(proof)` state
    /// that also gets persisted (changesets are last-write-wins). A later
    /// resume would then take the `Broadcast` arm and wait for a proof it
    /// already holds.
    ///
    /// So the promotion only fires while the row is *still* `Built`.
    /// Otherwise nothing is mutated and the caller is handed the row's
    /// current status and proof. `resume_asset_lock` re-dispatches from
    /// them; the create path has nothing to re-dispatch (its proof wait
    /// lives in a separate method) and simply leaves them intact.
    ///
    /// Still errors when the outpoint is untracked: that means a
    /// concurrent `untrack_asset_lock` removed a rejected row (releasing
    /// its funding reservation), and the caller must abort before
    /// re-broadcasting a transaction whose inputs are re-spendable.
    ///
    /// # Ordering
    ///
    /// The compare-and-set and the persistence enqueue happen as one
    /// unit under [`status_persist_serial`](AssetLockManager::status_persist_serial),
    /// so a promotion can never enqueue its `Broadcast + None` snapshot
    /// after a concurrent finalize enqueued a newer proof-bearing one.
    /// Both `Built` → `Broadcast` callers (create and resume) go through
    /// here, so neither can reorder against the other or against
    /// `advance_asset_lock_status`.
    pub(crate) async fn promote_built_to_broadcast(
        &self,
        out_point: &OutPoint,
    ) -> Result<BuiltPromotion, PlatformWalletError> {
        // Hold the ordering mutex across mutate → enqueue. Acquired
        // BEFORE `wallet_manager` (see the field's lock-ordering note).
        let _serial = self.status_persist_serial.lock().await;

        let promotion = self.compare_and_set_built_to_broadcast(out_point).await?;

        // Test-only pause in the post-CAS / pre-enqueue window. Under the
        // serialization above a concurrent finalize now blocks here rather
        // than slipping its newer snapshot in ahead of ours.
        #[cfg(test)]
        {
            let gate = self
                .promote_post_cas_gate
                .lock()
                .expect("promote post-CAS gate mutex")
                .clone();
            if let Some(gate) = gate {
                gate.arrived.notify_one();
                gate.release.notified().await;
            }
        }

        if let BuiltPromotion::Promoted(ref cs) = promotion {
            self.queue_asset_lock_changeset(cs.clone());
        }
        Ok(promotion)
    }

    /// The compare-and-set half of
    /// [`promote_built_to_broadcast`](Self::promote_built_to_broadcast):
    /// mutates the in-memory row under the wallet write lock and returns
    /// the resulting changeset WITHOUT queueing it.
    ///
    /// Private and callable only from `promote_built_to_broadcast`, which
    /// owns the ordering mutex — splitting it out keeps the wallet write
    /// guard scoped to the mutation so it is released before the
    /// synchronous persister call.
    async fn compare_and_set_built_to_broadcast(
        &self,
        out_point: &OutPoint,
    ) -> Result<BuiltPromotion, PlatformWalletError> {
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

        if entry.status != AssetLockStatus::Built {
            return Ok(BuiltPromotion::AlreadyAdvanced {
                current_status: entry.status.clone(),
                current_proof: entry.proof.clone(),
            });
        }

        entry.status = AssetLockStatus::Broadcast;
        let mut cs = AssetLockChangeSet::default();
        cs.asset_locks.insert(*out_point, (&*entry).into());
        Ok(BuiltPromotion::Promoted(cs))
    }

    /// Advance the status of a tracked asset lock and optionally attach the proof.
    ///
    /// Returns an [`AssetLockChangeSet`] carrying a full snapshot of the
    /// updated entry. The changeset has ALREADY been queued for
    /// persistence before return; the value is surfaced so callers and
    /// tests can inspect the diff.
    ///
    /// Assigns unconditionally — callers that race another writer for the
    /// same row must gate the write themselves (see
    /// [`promote_built_to_broadcast`](Self::promote_built_to_broadcast)).
    ///
    /// # Ordering
    ///
    /// Like the promotion, the mutation and the enqueue happen as one
    /// unit under [`status_persist_serial`](AssetLockManager::status_persist_serial).
    /// This is the finalizing half of the pair: without the shared mutex
    /// a proof-bearing `InstantSendLocked` / `ChainLocked` snapshot can
    /// be enqueued BEFORE a concurrently-delayed promoter enqueues its
    /// older `Broadcast + None` one, regressing the durable row.
    pub(crate) async fn advance_asset_lock_status(
        &self,
        out_point: &OutPoint,
        new_status: AssetLockStatus,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        // Hold the ordering mutex across mutate → enqueue. Acquired
        // BEFORE `wallet_manager` (see the field's lock-ordering note).
        let _serial = self.status_persist_serial.lock().await;

        // Scoped so the wallet write guard is released before the
        // synchronous persister call below.
        let cs = {
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
            cs
        };

        self.queue_asset_lock_changeset(cs.clone());
        Ok(cs)
    }
}
