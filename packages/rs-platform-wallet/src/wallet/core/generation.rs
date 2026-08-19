//! Per-wallet-*generation* shared state: the identity marker every handle to
//! one generation shares, that generation's lifecycle gate, and the
//! in-broadcast outpoint pins that fence a mid-dispatch transaction's inputs
//! against concurrent re-selection.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use dashcore::{OutPoint, Transaction};
use tokio::sync::{OwnedRwLockWriteGuard, RwLock, RwLockReadGuard};

use super::balance::WalletBalance;

/// The state one wallet *generation* shares across every handle that names it.
///
/// A "generation" is one live in-memory instance of a logical wallet. Removing a
/// wallet and re-creating it under the same `wallet_id` produces a *different*
/// generation: same id, same shared multi-wallet `WalletManager` `Arc`, fresh
/// `WalletGeneration`. `PlatformWalletManager` builds exactly one of these per
/// registration and clones the `Arc` into `PlatformWalletInfo`, `PlatformWallet`
/// and `CoreWallet`, so `Arc::ptr_eq` on it is the canonical generation identity
/// (see [`CoreWallet::is_same_generation`](super::CoreWallet::is_same_generation)).
///
/// # Why the balance and the lifecycle gate live in the *same* object
///
/// They are one indivisible fact — "which generation is this?" — and splitting
/// them into two `Arc`s threaded separately through ~15 construction sites would
/// let a future site clone the identity marker but mint a *fresh* gate. Two
/// handles would then compare as the same generation while excluding each other
/// through different locks: teardown would take one gate, an in-flight payment
/// would hold the other, and the exclusion would silently vanish with nothing to
/// fail. Keeping them in one `Arc` makes that divergence unrepresentable —
/// same generation is the same gate, by construction (`dashpay/platform#4185`).
///
/// [`Deref`] to [`WalletBalance`] keeps every existing lock-free balance read
/// (`generation.confirmed()`, `info.balance.locked()`, …) working unchanged.
#[derive(Debug)]
pub struct WalletGeneration {
    /// Lock-free balance for UI reads. Updated from `ManagedWalletInfo` after
    /// each SPV block/mempool processing and RPC refresh.
    balance: WalletBalance,
    /// This generation's lifecycle gate — held across whole *operations* rather
    /// than around individual state mutations.
    ///
    /// Shared side ([`payment_guard`](Self::payment_guard)): any operation that
    /// will publish an ownership handle for this generation, or push one of its
    /// transactions to the network, after observing that the generation is still
    /// live. Exclusive side ([`teardown_guard`](Self::teardown_guard)): removing
    /// the generation from the manager and sweeping its deferred state.
    ///
    /// This is deliberately **per generation** rather than one process-global
    /// lock. A deferred broadcast holds the shared side across an SPV send
    /// (seconds, up to the broadcaster's timeout); with a single global lock that
    /// send would block teardown — and, because tokio's `RwLock` is
    /// write-preferring, every subsequent payment operation — for *every
    /// unrelated wallet* in the process. Scoped here, one wallet's slow send
    /// only ever excludes that same wallet's teardown, which is exactly the pair
    /// that must not interleave.
    ///
    /// Held in its own `Arc` so [`teardown_guard`](Self::teardown_guard) can hand
    /// back an *owned* guard: the remover resolves which generation is current in
    /// a retry loop, and the guard must outlive the loop iteration that produced
    /// the `Arc<WalletGeneration>` it came from.
    lifecycle: Arc<RwLock<()>>,
    /// Outpoints currently fenced against re-selection because a broadcast
    /// dispatch owns them ([`pin_in_broadcast`](Self::pin_in_broadcast)).
    ///
    /// The guarded dispatch (`CoreWallet::dispatch_unexpired`) proves under the
    /// wallet-manager read guard that a finalized transaction's funding
    /// reservation is still its own, then must release that guard before the
    /// broadcaster await (holding it starves the SPV mempool pipeline). The
    /// broadcaster can suspend *before* submission, and in that gap sync
    /// catch-up can advance `last_processed_height` far enough that key-wallet's
    /// `ReservationSet` TTL sweeps the reservation and a concurrent build
    /// re-reserves the very same inputs — the dispatch would then put an
    /// already-signed transaction on the wire against inputs reassigned to
    /// another payment. This map is the fence that outlives the dropped guard:
    /// every coin-selection choke point (`CoreWallet::finalize_transaction`, the
    /// contact-payment build, the asset-lock build) checks its freshly reserved
    /// selection against it — still under the manager write lock, the same
    /// synchronization height advancement and the TTL sweep run under — and
    /// refuses a build whose selection picked a fenced input.
    ///
    /// # Two phases, because dispatch return is not "the spend is safe"
    ///
    /// [`InBroadcastFence`] holds both phases per outpoint:
    ///
    /// * **dispatching** — a counted, non-expiring pin, live from check-and-pin
    ///   until the broadcaster returns *and* the post-return height sample that
    ///   anchors the next phase has been taken.
    /// * **pending-spend** — a height-bounded fence installed *when the
    ///   broadcaster returns anything other than a definitive pre-send
    ///   rejection*, i.e. when the transaction may be on the network.
    ///
    /// The second phase exists because dispatch returning does not mean the
    /// wallet has observed the spend. `SpvBroadcaster` injects the transaction
    /// into dash-spv's local mempool pipeline, so its inputs leave this wallet's
    /// selectable set within milliseconds — but `DapiBroadcaster::broadcast` only
    /// awaits `sdk.execute` and performs no local injection at all, so both an
    /// accepted response and an ambiguous `MaybeSent` return with the input still
    /// selectable here while the transaction is in flight. Dropping the fence at
    /// dispatch return would therefore reopen, on the DAPI path, exactly the
    /// sweep + re-select race the pin was added to close
    /// (`dashpay/platform#4309`).
    ///
    /// # The pending-spend phase is anchored AFTER the await, never before
    ///
    /// A full [`IN_BROADCAST_FENCE_BLOCKS`](crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS)
    /// interval is measured from a `last_processed_height` sampled once the
    /// broadcaster has returned — not from the height the freshness check
    /// consumed on the way in. Anchoring on the pre-await sample looks
    /// clock-consistent and is in fact the same defect one layer along: a
    /// broadcast await can suspend for minutes mid-catch-up on mobile, and if
    /// the wallet advances a full fence interval in that gap the fence is
    /// ALREADY LAPSED at the instant it is installed. The next selection reaps
    /// it and may reselect an input of a transaction that reached the network
    /// — an already-expired fence is indistinguishable from no fence
    /// (`dashpay/platform#4309`).
    ///
    /// Clock consistency is preserved by making the post-await sample the
    /// SINGLE anchor: it is read under the manager guard, from the same
    /// `last_processed_height` the freshness check and key-wallet's TTL sweep
    /// use, and the dispatching phase stays held across the sampling so no
    /// selection can slip between the broadcaster's return and the fence being
    /// stamped.
    ///
    /// When a dispatch stops without ever taking that sample — cancelled or
    /// unwound inside `broadcast`, the case a caller's `timeout`/`select!`
    /// produces — [`InBroadcastFence::pending_unanchored`] fences
    /// unconditionally and the first selection to consult the fence anchors it
    /// on ITS height. That is the drop-time clock, deferred to the earliest
    /// moment it is both readable (a synchronous `Drop` cannot await the
    /// manager lock) and relevant (nothing can reach a fenced outpoint in
    /// between).
    ///
    /// A *count* for the dispatching phase rather than a set:
    /// `broadcast_finalized_transaction` takes `&SignedCoreTransaction`, so a
    /// direct Rust caller can dispatch the same transaction twice concurrently
    /// (idempotent on the wire — same txid). Counting keeps the pin held until
    /// the LAST dispatch returns instead of letting the first completion unpin
    /// the other's in-flight send.
    ///
    /// A `std::sync::Mutex` like key-wallet's own `ReservationSet`: critical
    /// sections are a few hash operations, never held across an await, and the
    /// sync lock is what lets [`InBroadcastPin::drop`] settle the fence from a
    /// plain (non-async) `Drop` — which is also what makes the pin
    /// cancellation-safe when the dispatching future is dropped mid-await.
    /// Never persisted: after a restart nothing is mid-dispatch, and a
    /// transaction that actually landed is reconciled by sync.
    in_broadcast: Mutex<HashMap<OutPoint, InBroadcastFence>>,
}

