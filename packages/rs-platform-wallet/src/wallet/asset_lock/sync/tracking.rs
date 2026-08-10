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
    ///
    /// # Lifecycle
    ///
    /// Errors with
    /// [`AssetLockManagerInactive`](PlatformWalletError::AssetLockManagerInactive)
    /// when the owning wallet has been removed. Checked under the
    /// ordering mutex, so a removal racing this call either completes
    /// first (and this insert is refused) or waits for it (and the row
    /// belongs to the wallet being removed, not to a replacement).
    pub(crate) async fn track_asset_lock(
        &self,
        lock: TrackedAssetLock,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        let _serial = self.lock_status_persist_serial().await;
        self.ensure_active_under_serial(&_serial)?;

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
        Ok(cs)
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
    ///
    /// # Lifecycle
    ///
    /// Errors with
    /// [`AssetLockManagerInactive`](PlatformWalletError::AssetLockManagerInactive)
    /// when the owning wallet has been removed — checked under the
    /// ordering mutex. This is the arm that matters most for a stale
    /// handle: the `removed` set DELETES the durable row, so a retired
    /// manager reaching this method after a re-import would erase a
    /// replacement wallet's freshly-tracked lock.
    pub(crate) async fn untrack_asset_lock(
        &self,
        out_point: &OutPoint,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        let _serial = self.lock_status_persist_serial().await;
        self.ensure_active_under_serial(&_serial)?;

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
        Ok(cs)
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
    /// - `Err(AssetLockManagerInactive)` — the owning wallet was
    ///   removed from the `PlatformWalletManager`. Checked under the
    ///   ordering mutex, before the wallet lookup, so a re-imported
    ///   wallet that happens to share the same deterministic id cannot
    ///   have its rows consumed through the retired handle.
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
        let _serial = self.lock_status_persist_serial().await;
        // Authoritative stale-handle check: under the mutex, before the
        // wallet lookup below can resolve a REPLACEMENT wallet that
        // re-import installed under the same deterministic id.
        self.ensure_active_under_serial(&_serial)?;

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
    ///
    /// # Lifecycle
    ///
    /// Errors with
    /// [`AssetLockManagerInactive`](PlatformWalletError::AssetLockManagerInactive)
    /// when the owning wallet has been removed. Both callers reach this
    /// method only after an unbounded `broadcast(&tx)` await, so the
    /// under-mutex check — not the entry-point one — is what stops a
    /// resume that began before the removal from writing to a
    /// replacement wallet's row.
    pub(crate) async fn promote_built_to_broadcast(
        &self,
        out_point: &OutPoint,
    ) -> Result<BuiltPromotion, PlatformWalletError> {
        // Hold the ordering mutex across mutate → enqueue. Acquired
        // BEFORE `wallet_manager` (see the field's lock-ordering note).
        let _serial = self.lock_status_persist_serial().await;
        self.ensure_active_under_serial(&_serial)?;

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
    /// tests can inspect the diff. When the write is refused as a
    /// downgrade (below) the changeset is EMPTY and nothing was queued.
    ///
    /// Forward-only within the [`AssetLockStatus`] lifecycle — callers
    /// that race another writer for the same row need no gate of their
    /// own for the *backward* case. A `Built` → `Broadcast` race still
    /// needs [`promote_built_to_broadcast`](Self::promote_built_to_broadcast),
    /// which is a different question (both writers agree on the target
    /// status; the loser must learn the row already advanced so it can
    /// re-dispatch).
    ///
    /// # Monotonicity
    ///
    /// A write whose `new_status` ranks strictly BELOW the row's current
    /// status (see [`AssetLockStatus::lifecycle_rank`]) mutates nothing,
    /// replaces no proof, and enqueues no snapshot.
    ///
    /// This is not the same race the ordering mutex closes. The mutex
    /// makes each writer's mutation and enqueue one indivisible unit, so
    /// the durable order matches the in-memory order — but two writers
    /// that are each internally consistent can still arrive in the wrong
    /// ORDER, because the IS-lock and ChainLock waiters are independent:
    /// `wait_for_proof` returns whichever SPV event fires first, and the
    /// IS→CL upgrade paths (`upgrade_to_chain_lock_proof` after a
    /// Platform IS rejection, `validate_or_upgrade_proof` on a stale
    /// quorum) can complete a ChainLocked write while an earlier IS
    /// waiter is still parked. Released afterwards, that IS waiter would
    /// write `InstantSendLocked` over `ChainLocked` and — since it
    /// passes `Some(is_proof)` — swap the ChainLock proof out for the IS
    /// one, in memory and durably.
    ///
    /// Both halves matter. The status regression alone puts the row
    /// below the `>= InstantSendLocked` predicates the catch-up scanner
    /// and the "ready to fund" UI filter on. The proof swap is worse:
    /// the ChainLock proof is the one that survives quorum rotation, and
    /// it is what an IS rejection retry upgraded TO — reinstating the IS
    /// proof re-arms exactly the rejection that upgrade resolved, and a
    /// restart reloads the weaker proof from the durable row.
    ///
    /// **Caller-return semantics are deliberately unchanged.** A refused
    /// caller gets `Ok(empty changeset)`, not an error: its own proof is
    /// still valid evidence for the submission it is about to make, and
    /// every production caller returns/uses the proof it already holds
    /// rather than anything from the changeset. So the delayed IS caller
    /// may go on submitting with its IS proof; only the SHARED state
    /// (in-memory row and durable row) is held at the stronger proof.
    ///
    /// Equal-rank writes are NOT refused, so same-status proof
    /// attachment and refresh keep working: attaching the first proof to
    /// a row already marked `InstantSendLocked` by
    /// `resolve_status_with_in_memory` (which sets that status with
    /// `None`, since it has no IS-lock data), and re-writing
    /// `ChainLocked` with a freshly-upgraded ChainLock proof at a newer
    /// height.
    ///
    /// # Ordering
    ///
    /// Like the promotion, the mutation and the enqueue happen as one
    /// unit under [`status_persist_serial`](AssetLockManager::status_persist_serial).
    /// This is the finalizing half of the pair: without the shared mutex
    /// a proof-bearing `InstantSendLocked` / `ChainLocked` snapshot can
    /// be enqueued BEFORE a concurrently-delayed promoter enqueues its
    /// older `Broadcast + None` one, regressing the durable row.
    ///
    /// # Lifecycle
    ///
    /// Errors with
    /// [`AssetLockManagerInactive`](PlatformWalletError::AssetLockManagerInactive)
    /// when the owning wallet has been removed. Every caller reaches
    /// this after an unbounded proof wait, which is precisely the span
    /// a `remove_wallet` + re-import can complete inside — so the
    /// under-mutex check is the one that matters here.
    pub(crate) async fn advance_asset_lock_status(
        &self,
        out_point: &OutPoint,
        new_status: AssetLockStatus,
        proof: Option<dpp::prelude::AssetLockProof>,
    ) -> Result<AssetLockChangeSet, PlatformWalletError> {
        // Test-only pause BEFORE the ordering mutex, so a test can hold
        // this writer while a competing finalize runs its whole
        // mutate→enqueue unit to completion. One-shot: taken rather than
        // cloned, so the competing finalize (same method) runs straight
        // through. See `advance_pre_lock_gate`.
        #[cfg(test)]
        {
            let gate = self
                .advance_pre_lock_gate
                .lock()
                .expect("advance pre-lock gate mutex")
                .take();
            if let Some(gate) = gate {
                gate.arrived.notify_one();
                gate.release.notified().await;
            }
        }

        // Hold the ordering mutex across mutate → enqueue. Acquired
        // BEFORE `wallet_manager` (see the field's lock-ordering note).
        let _serial = self.lock_status_persist_serial().await;
        self.ensure_active_under_serial(&_serial)?;

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

            // Monotonicity guard — see the `# Monotonicity` section.
            // Checked here, under the same `status_persist_serial` the
            // mutation and enqueue hold, so the status it reads is the
            // one this unit is about to overwrite: no concurrent writer
            // can finalize the row between the comparison and the
            // assignment.
            //
            // Bail out with an EMPTY changeset rather than an error. The
            // row is already at least as advanced as what this caller
            // would have written, which is a success from the wallet
            // state's point of view, and the caller's own proof stays
            // usable for its pending submission.
            if new_status.lifecycle_rank() < entry.status.lifecycle_rank() {
                tracing::info!(
                    outpoint = %out_point,
                    current_status = ?entry.status,
                    refused_status = ?new_status,
                    current_has_proof = entry.proof.is_some(),
                    refused_carries_proof = proof.is_some(),
                    "advance_asset_lock_status: refusing a status downgrade — \
                     keeping the row's stronger status and proof; the caller \
                     may still use the proof it obtained"
                );
                return Ok(AssetLockChangeSet::default());
            }

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
