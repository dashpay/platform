//! Asset lock lifecycle manager.
//!
//! Encapsulates all asset lock operations: building transactions, broadcasting,
//! waiting for proofs, and tracking lifecycle status. Shared across sub-wallets
//! via `Arc<AssetLockManager>`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{Notify, RwLock};

use crate::broadcaster::TransactionBroadcaster;
use crate::changeset::changeset::AssetLockChangeSet;
use crate::error::PlatformWalletError;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};

use super::tracked::TrackedAssetLock;
use key_wallet_manager::WalletManager;

/// Default fee rate in duffs per kilobyte for asset lock transactions.
pub(super) const DEFAULT_FEE_PER_KB: u64 = 1000;

/// Test-only rendezvous for
/// [`AssetLockManager::resume_pre_promote_gate`]. `arrived` fires once a
/// resume has taken its read-locked status snapshot; the resume then
/// blocks on `release`.
#[cfg(test)]
#[derive(Clone)]
pub(super) struct ResumePrePromoteGate {
    pub(super) arrived: Arc<Notify>,
    pub(super) release: Arc<Notify>,
}

/// Test-only rendezvous for
/// [`AssetLockManager::promote_post_cas_gate`]. `arrived` fires once a
/// `Built` → `Broadcast` promotion has mutated the in-memory row but has
/// NOT yet enqueued its changeset; the promoter then blocks on `release`.
///
/// This is precisely the window in which a stale promoter could enqueue
/// its older `Broadcast + None` snapshot AFTER a concurrent flow already
/// enqueued a newer proof-bearing one — the ordering hazard
/// [`AssetLockManager::status_persist_serial`] exists to close.
#[cfg(test)]
#[derive(Clone)]
pub(super) struct PromotePostCasGate {
    pub(super) arrived: Arc<Notify>,
    pub(super) release: Arc<Notify>,
}

/// Test-only rendezvous for
/// [`AssetLockManager::advance_pre_lock_gate`]. `arrived` fires once an
/// [`advance_asset_lock_status`](AssetLockManager::advance_asset_lock_status)
/// call has entered but BEFORE it acquires
/// [`status_persist_serial`](AssetLockManager::status_persist_serial);
/// the writer then blocks on `release`.
///
/// Parked *before* the mutex on purpose. The hazard the monotonicity
/// guard closes is a writer that is late relative to another writer's
/// whole mutate→enqueue unit — an IS-lock waiter released after a
/// ChainLock finalize has already completed. A gate placed after the
/// mutex could not produce that: the parked writer would hold the lock,
/// the ChainLock finalize would queue behind it, and the IS write would
/// land FIRST (a legal `Broadcast` → `InstantSendLocked` advance), which
/// is the opposite interleave.
#[cfg(test)]
#[derive(Clone)]
pub(super) struct AdvancePreLockGate {
    pub(super) arrived: Arc<Notify>,
    pub(super) release: Arc<Notify>,
}