/// One outpoint's broadcast fence — see `WalletGeneration::in_broadcast`.
#[derive(Debug, Default)]
struct InBroadcastFence {
    /// Dispatches currently *inside* the broadcaster await for this outpoint.
    /// Non-expiring while non-zero: a suspended dispatch keeps its inputs
    /// fenced no matter how far catch-up advances the clock.
    dispatching: u32,
    /// A dispatch settled this outpoint's pending-spend phase without a
    /// post-await height sample — the dispatching future was cancelled or
    /// unwound inside `broadcast`, or the wallet left the manager before the
    /// sample could be taken. Blocks UNCONDITIONALLY until
    /// [`WalletGeneration::in_broadcast_conflict`] anchors it, because a
    /// synchronous `Drop` has no lock-free way to read `last_processed_height`
    /// and the pre-await sample is exactly the stale anchor that made the fence
    /// arrive already lapsed (`dashpay/platform#4309`).
    pending_unanchored: bool,
    /// `last_processed_height` at which the ANCHORED pending-spend phase lapses.
    /// `None` means no dispatch has handed this outpoint to the network with a
    /// height to measure from.
    pending_until: Option<u32>,
}

impl InBroadcastFence {
    /// Whether this fence still blocks re-selection at `current_height`.
    fn blocks(&self, current_height: u32) -> bool {
        self.dispatching > 0
            || self.pending_unanchored
            || self
                .pending_until
                .is_some_and(|until| current_height < until)
    }

    /// Give the pending-spend phase a bound measured from `current_height`, the
    /// caller's `last_processed_height` read under the manager WRITE guard.
    ///
    /// Called by [`WalletGeneration::in_broadcast_conflict`] before it decides
    /// anything, so an unanchored fence is bounded at the first moment it could
    /// possibly matter — a fenced outpoint is unreachable by any other path, so
    /// there is no window between the settle and this stamp. Never SHORTENS an
    /// existing bound: a concurrent dispatch of the same transaction may
    /// already have installed a longer one.
    fn anchor(&mut self, current_height: u32) {
        if !self.pending_unanchored {
            return;
        }
        self.pending_unanchored = false;
        self.extend_pending_until(
            current_height.saturating_add(crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS),
        );
    }

    /// Push the anchored bound out to `until`, never in. Two concurrent
    /// dispatches of the same transaction must BOTH be covered, so the later
    /// bound wins.
    fn extend_pending_until(&mut self, until: u32) {
        self.pending_until = Some(self.pending_until.map_or(until, |cur| cur.max(until)));
    }

    /// Whether nothing holds this outpoint any more, so the entry can be
    /// dropped from the map.
    fn is_clear(&self) -> bool {
        self.dispatching == 0 && !self.pending_unanchored && self.pending_until.is_none()
    }
}

/// How one dispatch's pending-spend phase settles when its [`InBroadcastPin`]
/// is dropped — see [`WalletGeneration::pin_in_broadcast`].
///
/// The variants are ordered by how much the dispatch managed to prove, and the
/// INITIAL value is the least-informed one: a pin that learns nothing before it
/// drops must fence (`dashpay/platform#4309`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingSpendSettle {
    /// Nothing was learned: the dispatching future was cancelled or unwound
    /// mid-`broadcast`, or its post-await height sample never completed. The
    /// transaction may be on the wire and there is no trustworthy height to
    /// measure from, so the fence is installed unanchored and
    /// [`InBroadcastFence::anchor`] stamps it from the next selection's clock.
    Unanchored,
    /// The broadcaster returned something other than a definitive pre-send
    /// rejection, and `last_processed_height` was sampled AFTER that return:
    /// the fence lapses [`IN_BROADCAST_FENCE_BLOCKS`](crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS)
    /// past this height.
    AnchoredAt(u32),
    /// A definitive pre-send rejection — the one outcome that proves the
    /// transaction never reached the network. No pending-spend phase at all.
    Released,
}

impl Default for WalletGeneration {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletGeneration {
    /// A fresh generation: zeroed balance, uncontended gate, nothing pinned.
    pub fn new() -> Self {
        Self {
            balance: WalletBalance::new(),
            lifecycle: Arc::new(RwLock::new(())),
            in_broadcast: Mutex::new(HashMap::new()),
        }
    }

    /// This generation's lock-free balance.
    pub fn balance(&self) -> &WalletBalance {
        &self.balance
    }

    /// Enter this generation's lifecycle gate as a *payment* operation.
    ///
    /// Shared: any number of payment operations on this generation (and every
    /// operation on every *other* generation) run concurrently. What it excludes
    /// is this generation's own teardown ([`teardown_guard`](Self::teardown_guard)),
    /// which is what makes a liveness observation
    /// ([`CoreWallet::is_current_generation`](super::CoreWallet::is_current_generation))
    /// safe to act on: held across both the check and the action it gates, a
    /// removal cannot interleave between them.
    ///
    /// Callers must hold it across the check *and* the publication/network step,
    /// and must not already hold it (the `RwLock` is not reentrant, and because
    /// tokio's is write-preferring a queued teardown would deadlock the
    /// re-entry).
    ///
    /// # Lock ordering
    ///
    /// Always taken BEFORE the wallet-manager `RwLock`, never while holding it.
    /// Teardown takes it and then awaits the manager write lock; payment
    /// operations take it and then await the manager read lock. The order is
    /// total, so the two locks cannot deadlock.
    pub async fn payment_guard(&self) -> RwLockReadGuard<'_, ()> {
        self.lifecycle.read().await
    }

    /// Enter this generation's lifecycle gate as a *teardown*.
    ///
    /// Exclusive against every payment operation on this generation. Removal
    /// holds it across BOTH the manager removal and the deferred-state sweep, so
    /// the two are one linearization point rather than two steps with a window
    /// between them that a retained handle could broadcast through
    /// (`dashpay/platform#4185`).
    ///
    /// Acquiring it waits for payment operations that have already entered their
    /// liveness-check/publish section. It does **not** wait for an operation
    /// still awaiting an external signer: those acquire the gate only *after*
    /// the signature returns, precisely so an open signing prompt cannot stall
    /// teardown. Such a late finalizer then observes the removed generation at
    /// its liveness check and abandons instead of publishing.
    ///
    /// Returns an *owned* guard so it can outlive the `Arc<WalletGeneration>`
    /// binding it was taken from — the remover resolves the current generation in
    /// a retry loop and must carry the guard out of the iteration that found it.
    pub async fn teardown_guard(&self) -> OwnedRwLockWriteGuard<()> {
        Arc::clone(&self.lifecycle).write_owned().await
    }