/// Manages the full asset lock lifecycle: build, broadcast, proof, and tracking.
///
/// Shared across sub-wallets via `Arc<AssetLockManager>` so that any sub-wallet
/// (identity, platform-address, shielded) can create and consume asset locks
/// without going through `CoreWallet`.
///
/// `B` is the concrete broadcaster type; every `broadcast()` call dispatches
/// statically instead of through a `dyn` vtable.
pub struct AssetLockManager<B: TransactionBroadcaster + ?Sized> {
    pub(super) sdk: Arc<dash_sdk::Sdk>,
    /// The shared wallet manager lock for all mutable wallet state.
    pub(super) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Identifies which wallet within the manager this manager operates on.
    pub(super) wallet_id: WalletId,
    /// Notified on InstantLock / ChainLock events by SpvEventForwarder.
    /// Used by `wait_for_proof()` and `wait_for_chain_lock()`.
    pub(super) lock_notify: Arc<Notify>,
    /// Transaction broadcaster — pluggable so the same `AssetLockManager`
    /// works with different broadcast backends:
    ///
    /// - [`DapiBroadcaster`](crate::broadcaster::DapiBroadcaster) — gRPC via
    ///   Platform DAPI (default for standalone wallets without SPV).
    /// - [`SpvBroadcaster`](crate::broadcaster::SpvBroadcaster) — P2P via SPV
    ///   peers (used when managed by `PlatformWalletManager` with SPV enabled).
    ///
    /// Injected at construction by `PlatformWallet::new()`. The caller
    /// (typically `PlatformWalletManager`) decides which implementation to use.
    pub(super) broadcaster: Arc<B>,
    /// Per-wallet persistence handle. Cloned from the parent
    /// `PlatformWallet` at construction so asset lock mutations can
    /// queue their own `AssetLockChangeSet`s into the changeset flush
    /// boundary without round-tripping through the parent wallet.
    ///
    /// Item 8 sub-step 1a: previously mutations returned
    /// `AssetLockChangeSet` and callers (including
    /// `create_funded_asset_lock_proof` itself) dropped them with
    /// `let _cs = ...`. Every emitted changeset now flows straight
    /// into `queue_persist` here.
    pub(super) persister: WalletPersister,
    /// Serializes the funding-index-critical section of
    /// [`broadcast_funded_asset_lock`](Self::broadcast_funded_asset_lock) —
    /// build (index allocation + in-memory mark-used) through the address-pool
    /// persist/flush. Without it, two concurrent builds can interleave so that
    /// the FIRST build's pool snapshot (collected before the second build
    /// marked its index) is persisted LAST, rolling the durable snapshot back
    /// to a state where the second index reads unused — after a restart the
    /// next invitation re-selects that index and re-exports the same bearer
    /// voucher key. A dedicated mutex (never held while awaiting
    /// `wallet_manager`'s lock from outside this section, and never acquired
    /// by code that already holds it) avoids the self-deadlock a
    /// `wallet_manager.write()` guard would cause across the build→persist
    /// span. Deliberately NOT held across the broadcast/proof-wait — only the
    /// snapshot ordering needs serialization.
    pub(super) build_persist_serial: tokio::sync::Mutex<()>,
    /// Test-only gauge of builds currently at or past the
    /// `build_persist_serial` gate within `broadcast_funded_asset_lock`
    /// (incremented before the `lock().await`, RAII-decremented on every
    /// exit from the call). Lets the concurrency regression test
    /// synchronize on "the second build has reached the serialization
    /// gate" instead of assuming a scheduling delay — while the first
    /// build holds the lock, a gauge of 2 proves the second build cannot
    /// yet have collected its pool snapshot.
    #[cfg(test)]
    pub(super) build_serial_gate: std::sync::atomic::AtomicUsize,
    /// Test-only gauge of tasks currently BLOCKED on
    /// [`build_persist_serial`](Self::build_persist_serial): incremented
    /// before the `lock().await` and RAII-decremented the moment it is
    /// acquired (see
    /// [`lock_build_persist_serial`](Self::lock_build_persist_serial)).
    ///
    /// Distinct from [`build_serial_gate`](Self::build_serial_gate),
    /// which counts builds at *or past* the gate for the whole of
    /// `broadcast_funded_asset_lock`. This one answers the narrower
    /// "who is still queued and cannot get past the boundary while the
    /// current holder keeps the lock" — the arrival signal the retirement
    /// barrier test rendezvous on, in place of a sleep.
    #[cfg(test)]
    pub(super) build_serial_waiters: std::sync::atomic::AtomicUsize,
    /// Serializes every asset-lock **status mutation + persistence
    /// enqueue** pair, so the order in which rows are mutated in memory
    /// is the order in which their snapshots reach the persister.
    ///
    /// `wallet_manager`'s write lock alone is not enough. Each mutator
    /// (`promote_built_to_broadcast`, `advance_asset_lock_status`, …)
    /// takes it, mutates, and RELEASES it before returning the
    /// changeset the caller then hands to `queue_asset_lock_changeset`.
    /// In that post-mutation/pre-enqueue window another flow can acquire
    /// the wallet lock, finalize the same row to `InstantSendLocked` /
    /// `ChainLocked` with a proof, and enqueue that newer snapshot
    /// first — after which the delayed writer enqueues its older
    /// `Broadcast + None` snapshot LAST. Nothing downstream repairs the
    /// inversion: `FFIPersister::store_round` only serializes rounds by
    /// acquisition order (so it faithfully preserves the reversed
    /// order), `AssetLockChangeSet::merge` is last-write-wins, and
    /// Swift's `persistAssetLocks` upsert overwrites `statusRaw` /
    /// `proofBytes` unconditionally. Memory stays finalized while the
    /// durable row regresses to `Broadcast`, and because the load path
    /// treats `statusRaw < 2` as still-pending, a restart resumes a lock
    /// whose proof it already had.
    ///
    /// Held across mutation → enqueue, and deliberately NOT across the
    /// unbounded awaits (`broadcast`, `wait_for_proof`) that surround
    /// them: only the relative order of the two adjacent steps needs
    /// serializing. A dedicated mutex rather than extending the
    /// `wallet_manager` write guard because `queue_asset_lock_changeset`
    /// calls the persister synchronously — on iOS that reenters Swift
    /// via FFI — and holding the wallet lock across that would serialize
    /// every unrelated wallet reader behind host I/O.
    ///
    /// Lock ordering: acquire this BEFORE `wallet_manager`, never the
    /// reverse. Every holder follows the same
    /// `status_persist_serial → wallet_manager.write() → drop(wallet) →
    /// enqueue → drop(serial)` shape, so no cycle exists.
    pub(super) status_persist_serial: tokio::sync::Mutex<()>,
    /// Test-only gauge of tasks currently BLOCKED on
    /// [`status_persist_serial`](Self::status_persist_serial):
    /// incremented before the `lock().await` and decremented the moment
    /// it is acquired (see
    /// [`lock_status_persist_serial`](Self::lock_status_persist_serial)),
    /// so a non-zero value means "some task reached the ordering
    /// boundary and cannot get past it while the current holder keeps
    /// the lock".
    ///
    /// This is the arrival signal the ordering tests rendezvous on. A
    /// sleep only shows that time passed — it cannot distinguish "the
    /// competing task is queued at the mutex" from "the competing task
    /// has not been scheduled yet", so a test that released its parked
    /// holder after a delay could grade an unserialized implementation
    /// as passing whenever the scheduler happened to run things in the
    /// non-regressing order. Because the counter is maintained by the
    /// lock helper itself, an implementation that stops taking the
    /// mutex never increments it, and the waiting test fails instead of
    /// silently passing.
    ///
    /// Covers the async acquirers only. The one `blocking_lock` caller
    /// (`recover_tracked_asset_lock`) runs outside the async runtime,
    /// so no test can observe it mid-wait.
    #[cfg(test)]
    pub(super) status_serial_waiters: std::sync::atomic::AtomicUsize,
    /// Test-only pause point inside
    /// [`resume_asset_lock`](Self::resume_asset_lock), between the
    /// read-locked status snapshot and the write-locked `Built` →
    /// `Broadcast` compare-and-set. When set, a resume signals `arrived`
    /// and then awaits `release` at that point, which lets a test hold a
    /// resume on a stale `Built` snapshot while another flow finalizes
    /// the same row to `InstantSendLocked` / `ChainLocked` — the exact
    /// interleave the compare-and-set exists to survive.
    ///
    /// Both halves matter for determinism: `arrived` proves the resume
    /// really did snapshot `Built` BEFORE the test finalized the row (so
    /// the snapshot under test is genuinely stale), and `release` holds
    /// it there until the finalize has landed. `None` (the default)
    /// makes the hook a no-op.
    #[cfg(test)]
    pub(super) resume_pre_promote_gate: std::sync::Mutex<Option<ResumePrePromoteGate>>,
    /// Whether this manager may still mutate and persist asset-lock
    /// state. Set once, `true` → `false`, by
    /// [`deactivate`](Self::deactivate) when the owning wallet is
    /// removed from its `PlatformWalletManager`.
    ///
    /// # Why a manager can outlive its wallet
    ///
    /// `AssetLockManager` resolves its target by `wallet_id` alone,
    /// through the *shared* `WalletManager`. Wallet ids are
    /// deterministic in (seed, network), so removing a wallet and
    /// re-importing the same mnemonic re-creates the identical id over
    /// a brand-new `PlatformWalletInfo` — and `PlatformWallet::new`
    /// builds a brand-new `AssetLockManager` with its own
    /// [`status_persist_serial`](Self::status_persist_serial). Any
    /// `Arc<AssetLockManager>` retained across that removal (an FFI
    /// `asset_lock_manager` handle the host never destroyed, or a
    /// resume task still parked in `wait_for_proof`) therefore becomes
    /// live again against the *replacement* wallet's rows. Two managers
    /// would then mutate and enqueue the same row under two different
    /// mutexes, which is exactly the unserialized mutate→enqueue
    /// interleave `status_persist_serial` exists to prevent — only now
    /// across instances, where a single mutex cannot see it.
    ///
    /// # Why the check must live under `status_persist_serial`
    ///
    /// A bare `if !active { return }` before an `await` is racy: the
    /// removal can land in the window between the check and the
    /// mutation. So the authoritative test is taken AFTER acquiring
    /// `status_persist_serial` and before the wallet lookup (see
    /// [`ensure_active_under_serial`](Self::ensure_active_under_serial)),
    /// and `deactivate` flips the flag while HOLDING that same mutex.
    /// That gives both halves of the guarantee:
    ///
    /// - removal waits for any in-flight mutate→enqueue unit to finish
    ///   (it cannot take the mutex until the holder releases it), so no
    ///   changeset is stranded mid-unit;
    /// - every operation that starts, or resumes from an await, after
    ///   the flip observes `false` before touching the wallet row or
    ///   the persister.
    ///
    /// `SeqCst` rather than `Relaxed` because the flag is also read
    /// outside the mutex by the cheap early-out checks on the public
    /// entry points; those are advisory (they only avoid wasted work),
    /// but keeping one ordering for a single-writer flag costs nothing
    /// here and avoids reasoning about two.
    pub(super) active: AtomicBool,
    /// Test-only pause point inside
    /// [`promote_and_queue_built_to_broadcast`](Self::promote_and_queue_built_to_broadcast),
    /// between the compare-and-set mutation and the persistence enqueue.
    /// Lets a test hold a promoter in that window while another flow
    /// finalizes and enqueues the same row, which is the interleave
    /// [`Self::status_persist_serial`] exists to prevent. `None` (the
    /// default) makes the hook a no-op.
    #[cfg(test)]
    pub(super) promote_post_cas_gate: std::sync::Mutex<Option<PromotePostCasGate>>,
    /// Test-only pause point at the very top of
    /// [`advance_asset_lock_status`](Self::advance_asset_lock_status),
    /// before [`status_persist_serial`](Self::status_persist_serial) is
    /// taken. Lets a test hold an `InstantSendLocked` writer there while
    /// another flow finalizes the same row to `ChainLocked` with a
    /// ChainLock proof — the delayed-downgrade interleave the
    /// monotonicity guard exists to refuse. `None` (the default) makes
    /// the hook a no-op.
    ///
    /// Consumed on arrival (taken, not cloned), so only the FIRST
    /// advance parks: the finalize the test performs while the IS writer
    /// waits goes through the same method and must not deadlock against
    /// the gate it is racing.
    #[cfg(test)]
    pub(super) advance_pre_lock_gate: std::sync::Mutex<Option<AdvancePreLockGate>>,
}

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Create a new `AssetLockManager`.
    pub(crate) fn new(
        sdk: Arc<dash_sdk::Sdk>,
        wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
        wallet_id: WalletId,
        lock_notify: Arc<Notify>,
        broadcaster: Arc<B>,
        persister: WalletPersister,
    ) -> Self {
        Self {
            sdk,
            wallet_manager,
            wallet_id,
            lock_notify,
            broadcaster,
            persister,
            build_persist_serial: tokio::sync::Mutex::new(()),
            status_persist_serial: tokio::sync::Mutex::new(()),
            active: AtomicBool::new(true),
            #[cfg(test)]
            status_serial_waiters: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            build_serial_gate: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            build_serial_waiters: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            resume_pre_promote_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            promote_post_cas_gate: std::sync::Mutex::new(None),
            #[cfg(test)]
            advance_pre_lock_gate: std::sync::Mutex::new(None),
        }
    }

    /// Acquire [`status_persist_serial`](Self::status_persist_serial).
    ///
    /// Every async mutate→enqueue pair goes through here rather than
    /// locking the field directly, so the test-only
    /// [`status_serial_waiters`](Self::status_serial_waiters) gauge sees
    /// every arrival at the ordering boundary. In non-test builds this
    /// compiles to the bare `lock().await`.
    pub(super) async fn lock_status_persist_serial(&self) -> tokio::sync::MutexGuard<'_, ()> {
        // RAII rather than a bare decrement after the await: if the
        // caller's future is dropped while still queued, the count must
        // come back down, or a cancelled task would leave the gauge
        // permanently non-zero and every later wait would return
        // instantly on a phantom arrival.
        #[cfg(test)]
        struct WaiterGauge<'a>(&'a std::sync::atomic::AtomicUsize);
        #[cfg(test)]
        impl Drop for WaiterGauge<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        #[cfg(test)]
        let waiting = {
            self.status_serial_waiters
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            WaiterGauge(&self.status_serial_waiters)
        };

        let guard = self.status_persist_serial.lock().await;

        // Dropped on acquisition, not on release: the gauge answers
        // "who is still queued at the boundary", so the holder must not
        // count itself.
        #[cfg(test)]
        drop(waiting);

        guard
    }

    /// Acquire [`build_persist_serial`](Self::build_persist_serial).
    ///
    /// Every build→persist unit goes through here rather than locking
    /// the field directly, so the test-only
    /// [`build_serial_waiters`](Self::build_serial_waiters) gauge sees
    /// every arrival at the boundary. In non-test builds this compiles
    /// to the bare `lock().await`.
    pub(super) async fn lock_build_persist_serial(&self) -> tokio::sync::MutexGuard<'_, ()> {
        // RAII for the same reason as `lock_status_persist_serial`: a
        // future dropped while queued must not leave the gauge
        // permanently non-zero.
        #[cfg(test)]
        struct WaiterGauge<'a>(&'a std::sync::atomic::AtomicUsize);
        #[cfg(test)]
        impl Drop for WaiterGauge<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        #[cfg(test)]
        let waiting = {
            self.build_serial_waiters
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            WaiterGauge(&self.build_serial_waiters)
        };

        let guard = self.build_persist_serial.lock().await;

        // Dropped on acquisition, not on release: the gauge answers
        // "who is still queued at the boundary", so the holder must not
        // count itself.
        #[cfg(test)]
        drop(waiting);

        guard
    }

    /// Retire this manager: no later operation may mutate or persist
    /// asset-lock state through it.
    ///
    /// Called by
    /// [`PlatformWalletManager::remove_wallet`](crate::manager::PlatformWalletManager::remove_wallet)
    /// BEFORE the wallet's entry is dropped from the shared
    /// `WalletManager`, so a retained handle can never observe (let
    /// alone write) the `PlatformWalletInfo` that a later re-import of
    /// the same mnemonic installs under the same deterministic
    /// `wallet_id`. See [`active`](Self::active) for why a manager can
    /// outlive its wallet at all.
    ///
    /// The flag is flipped while HOLDING **both** ordering mutexes —
    /// [`build_persist_serial`](Self::build_persist_serial) and
    /// [`status_persist_serial`](Self::status_persist_serial) — which is
    /// what makes the retirement a barrier rather than a hint:
    ///
    /// - it cannot take either mutex until any in-flight unit has
    ///   released it, so removal never interrupts one halfway (a build
    ///   that allocated a funding index but has not persisted the pool;
    ///   a row mutated in memory whose changeset has not reached the
    ///   persister);
    /// - once it returns, every subsequent acquirer — including
    ///   operations that had already started and were parked on
    ///   `broadcast` / `wait_for_proof`, or queued at either boundary —
    ///   sees `false` at
    ///   [`ensure_active_under_serial`](Self::ensure_active_under_serial)
    ///   or
    ///   [`ensure_active_under_build_serial`](Self::ensure_active_under_build_serial)
    ///   before it touches the wallet, a row, or the persister.
    ///
    /// Both are needed because the two units are guarded separately and
    /// neither subsumes the other: builds mutate `Wallet` /
    /// `ManagedWalletInfo` (deriving a top-up account, consuming a
    /// funding address, reserving inputs) without ever taking
    /// `status_persist_serial`, so retiring under the status mutex alone
    /// would leave a queued build free to run against the replacement
    /// wallet a same-mnemonic re-import installs under the same
    /// deterministic id.
    ///
    /// Acquisition order is `build_persist_serial` then
    /// `status_persist_serial`, and it is the only place both are held:
    /// `broadcast_funded_asset_lock` drops the build guard before
    /// `track_asset_lock` takes the status one, so nothing ever holds
    /// them in the opposite order and no cycle exists.
    ///
    /// Idempotent: a second call is a no-op (it still takes both
    /// mutexes, so it still waits out any in-flight unit).
    ///
    /// Must NOT be called while holding the `wallet_manager` lock — the
    /// units this waits on acquire `wallet_manager` themselves, so doing
    /// so would invert the documented `serial → wallet_manager` order
    /// and deadlock.
    pub(crate) async fn deactivate(&self) {
        let _build = self.lock_build_persist_serial().await;
        let _serial = self.lock_status_persist_serial().await;
        let was_active = self.active.swap(false, Ordering::SeqCst);
        if was_active {
            tracing::debug!(
                wallet_id = %hex::encode(self.wallet_id),
                "AssetLockManager deactivated: its wallet was removed from the manager"
            );
        }
    }

    /// The authoritative stale-handle check, taken by every asset-lock
    /// status mutate→enqueue primitive AFTER it has acquired
    /// [`status_persist_serial`](Self::status_persist_serial) and
    /// BEFORE it looks the wallet up.
    ///
    /// Ordering is the whole point: the caller already holds the mutex
    /// that [`deactivate`](Self::deactivate) must take to flip the
    /// flag, so a `true` observed here cannot go stale for the
    /// remainder of the critical section — the mutation and its
    /// enqueue both complete against the wallet this manager was built
    /// for. Checking before the mutex (as the public entry points also
    /// do, cheaply) is only an early-out; it can be invalidated by a
    /// removal landing during the very next `await`.
    pub(super) fn ensure_active_under_serial(
        &self,
        _serial: &tokio::sync::MutexGuard<'_, ()>,
    ) -> Result<(), PlatformWalletError> {
        self.ensure_active()
    }

    /// The authoritative stale-handle check for the *build* unit, taken
    /// after acquiring
    /// [`build_persist_serial`](Self::build_persist_serial) and BEFORE
    /// the `wallet_manager` write lock.
    ///
    /// Same argument as
    /// [`ensure_active_under_serial`](Self::ensure_active_under_serial),
    /// against the other mutex: a build mutates the wallet itself
    /// (deriving a top-up account, consuming a funding address,
    /// reserving inputs) without ever touching a status row, so the
    /// status mutex says nothing about it. Because
    /// [`deactivate`](Self::deactivate) must take this mutex too, a
    /// `true` observed here holds for the rest of the build: either the
    /// retirement landed before us and we refuse, or it is queued behind
    /// us and the wallet we are about to mutate is still ours.
    pub(super) fn ensure_active_under_build_serial(
        &self,
        _build_serial: &tokio::sync::MutexGuard<'_, ()>,
    ) -> Result<(), PlatformWalletError> {
        self.ensure_active()
    }

    /// Cheap advisory activity check for public entry points
    /// (`resume_asset_lock`, `broadcast_funded_asset_lock`, …), so a
    /// stale handle fails immediately instead of doing a build or an
    /// unbounded proof wait whose mutation will be refused anyway.
    ///
    /// NOT sufficient on its own — a removal can land between this
    /// check and any later await. The under-mutex
    /// [`ensure_active_under_serial`](Self::ensure_active_under_serial)
    /// is what actually closes the race.
    pub(super) fn ensure_active(&self) -> Result<(), PlatformWalletError> {
        if self.active.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(PlatformWalletError::AssetLockManagerInactive(hex::encode(
                self.wallet_id,
            )))
        }
    }

    /// Test-only: wait until at least `n` tasks are blocked on
    /// [`status_persist_serial`](Self::status_persist_serial).
    ///
    /// The rendezvous the ordering tests use in place of a sleep. The
    /// caller must already hold the mutex (or otherwise know it is
    /// held), so an arrival observed here is an arrival that provably
    /// cannot proceed past the boundary. Polls rather than using a
    /// `Notify` because the waiters are inside the production lock
    /// helper, which must stay free of test-only signalling on the
    /// contended path.
    ///
    /// Panics after ~10s so a genuine hang fails loudly instead of
    /// running until the harness times out.
    #[cfg(test)]
    pub(super) async fn await_status_serial_waiters(&self, n: usize) {
        for _ in 0..2_000 {
            if self
                .status_serial_waiters
                .load(std::sync::atomic::Ordering::SeqCst)
                >= n
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!(
            "timed out waiting for {n} task(s) to block on status_persist_serial \
             (saw {}) — either the competing task never reached the ordering \
             boundary, or the code under test no longer acquires the mutex there",
            self.status_serial_waiters
                .load(std::sync::atomic::Ordering::SeqCst)
        );
    }

    /// Test-only: wait until at least `n` tasks are blocked on
    /// [`build_persist_serial`](Self::build_persist_serial).
    ///
    /// [`await_status_serial_waiters`](Self::await_status_serial_waiters)
    /// for the other ordering mutex; same contract, same reason for
    /// polling.
    #[cfg(test)]
    pub(super) async fn await_build_serial_waiters(&self, n: usize) {
        for _ in 0..2_000 {
            if self
                .build_serial_waiters
                .load(std::sync::atomic::Ordering::SeqCst)
                >= n
            {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!(
            "timed out waiting for {n} task(s) to block on build_persist_serial \
             (saw {}) — either the competing task never reached the ordering \
             boundary, or the code under test no longer acquires the mutex there",
            self.build_serial_waiters
                .load(std::sync::atomic::Ordering::SeqCst)
        );
    }

    /// Queue an `AssetLockChangeSet` onto the per-wallet persister.
    /// No-op when the changeset is empty.
    ///
    /// `pub(crate)` so the orchestrated funding flows in
    /// `wallet::platform_addresses` and `wallet::identity::network`
    /// can pair an `advance_asset_lock_status` call with a flush
    /// without going through the asset-lock module boundary. The
    /// internal-only flag (no `pub`) keeps the API hidden from
    /// crate consumers.
    pub(crate) fn queue_asset_lock_changeset(&self, cs: AssetLockChangeSet) {
        if <AssetLockChangeSet as crate::changeset::Merge>::is_empty(&cs) {
            return;
        }
        if let Err(e) = self
            .persister
            .store(crate::changeset::PlatformWalletChangeSet {
                asset_locks: Some(cs),
                ..Default::default()
            })
        {
            tracing::error!(
                error = %e,
                "AssetLockManager: failed to queue changeset for persistence"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Public read accessors
// ---------------------------------------------------------------------------

impl<B: TransactionBroadcaster + ?Sized> AssetLockManager<B> {
    /// Wallet id this manager operates on. Exposed so FFI callers that
    /// build a `MnemonicResolverCoreSigner` (or similar) on demand can
    /// thread the wallet id through to the resolver callback without
    /// reaching into private fields.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Network the SDK was constructed with. Same rationale as
    /// [`Self::wallet_id`] — needed by FFI callers that build a
    /// `key_wallet::signer::Signer` per call.
    pub fn network(&self) -> dashcore::Network {
        self.sdk.network
    }

    /// List all tracked asset locks (blocking version for UI / synchronous contexts).
    ///
    /// Uses `tokio::sync::RwLock::blocking_read` — must NOT be called from
    /// within a tokio async context.
    pub fn list_tracked_locks_blocking(&self) -> Vec<TrackedAssetLock> {
        let wm = self.wallet_manager.blocking_read();
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.tracked_asset_locks.values().cloned().collect())
            .unwrap_or_default()
    }

    /// List all tracked asset locks (async version).
    pub async fn list_tracked_locks(&self) -> Vec<TrackedAssetLock> {
        let wm = self.wallet_manager.read().await;
        wm.get_wallet_info(&self.wallet_id)
            .map(|info| info.tracked_asset_locks.values().cloned().collect())
            .unwrap_or_default()
    }
}

impl<B: TransactionBroadcaster + ?Sized> std::fmt::Debug for AssetLockManager<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetLockManager")
            .field("network", &self.sdk.network)
            .finish()
    }
}