    /// Recovers from a poisoned mutex rather than panicking: the guarded data
    /// is a plain count map with no invariant a partial write could break, and
    /// panicking here would strand every later build and dispatch on this
    /// generation. (Same policy as key-wallet's `ReservationSet`.)
    fn in_broadcast_lock(&self) -> MutexGuard<'_, HashMap<OutPoint, InBroadcastFence>> {
        self.in_broadcast
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Pin `transaction`'s inputs as **in-broadcast** until the returned
    /// [`InBroadcastPin`] is dropped.
    ///
    /// Taken by the guarded dispatch (`CoreWallet::dispatch_unexpired`) while
    /// it still holds the wallet-manager READ guard that proved the funding
    /// reservation fresh — the freshness bound sits strictly below key-wallet's
    /// reservation TTL on the same `last_processed_height` clock, and both the
    /// TTL sweep and height advancement mutate under the manager WRITE lock, so
    /// under that guard the reservation is provably still this build's: that
    /// proof is the pin's owner check, and installing the pin before the guard
    /// drops makes check-and-pin one atomic step. The pin then *outlives* the
    /// guard, deliberately: it is what keeps the check meaningful across the
    /// broadcaster await the guard must not span (see the
    /// [`in_broadcast`](Self::in_broadcast) field docs for the full race).
    ///
    /// The dispatching phase has **no TTL** — a suspended dispatch keeps its
    /// inputs fenced no matter how far catch-up advances the clock — and ends
    /// only when the returned guard is dropped, which happens even when the
    /// dispatching future is cancelled mid-await (`Drop` runs on unwind and on
    /// future drop alike).
    ///
    /// # No height is taken here, deliberately
    ///
    /// This call takes NO `last_processed_height`, even though one is in hand
    /// under the guard. The pending-spend phase must be measured from a height
    /// sampled once the broadcaster has RETURNED
    /// ([`InBroadcastPin::anchor_pending_spend`]): a broadcast await can
    /// suspend for minutes mid-catch-up, and a fence anchored on the pre-await
    /// sample arrives already lapsed whenever the wallet advanced a full fence
    /// interval in the gap — which is no fence at all
    /// (`dashpay/platform#4309`). Not accepting the height makes that
    /// mis-anchoring unrepresentable rather than merely fixed. A pin that is
    /// never anchored still fences, unconditionally, until the first selection
    /// stamps it (see [`InBroadcastPin`]).
    ///
    /// Callers pin on the generation currently REGISTERED in the manager
    /// (`PlatformWalletInfo::generation`), the same object the build-side
    /// conflict checks read, so the fence works even for a dispatch through a
    /// stale-generation handle.
    pub(crate) fn pin_in_broadcast(self: &Arc<Self>, transaction: &Transaction) -> InBroadcastPin {
        let outpoints: Vec<OutPoint> = transaction
            .input
            .iter()
            .map(|input| input.previous_output)
            .collect();
        {
            let mut pinned = self.in_broadcast_lock();
            for outpoint in &outpoints {
                pinned.entry(*outpoint).or_default().dispatching += 1;
            }
        }
        InBroadcastPin {
            generation: Arc::clone(self),
            outpoints,
            // Fenced by default, and with no anchor yet: a pin that learns
            // nothing before it drops must still hold the inputs. Only a
            // definitive pre-send rejection releases, and only a post-await
            // height sample bounds. See the `InBroadcastPin` type docs
            // (`dashpay/platform#4309`).
            settle: PendingSpendSettle::Unanchored,
        }
    }

    /// The first of `transaction`'s inputs that is currently fenced by a
    /// broadcast dispatch, or `None` when the selection is clear.
    ///
    /// Called by every coin-selection choke point immediately after it built
    /// and reserved a selection, while it still holds the wallet-manager WRITE
    /// guard: a hit means this build's own selection swept an aged reservation
    /// whose transaction is mid-dispatch (or already handed to the network) and
    /// re-reserved its input — completing the build would race that transaction
    /// on the wire, so the caller must release its fresh reservation (exact
    /// under the still-held write guard) and refuse the build. In the normal
    /// case a fenced input is still *reserved* and never reaches selection at
    /// all; this check is the backstop for exactly the post-sweep window.
    ///
    /// `current_height` is the caller's `last_processed_height`, read under the
    /// same write guard — the identical clock the pending-spend bound was
    /// stamped against and the one key-wallet's TTL sweep runs on.
    ///
    /// # Anchoring, before anything is decided
    ///
    /// A dispatch that stopped without a post-await height sample (cancelled or
    /// unwound inside `broadcast`) leaves its fence UNANCHORED, because a
    /// synchronous `Drop` cannot await the manager lock to read the clock. This
    /// call supplies it: every unanchored fence is stamped
    /// [`IN_BROADCAST_FENCE_BLOCKS`](crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS)
    /// past `current_height` BEFORE the reap and the lookup below, so the
    /// interval always runs from a clock at least as recent as the moment the
    /// dispatch stopped — never from the stale pre-await sample that made the
    /// fence arrive already lapsed (`dashpay/platform#4309`).
    ///
    /// Deferring the anchor to here loses nothing: this is the ONLY place the
    /// fence is read, so between a pin's drop and this call there is no path by
    /// which a fenced outpoint could be selected. It also cannot over-hold
    /// across repeated builds — the stamp happens once, and the entry is a
    /// plain bounded fence from then on.
    ///
    /// Lapsed entries are reaped here rather than by a timer: this is the only
    /// place the fence is consulted, so pruning on read keeps the map bounded by
    /// the outpoints dispatched since the last build without any background
    /// task.
    pub(crate) fn in_broadcast_conflict(
        &self,
        transaction: &Transaction,
        current_height: u32,
    ) -> Option<OutPoint> {
        let mut pinned = self.in_broadcast_lock();
        for fence in pinned.values_mut() {
            fence.anchor(current_height);
        }
        pinned.retain(|_, fence| fence.blocks(current_height));
        transaction
            .input
            .iter()
            .map(|input| input.previous_output)
            .find(|outpoint| pinned.contains_key(outpoint))
    }

    /// End one dispatch's hold on `outpoints` — the [`InBroadcastPin`] release
    /// half of [`pin_in_broadcast`](Self::pin_in_broadcast).
    ///
    /// `settle` says what that dispatch proved:
    ///
    /// * [`PendingSpendSettle::AnchoredAt`] — it returned something other than a
    ///   definitive pre-send rejection AND a post-return `last_processed_height`
    ///   was sampled: the dispatching count drops but the outpoint stays fenced
    ///   a full interval past that height.
    /// * [`PendingSpendSettle::Unanchored`] — it stopped without that sample
    ///   (cancelled or unwound mid-`broadcast`). The outpoint stays fenced with
    ///   no bound; [`InBroadcastFence::anchor`] supplies one from the next
    ///   selection's clock.
    /// * [`PendingSpendSettle::Released`] — a definitive pre-send rejection,
    ///   which frees the outpoint immediately: the transaction is provably not
    ///   on the wire, and the caller releases its reservation in the same breath
    ///   so an immediate rebuild can reselect.
    fn unpin_in_broadcast(&self, outpoints: &[OutPoint], settle: PendingSpendSettle) {
        let mut pinned = self.in_broadcast_lock();
        for outpoint in outpoints {
            let Some(fence) = pinned.get_mut(outpoint) else {
                // Unreachable by construction — every pin inserts before its
                // guard can remove — but a miscount must not panic a Drop.
                debug_assert!(false, "unpin of an outpoint that was never pinned");
                continue;
            };
            fence.dispatching = fence.dispatching.saturating_sub(1);
            match settle {
                // Never shorten a fence another dispatch already extended: two
                // concurrent dispatches of the same transaction must both be
                // covered, so the later bound wins.
                PendingSpendSettle::AnchoredAt(height) => fence.extend_pending_until(
                    height.saturating_add(crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS),
                ),
                // Additive alongside any existing bound rather than replacing
                // it: this phase outlasts every anchored one until it is
                // stamped, and stamping keeps the longer of the two.
                PendingSpendSettle::Unanchored => fence.pending_unanchored = true,
                PendingSpendSettle::Released => {}
            }
            if fence.is_clear() {
                pinned.remove(outpoint);
            }
        }
    }
}

/// RAII guard for one dispatch's in-broadcast input fence — see
/// [`WalletGeneration::pin_in_broadcast`]. Dropping it (normal return,
/// unwind, or the dispatching future being cancelled mid-await) ends exactly
/// the dispatching hold that call took, count-wise, never another dispatch's.
///
/// The drop fences the outpoints by DEFAULT, as a pending spend. Only
/// [`release_pending_spend`](Self::release_pending_spend) — called on a
/// definitive pre-send rejection, the one outcome that PROVES the transaction
/// is not on the wire — frees them outright.
///
/// The default is deliberately the conservative one (`dashpay/platform#4309`).
/// This guard's drop runs on paths that carry no information about whether the
/// transaction was sent: the dispatching future cancelled mid-await, an unwind,
/// or a suspension inside the broadcaster before submission. Treating those
/// like a rejection — the previous behaviour — frees inputs that may already be
/// spent on the network, so an immediate reselection double-spends them. Absence
/// of evidence that a send happened is not evidence that it did not, so the
/// fence must survive every exit except the one that proves otherwise.
///
/// # The bound is set by the dispatch, not by the pin's birth
///
/// Fencing by default is only half of it: a fence installed with an
/// ALREADY-LAPSED bound is indistinguishable from no fence. The bound therefore
/// comes from [`anchor_pending_spend`](Self::anchor_pending_spend), called with
/// a `last_processed_height` sampled AFTER the broadcaster returned and while
/// this pin's dispatching phase is still held. A pin that never reaches that
/// call — the cancellation and unwind paths — settles UNANCHORED and blocks
/// unconditionally until the next coin selection stamps it from its own clock.
/// Neither path can consult the pre-await height, because this pin does not
/// carry one (`dashpay/platform#4309`).
pub(crate) struct InBroadcastPin {
    generation: Arc<WalletGeneration>,
    outpoints: Vec<OutPoint>,
    /// How the pending-spend phase settles on drop. Starts
    /// [`PendingSpendSettle::Unanchored`] — the least-informed, most
    /// conservative state — and is narrowed only by an explicit call.
    settle: PendingSpendSettle,
}

impl InBroadcastPin {
    /// Bound the pending-spend fence at `current_height` +
    /// [`IN_BROADCAST_FENCE_BLOCKS`](crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS).
    ///
    /// `current_height` MUST be a `last_processed_height` sampled after the
    /// broadcaster returned, under the wallet-manager guard, while this pin is
    /// still alive. Those three conditions are what make the bound meaningful:
    /// sampling before the await lets a long suspension (minutes, mid-catch-up
    /// on mobile) age the anchor past the whole interval, so the fence lapses
    /// the moment it is installed and the next selection reselects an input of
    /// a transaction that may have reached the network; holding the pin across
    /// the sampling leaves no window between the broadcaster's return and the
    /// stamp; and reading under the manager guard keeps this anchor on the same
    /// clock the freshness check, key-wallet's TTL sweep and
    /// [`WalletGeneration::in_broadcast_conflict`] all use
    /// (`dashpay/platform#4309`).
    ///
    /// Not calling this is always SAFE — the fence simply stays unanchored and
    /// is stamped by the first selection instead — so a cancelled dispatch, an
    /// unwind, or a wallet that left the manager needs no special handling.
    pub(crate) fn anchor_pending_spend(&mut self, current_height: u32) {
        // A rejection already proved nothing was sent; a late anchor must not
        // resurrect a fence the caller deliberately released.
        if self.settle != PendingSpendSettle::Released {
            self.settle = PendingSpendSettle::AnchoredAt(current_height);
        }
    }

    /// Free the outpoints outright on drop, instead of leaving the pending-spend
    /// fence this pin installs by default.
    ///
    /// Call ONLY on a definitive pre-send rejection — the single outcome that
    /// proves the transaction never reached the network, so there is nothing on
    /// the wire to fence against and an immediate retry may reselect the inputs.
    /// An ambiguous outcome, a cancellation, or an unwind must NOT call this:
    /// see the type docs and the `WalletGeneration::in_broadcast` field docs for
    /// why dispatch return is not, by itself, safe.
    pub(crate) fn release_pending_spend(&mut self) {
        self.settle = PendingSpendSettle::Released;
    }
}

impl Drop for InBroadcastPin {
    fn drop(&mut self) {
        self.generation
            .unpin_in_broadcast(&self.outpoints, self.settle);
    }
}

impl Deref for WalletGeneration {
    type Target = WalletBalance;

    fn deref(&self) -> &WalletBalance {
        &self.balance
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use dashcore::{OutPoint, Transaction, TxIn, Txid};

    use super::WalletGeneration;
    use crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS;

    /// The height every test pins at, so a fence installed by
    /// `retain_pending_spend` lapses at `DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS`.
    const DISPATCH_HEIGHT: u32 = 1_000;

    /// A minimal transaction spending exactly the given outpoints — the only
    /// part of a transaction the pin machinery reads.
    fn spending(outpoints: &[OutPoint]) -> Transaction {
        Transaction {
            version: 2,
            lock_time: 0,
            input: outpoints
                .iter()
                .map(|outpoint| TxIn {
                    previous_output: *outpoint,
                    ..Default::default()
                })
                .collect(),
            output: Vec::new(),
            special_transaction_payload: None,
        }
    }

    fn outpoint(byte: u8, vout: u32) -> OutPoint {
        OutPoint::new(Txid::from([byte; 32]), vout)
    }

    /// Settle a pin the way a dispatch that reached the network does: the
    /// broadcaster returned something other than a definitive rejection, and
    /// `last_processed_height` was sampled at `height` AFTER that return. The
    /// anchor is the POST-await reading — `pin_in_broadcast` is deliberately
    /// given no height at all (`dashpay/platform#4309`).
    fn settle_dispatched(generation: &Arc<WalletGeneration>, tx: &Transaction, height: u32) {
        let mut pin = generation.pin_in_broadcast(tx);
        pin.anchor_pending_spend(height);
        drop(pin);
    }

    /// A held pin flags every input of the pinned transaction — and only
    /// those — and dropping a pin whose fence was explicitly RELEASED clears
    /// the conflict. Release models the one outcome that proves nothing was
    /// sent: a definitive pre-send rejection. Every other exit keeps the fence
    /// (see [`dropping_an_unreleased_pin_keeps_the_fence`]).
    #[test]
    fn pin_flags_inputs_until_dropped() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x01, 0);
        let b = outpoint(0x02, 1);
        let unrelated = outpoint(0x03, 0);

        let mut pin = generation.pin_in_broadcast(&spending(&[a, b]));

        // Both pinned inputs conflict; an unrelated selection does not.
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[a]), DISPATCH_HEIGHT),
            Some(a)
        );
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[b]), DISPATCH_HEIGHT),
            Some(b)
        );
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated, a]), DISPATCH_HEIGHT),
            Some(a),
            "a mixed selection must surface its pinned input"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated]), DISPATCH_HEIGHT),
            None
        );

        // A definitive pre-send rejection: nothing is on the wire, so the
        // inputs are free again the moment the guard drops.
        pin.release_pending_spend();
        drop(pin);
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[a, b]), DISPATCH_HEIGHT),
            None,
            "dropping a released pin must clear the conflict"
        );
    }

    /// `dashpay/platform#4309`: the paths this guard's drop actually runs on —
    /// the dispatching future cancelled mid-await, or an unwind — carry NO
    /// information about whether the transaction reached the network. The
    /// previous default freed the inputs there, so an immediate reselection
    /// could double-spend a transaction already on the wire. Dropping without
    /// an explicit release must therefore leave the pending-spend fence
    /// standing, exactly as an ambiguous `MaybeSent` does.
    #[test]
    fn dropping_an_unreleased_pin_keeps_the_fence() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x07, 0);
        let tx = spending(&[a]);

        // Neither `release_pending_spend()` nor `anchor_pending_spend()` —
        // models a dispatch cancelled inside `broadcast`, which reaches
        // neither call.
        drop(generation.pin_in_broadcast(&tx));

        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT),
            Some(a),
            "a cancelled dispatch must NOT free inputs that may be on the wire"
        );
        assert_eq!(
            generation.in_broadcast_conflict(
                &tx,
                DISPATCH_HEIGHT + crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS - 1
            ),
            Some(a),
            "the fence must hold for the full pending-spend bound"
        );
        // Negative control: the fence is bounded, not permanent. The bound is
        // measured from the height of the FIRST consult above, which here is
        // `DISPATCH_HEIGHT`.
        assert_eq!(
            generation.in_broadcast_conflict(
                &tx,
                DISPATCH_HEIGHT + crate::wallet::reservations::IN_BROADCAST_FENCE_BLOCKS
            ),
            None,
            "the fence must lapse once the bound is reached"
        );
    }

    /// `dashpay/platform#4309`, the CANCELLATION half of the stale-anchor
    /// defect. Keeping the fence on cancellation is not enough on its own: the
    /// cancelled dispatch had been suspended inside `broadcast` while catch-up
    /// ran, so a bound measured from anything sampled before that await is
    /// already in the past when the pin drops, and the fence is installed
    /// DEAD — reaped by the very next selection, exactly as if it had never
    /// been installed.
    ///
    /// A cancelled pin therefore settles with no bound at all and is anchored
    /// by the first selection to consult it, on the height THAT selection reads
    /// under the manager write guard. Here the clock has run 10_000 blocks past
    /// where the dispatch started — far beyond `IN_BROADCAST_FENCE_BLOCKS` —
    /// and the fence must still be live, then run a full interval from the
    /// height that observed it.
    #[test]
    fn cancelled_dispatch_fence_anchors_at_the_first_selection_not_at_dispatch() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x08, 0);
        let tx = spending(&[a]);

        // Cancelled mid-`broadcast`: no anchor, no release.
        drop(generation.pin_in_broadcast(&tx));

        // Catch-up ran far past a whole fence interval during the await. A
        // pre-await anchor would have lapsed thousands of blocks ago.
        let observed = DISPATCH_HEIGHT + 10_000;
        assert!(observed > DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS);
        assert_eq!(
            generation.in_broadcast_conflict(&tx, observed),
            Some(a),
            "a fence settled without a post-dispatch height sample must not \
             arrive already lapsed, however far catch-up ran"
        );

        // ...and it is anchored on THAT height, not re-anchored by later reads.
        assert_eq!(
            generation.in_broadcast_conflict(&tx, observed + IN_BROADCAST_FENCE_BLOCKS - 1),
            Some(a),
            "the interval must run a full bound from the observing height"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&tx, observed + IN_BROADCAST_FENCE_BLOCKS),
            None,
            "the anchor is stamped once: repeated reads must not extend the fence"
        );
    }

    /// An unanchored fence outlasts an anchored one for the same outpoint. Two
    /// concurrent dispatches of the same transaction can settle differently —
    /// one returns and anchors, the other is cancelled — and the outpoint must
    /// be covered by the longer of the two. Anchoring at the observing height
    /// is what guarantees that: heights advance, so a bound stamped now is
    /// never shorter than one stamped from an earlier sample.
    #[test]
    fn an_unanchored_fence_outlives_an_anchored_one() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x09, 0);
        let tx = spending(&[a]);

        let cancelled = generation.pin_in_broadcast(&tx);
        settle_dispatched(&generation, &tx, DISPATCH_HEIGHT);
        drop(cancelled);

        // Past the anchored dispatch's bound, the cancelled one still holds.
        let observed = DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS;
        assert_eq!(
            generation.in_broadcast_conflict(&tx, observed),
            Some(a),
            "the cancelled dispatch's unanchored fence must outlast the \
             anchored one"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&tx, observed + IN_BROADCAST_FENCE_BLOCKS),
            None,
            "and it must still lapse a bound past the height that anchored it"
        );
    }

    /// The dispatching pin has no TTL: however far catch-up advances the
    /// clock while the broadcaster is suspended, the inputs stay fenced.
    #[test]
    fn dispatching_pin_never_expires() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x05, 0);
        let tx = spending(&[a]);

        let _pin = generation.pin_in_broadcast(&tx);

        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT + 10_000),
            Some(a),
            "a suspended dispatch must keep its inputs fenced at any height"
        );
    }

    /// `dashpay/platform#4309`: a dispatch that reached the network keeps its
    /// inputs fenced AFTER the broadcaster returns — the DAPI path performs no
    /// local mempool injection, so the wallet has not observed the spend yet
    /// and the outpoint would otherwise be immediately re-selectable. The
    /// fence lapses only once the clock has advanced a full
    /// `IN_BROADCAST_FENCE_BLOCKS` past the dispatch height.
    #[test]
    fn retained_pin_fences_past_dispatch_until_the_bound() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x30, 0);
        let tx = spending(&[a]);

        // Fenced by default; the bound comes from the POST-await sample.
        settle_dispatched(&generation, &tx, DISPATCH_HEIGHT);

        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT),
            Some(a),
            "the fence must survive the dispatch return"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS - 1),
            Some(a),
            "one block below the bound must still fence"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS),
            None,
            "exactly at the bound the fence lapses"
        );
    }

    /// A lapsed fence is reaped, not merely ignored: the read that observes
    /// the lapse is what prunes the entry, so the map cannot grow without
    /// bound across dispatches.
    #[test]
    fn lapsed_fences_are_reaped_on_read() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x31, 0);
        let unrelated = outpoint(0x32, 0);

        settle_dispatched(&generation, &spending(&[a]), DISPATCH_HEIGHT);
        assert_eq!(generation.in_broadcast_lock().len(), 1);

        // A read past the bound — about an unrelated selection — still reaps.
        assert_eq!(
            generation.in_broadcast_conflict(
                &spending(&[unrelated]),
                DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS
            ),
            None
        );
        assert!(
            generation.in_broadcast_lock().is_empty(),
            "the lapsed entry must be pruned by the read that observed the lapse"
        );
    }

    /// The rejection path is the ONLY one that frees inputs at dispatch
    /// return, and it frees them completely — no residual pending-spend fence
    /// keeps an immediate rebuild out.
    #[test]
    fn rejected_dispatch_frees_the_input_immediately() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x33, 0);
        let tx = spending(&[a]);

        // An explicit release — this models `BroadcastError::Rejected`, the one
        // outcome that proves the transaction never reached the network.
        let mut pin = generation.pin_in_broadcast(&tx);
        pin.release_pending_spend();
        drop(pin);

        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT),
            None,
            "a definitively rejected send must not fence its inputs"
        );
        assert!(generation.in_broadcast_lock().is_empty());
    }

    /// An UNANCHORED fence must still be bounded and reaped even when its own
    /// outpoint is never re-selected. Anchoring runs over every entry, not just
    /// the ones the querying transaction spends, so a cancelled dispatch of a
    /// coin nobody touches again cannot sit in the map unbounded forever —
    /// which is the same "it must lapse" property `IN_BROADCAST_FENCE_BLOCKS`
    /// exists to guarantee for funds that are otherwise stranded.
    #[test]
    fn unanchored_fences_are_bounded_and_reaped_by_unrelated_reads() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x34, 0);
        let unrelated = outpoint(0x35, 0);

        // Cancelled dispatch: unanchored, no bound yet.
        drop(generation.pin_in_broadcast(&spending(&[a])));
        assert_eq!(generation.in_broadcast_lock().len(), 1);

        // A read about a DIFFERENT selection anchors it at this height...
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated]), DISPATCH_HEIGHT),
            None
        );
        assert_eq!(
            generation.in_broadcast_lock().len(),
            1,
            "the fence must still stand — anchoring is not releasing"
        );

        // ...so a later unrelated read past that bound reaps it.
        assert_eq!(
            generation.in_broadcast_conflict(
                &spending(&[unrelated]),
                DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS
            ),
            None
        );
        assert!(
            generation.in_broadcast_lock().is_empty(),
            "an unanchored fence must not outlive its bound in the map"
        );
    }

    /// Pins COUNT per outpoint: two concurrent dispatches of the same
    /// transaction (legal through `&SignedCoreTransaction`, idempotent on the
    /// wire) each take a pin, and the fence must hold until the LAST one
    /// returns — the first completion must not unpin the other's in-flight
    /// send.
    #[test]
    fn pins_are_counted_per_outpoint() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x10, 0);
        let tx = spending(&[a]);

        // Both model a definitive rejection, so the count — not a leftover
        // pending-spend fence — is what keeps the outpoint held.
        let mut first = generation.pin_in_broadcast(&tx);
        let mut second = generation.pin_in_broadcast(&tx);
        first.release_pending_spend();
        second.release_pending_spend();

        drop(first);
        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT),
            Some(a),
            "one dispatch still in flight must keep the outpoint fenced"
        );

        drop(second);
        assert_eq!(generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT), None);
    }

    /// Two concurrent dispatches of the same transaction that BOTH reach the
    /// network must leave the longer fence standing — a first completion at a
    /// lower dispatch height must not shorten the second's protection.
    #[test]
    fn the_longer_pending_fence_wins() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x11, 0);
        let tx = spending(&[a]);

        let mut early = generation.pin_in_broadcast(&tx);
        let mut late = generation.pin_in_broadcast(&tx);
        // Each anchors on its OWN post-await sample; the later return sees the
        // higher clock.
        early.anchor_pending_spend(DISPATCH_HEIGHT);
        late.anchor_pending_spend(DISPATCH_HEIGHT + 5);
        drop(late);
        drop(early);

        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT + IN_BROADCAST_FENCE_BLOCKS),
            Some(a),
            "the later dispatch's bound must win over the earlier one's"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&tx, DISPATCH_HEIGHT + 5 + IN_BROADCAST_FENCE_BLOCKS),
            None
        );
    }

    /// Pins are per generation: a re-created wallet's fresh generation starts
    /// with nothing pinned, and the old generation's pins die with its last
    /// handle — nothing leaks across the recreation boundary.
    #[test]
    fn pins_do_not_cross_generations() {
        let old_generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x20, 0);
        let _pin = old_generation.pin_in_broadcast(&spending(&[a]));

        let new_generation = Arc::new(WalletGeneration::new());
        assert_eq!(
            new_generation.in_broadcast_conflict(&spending(&[a]), DISPATCH_HEIGHT),
            None,
            "a fresh generation must not inherit the old generation's pins"
        );
    }
}
