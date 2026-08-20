//! Per-wallet-*generation* shared state: the identity marker every handle to
//! one generation shares, that generation's lifecycle gate, and the
//! in-broadcast outpoint pins that fence a mid-dispatch transaction's inputs
//! against concurrent re-selection.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

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
    /// * **dispatching** — a counted, never-expiring pin, live from
    ///   check-and-pin until the broadcaster returns.
    /// * **pending-spend** — installed when the broadcaster returns anything
    ///   other than a definitive pre-send rejection, i.e. when the transaction
    ///   may be on the network. It is released when the wallet OBSERVES the
    ///   outpoint spent ([`observe_spent`](Self::observe_spent)), and expires
    ///   only against an orphan backstop
    ///   ([`IN_BROADCAST_FENCE_ORPHAN_TIMEOUT`](crate::wallet::reservations::IN_BROADCAST_FENCE_ORPHAN_TIMEOUT)).
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
    /// # The pending-spend phase ends on EVIDENCE, not on elapsed height
    ///
    /// Three earlier revisions bounded this phase at `height + N` blocks and
    /// argued only about *which* height to anchor on — the pre-send check's, a
    /// post-await sample, a post-await sample taken and installed under one
    /// manager guard. All three are unsound for the same reason, which is not
    /// about the anchor at all: `last_processed_height` is not a clock during
    /// catch-up. The wallet can advance it by thousands of blocks in seconds,
    /// and every one of those blocks was mined BEFORE the transaction was
    /// submitted. Elapsed height says something about the chain's past and
    /// nothing about whether a transaction submitted a moment ago has been seen
    /// or dropped, so an ordinary historical sync completing between the
    /// install and the next build consumes the whole interval and the input
    /// becomes reselectable while the transaction may be on the wire
    /// (`dashpay/platform#4309`, review round 5).
    ///
    /// So the bound is gone. The pending-spend phase is released by exactly one
    /// thing — the wallet observing the outpoint spent, which is positive
    /// evidence that the race the fence exists to prevent can no longer happen:
    ///
    /// * the dispatch's own transaction is seen in the mempool or in a block —
    ///   the spend the fence was protecting has landed; or
    /// * a competing transaction spends the outpoint — the outpoint is gone
    ///   from this wallet's selectable set regardless, so there is nothing left
    ///   to fence.
    ///
    /// [`observe_spent`](Self::observe_spent) is driven from the same
    /// spend-processing path that feeds
    /// [`CoreChangeSet::spent_utxos`](crate::changeset::CoreChangeSet), so the
    /// fence and the persisted spent set agree on what "spent" means by
    /// construction.
    ///
    /// # The backstop is a monotonic wall clock, not a chain quantity
    ///
    /// A fence whose transaction is NEVER observed — evicted for fee, or
    /// conflicted away — would otherwise hold its inputs for the life of the
    /// process with nothing able to clear it. The orphan backstop
    /// ([`IN_BROADCAST_FENCE_ORPHAN_TIMEOUT`](crate::wallet::reservations::IN_BROADCAST_FENCE_ORPHAN_TIMEOUT))
    /// is that release valve, and it is deliberately measured on
    /// [`Instant`](std::time::Instant): a monotonic clock with no chain input,
    /// which catch-up, a re-org, a peer feeding historical headers, or a system
    /// clock adjustment cannot fast-forward. It is a liveness device and is not
    /// claimed to be evidence of anything.
    ///
    /// Reading it needs no lock and no await, so — unlike a height — it is
    /// available from a synchronous `Drop`. That is what collapses the whole
    /// anchored/unanchored split earlier revisions needed: the deadline is
    /// computed inside the same `in_broadcast` critical section that installs
    /// it, on every exit path including cancellation and unwind, so there is no
    /// sample-to-install window left to make atomic.
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
    /// Test-only one-shot hook fired at the dispatching→pending midpoint —
    /// see [`WalletGeneration::on_next_settle_boundary`].
    #[cfg(test)]
    settle_boundary_hook: SettleBoundaryHook,
}

/// Holder for the test-only settle-boundary hook.
///
/// A newtype purely so [`WalletGeneration`] can keep its derived [`Debug`]:
/// `Box<dyn FnOnce>` has none.
#[cfg(test)]
#[derive(Default)]
struct SettleBoundaryHook(Mutex<Option<Box<dyn FnOnce() + Send>>>);

#[cfg(test)]
impl std::fmt::Debug for SettleBoundaryHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SettleBoundaryHook(..)")
    }
}

/// One outpoint's broadcast fence — see `WalletGeneration::in_broadcast`.
#[derive(Debug, Default)]
struct InBroadcastFence {
    /// Dispatches currently *inside* the broadcaster await for this outpoint.
    /// Never expires while non-zero: a suspended dispatch keeps its inputs
    /// fenced no matter what else happens.
    dispatching: u32,
    /// The [`Instant`] at which the pending-spend phase gives up waiting for an
    /// observation and releases the outpoint anyway — the ORPHAN BACKSTOP, for
    /// a transaction the wallet never sees spent at all.
    ///
    /// `None` means no dispatch has handed this outpoint to the network (or an
    /// observation already retired the phase). A monotonic instant, never a
    /// height: see the `WalletGeneration::in_broadcast` field docs for why
    /// every chain-derived bound here was unsound.
    pending_until: Option<Instant>,
    /// The wallet has OBSERVED this outpoint spent
    /// ([`WalletGeneration::observe_spent`]). Retires the pending-spend phase
    /// and suppresses re-installation by a dispatch of the same transaction
    /// that is still inside its broadcaster await — the SPV path routinely
    /// observes the spend before `broadcast` returns, and re-fencing an
    /// already-spent outpoint for a full backstop interval would leave dead
    /// entries in the map for no benefit.
    observed_spent: bool,
}

impl InBroadcastFence {
    /// Whether this fence still blocks re-selection at `now`.
    ///
    /// Takes no height, deliberately. The fence is released by evidence
    /// ([`WalletGeneration::observe_spent`]) and, failing that, by a monotonic
    /// clock — never by chain progress, which during catch-up runs over blocks
    /// that predate the dispatch entirely (`dashpay/platform#4309`).
    fn blocks(&self, now: Instant) -> bool {
        self.dispatching > 0 || self.pending_until.is_some_and(|until| now < until)
    }

    /// Open (or extend) the pending-spend phase, expiring one orphan-backstop
    /// interval past `now`.
    ///
    /// A no-op once the spend has been observed: the evidence that retires the
    /// phase must not be undone by a slower concurrent dispatch of the same
    /// transaction settling afterwards.
    ///
    /// Never SHORTENS an existing deadline. Two concurrent dispatches of the
    /// same transaction must both be covered, so the later one wins.
    fn open_pending(&mut self, now: Instant) {
        if self.observed_spent {
            return;
        }
        let until = now + crate::wallet::reservations::IN_BROADCAST_FENCE_ORPHAN_TIMEOUT;
        self.pending_until = Some(self.pending_until.map_or(until, |cur| cur.max(until)));
    }

    /// Record that the wallet observed this outpoint spent and retire the
    /// pending-spend phase.
    ///
    /// The dispatching count is untouched: it tracks live `InBroadcastPin`s,
    /// not chain state, and a pin must end at its own drop or the count leaks.
    fn observe_spent(&mut self) {
        self.observed_spent = true;
        self.pending_until = None;
    }

    /// Whether nothing holds this outpoint any more, so the entry can be
    /// dropped from the map.
    fn is_clear(&self) -> bool {
        self.dispatching == 0 && self.pending_until.is_none()
    }
}

/// How one dispatch's pending-spend phase settles when its [`InBroadcastPin`]
/// is dropped — see [`WalletGeneration::pin_in_broadcast`].
///
/// The INITIAL value is the least-informed one: a pin that learns nothing
/// before it drops must fence (`dashpay/platform#4309`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PendingSpendSettle {
    /// The transaction may be on the network — the broadcaster returned
    /// something other than a definitive pre-send rejection, or the dispatch
    /// stopped without returning at all (cancelled or unwound mid-`broadcast`).
    /// Both open the pending-spend phase, which then waits for an observed
    /// spend and falls back on the orphan backstop.
    ///
    /// The two cases need no distinction any more. When the phase carried a
    /// height-derived bound they did: a cancelled dispatch had no post-await
    /// sample to anchor on, so it had to fence unanchored and borrow a later
    /// selection's clock. A monotonic [`Instant`] is readable from `Drop`
    /// itself, so both cases stamp their own deadline at the moment they
    /// settle.
    #[default]
    Pending,
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
            #[cfg(test)]
            settle_boundary_hook: SettleBoundaryHook::default(),
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
    /// inputs fenced no matter how long it takes — and ends only when the
    /// returned guard is dropped, which happens even when the dispatching
    /// future is cancelled mid-await (`Drop` runs on unwind and on future drop
    /// alike).
    ///
    /// # No height is taken here, and none is taken later either
    ///
    /// This call takes NO `last_processed_height`, and neither does the settle
    /// that follows it. Chain height cannot bound this fence at all: catch-up
    /// advances it over blocks mined before the transaction was ever submitted,
    /// so any `height + N` bound can be consumed by an ordinary historical sync
    /// without a single piece of evidence about the dispatch
    /// (`dashpay/platform#4309`). The pending-spend phase ends when the wallet
    /// OBSERVES the outpoint spent ([`observe_spent`](Self::observe_spent)),
    /// backstopped by a monotonic [`Instant`] the chain cannot move. Accepting
    /// no height at either end makes the mis-anchoring unrepresentable rather
    /// than merely corrected.
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
            // Fenced by default: a pin that learns nothing before it drops must
            // still hold the inputs. Only a definitive pre-send rejection
            // narrows this. See the `InBroadcastPin` type docs
            // (`dashpay/platform#4309`).
            settle: PendingSpendSettle::Pending,
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
    /// # No height parameter, deliberately
    ///
    /// This used to take the caller's `last_processed_height` and reap every
    /// fence the chain had advanced past. That is the defect: during catch-up
    /// the wallet advances that height over blocks mined BEFORE the dispatch,
    /// so an ordinary historical sync completing between a dispatch and this
    /// call could retire a fence protecting a transaction that had just gone to
    /// the network (`dashpay/platform#4309`, review round 5). The fence now
    /// answers to observed spends and a monotonic clock only, so the chain
    /// clock has no way in.
    ///
    /// Expired entries are reaped here rather than by a timer: this is the only
    /// place the fence is consulted, so pruning on read keeps the map bounded by
    /// the outpoints dispatched since the last build without any background
    /// task.
    pub(crate) fn in_broadcast_conflict(&self, transaction: &Transaction) -> Option<OutPoint> {
        let now = Instant::now();
        let mut pinned = self.in_broadcast_lock();
        pinned.retain(|_, fence| fence.blocks(now));
        transaction
            .input
            .iter()
            .map(|input| input.previous_output)
            .find(|outpoint| pinned.contains_key(outpoint))
    }

    /// Release the pending-spend fence on every outpoint in `outpoints` that
    /// this wallet has just OBSERVED spent.
    ///
    /// This is the fence's real release path — the one that carries evidence.
    /// It is driven off the wallet-event fan-out by
    /// [`SpendObservationHandler`](super::SpendObservationHandler), from the
    /// same per-record input walk that feeds
    /// [`CoreChangeSet::spent_utxos`](crate::changeset::CoreChangeSet), so
    /// "the fence considers this spent" and "the persister removes this UTXO"
    /// are the same fact by construction.
    ///
    /// Both spend shapes are a release, and for the same reason — after either
    /// one there is no longer a selectable outpoint whose re-selection could
    /// race a transaction on the wire:
    ///
    /// * **the dispatch's own transaction**, seen in the mempool or in a block.
    ///   This is the overwhelmingly common case and the one the fence was
    ///   waiting for.
    /// * **a competing transaction** spending the same outpoint. The outpoint
    ///   leaves this wallet's UTXO set either way; continuing to fence it would
    ///   protect nothing and only delay the orphan backstop.
    ///
    /// Idempotent, and safe for outpoints this generation never fenced —
    /// block processing hands over every spend it sees, the vast majority of
    /// which have nothing to do with any dispatch.
    ///
    /// Takes only the `in_broadcast` `std::sync::Mutex` for a few hash
    /// operations and never awaits, so it is safe to call from a synchronous
    /// event handler running inside SPV's block-processing write section.
    pub(crate) fn observe_spent(&self, outpoints: impl IntoIterator<Item = OutPoint>) {
        let mut pinned = self.in_broadcast_lock();
        for outpoint in outpoints {
            let Some(fence) = pinned.get_mut(&outpoint) else {
                continue;
            };
            fence.observe_spent();
            if fence.is_clear() {
                pinned.remove(&outpoint);
            }
        }
    }

    /// End one dispatch's hold on `outpoints` — the [`InBroadcastPin`] release
    /// half of [`pin_in_broadcast`](Self::pin_in_broadcast).
    ///
    /// `settle` says what that dispatch proved:
    ///
    /// * [`PendingSpendSettle::Pending`] — the transaction may be on the
    ///   network (any non-rejection outcome, or a cancelled/unwound dispatch
    ///   that returned nothing at all). The dispatching count drops and the
    ///   pending-spend phase opens, to be released by an observed spend or, if
    ///   none ever arrives, by the orphan backstop.
    /// * [`PendingSpendSettle::Released`] — a definitive pre-send rejection,
    ///   which frees the outpoint immediately: the transaction is provably not
    ///   on the wire, and the caller releases its reservation in the same breath
    ///   so an immediate rebuild can reselect.
    ///
    /// # One critical section, so the handoff is never observable half-done
    ///
    /// Lifting the dispatching hold and opening the pending-spend phase happen
    /// under a single `in_broadcast` lock acquisition, and the backstop
    /// deadline is computed from [`Instant::now`] INSIDE it. There is no
    /// second clock to read and no guard to release in between, so no observer
    /// can catch this outpoint in the torn state — `dispatching` already
    /// lifted, pending-spend not yet open — that would make it briefly
    /// selectable. Earlier revisions sampled a `last_processed_height` from the
    /// wallet-manager lock and had to hold that guard across the install to get
    /// the same property (`dashpay/platform#4309`, review round 4); reading a
    /// monotonic clock needs no guard at all.
    ///
    /// [`Self::settle_boundary_hook`] fires at exactly that midpoint under
    /// `cfg(test)` — after the first outpoint's dispatching hold is lifted and
    /// before its pending phase opens, i.e. inside the torn state itself, not
    /// merely after the lock is acquired. A hook that fired on lock
    /// acquisition would be satisfied by the first half of a split
    /// implementation too; fired here, only a critical section that spans
    /// both halves keeps the boundary unobservable
    /// (`dashpay/platform#4309`, review round 6).
    fn unpin_in_broadcast(&self, outpoints: &[OutPoint], settle: PendingSpendSettle) {
        let now = Instant::now();
        let mut pinned = self.in_broadcast_lock();
        for outpoint in outpoints {
            let Some(fence) = pinned.get_mut(outpoint) else {
                // Unreachable by construction — every pin inserts before its
                // guard can remove — but a miscount must not panic a Drop.
                debug_assert!(false, "unpin of an outpoint that was never pinned");
                continue;
            };
            fence.dispatching = fence.dispatching.saturating_sub(1);
            // The dispatching→pending midpoint: this outpoint's dispatching
            // hold is lifted, its pending phase is not yet open. One-shot, so
            // in effect it fires at the first outpoint's midpoint.
            #[cfg(test)]
            self.fire_settle_boundary_hook();
            match settle {
                PendingSpendSettle::Pending => fence.open_pending(now),
                PendingSpendSettle::Released => {}
            }
            if fence.is_clear() {
                pinned.remove(outpoint);
            }
        }
    }

    /// `outpoint`'s raw fence state — `(dispatching, pending_until,
    /// observed_spent)` — or `None` when nothing holds it.
    ///
    /// A test-only WINDOW ON THE TRANSITION, deliberately not
    /// [`in_broadcast_conflict`](Self::in_broadcast_conflict): that call reaps
    /// as a side effect, so it cannot report whether a dispatch's
    /// dispatching→pending handoff had completed at the moment it was observed.
    #[cfg(test)]
    pub(crate) fn in_broadcast_fence_state(
        &self,
        outpoint: &OutPoint,
    ) -> Option<(u32, Option<Instant>, bool)> {
        self.in_broadcast_lock()
            .get(outpoint)
            .map(|fence| (fence.dispatching, fence.pending_until, fence.observed_spent))
    }

    /// Force `outpoint`'s orphan backstop to have already elapsed.
    ///
    /// The deterministic stand-in for waiting out
    /// [`IN_BROADCAST_FENCE_ORPHAN_TIMEOUT`](crate::wallet::reservations::IN_BROADCAST_FENCE_ORPHAN_TIMEOUT).
    /// Sets the deadline to the current instant, so the next
    /// [`in_broadcast_conflict`](Self::in_broadcast_conflict) — which compares
    /// `now < until` against a later reading of a monotonic clock — sees it
    /// lapsed. Returns whether there was a pending phase to expire.
    #[cfg(test)]
    pub(crate) fn test_elapse_orphan_backstop(&self, outpoint: &OutPoint) -> bool {
        let mut pinned = self.in_broadcast_lock();
        match pinned.get_mut(outpoint) {
            Some(fence) if fence.pending_until.is_some() => {
                fence.pending_until = Some(Instant::now());
                true
            }
            _ => false,
        }
    }

    /// Run `hook` at the dispatching→pending midpoint of the very next
    /// [`unpin_in_broadcast`](Self::unpin_in_broadcast) on this generation:
    /// after an outpoint's dispatching hold is lifted, before its pending
    /// phase is opened — the torn state itself.
    ///
    /// The test-only synchronization hook that makes the handoff regression
    /// DETERMINISTIC (`dashpay/platform#4309`, review round 5 suggestion). The
    /// previous regression parked a writer and hoped the scheduler granted it
    /// the lock inside a window a handful of instructions wide, so it stayed
    /// green against the pre-fix code. With this hook the observer is run at
    /// the midpoint by construction, and what it can see there is the whole
    /// assertion.
    ///
    /// The firing point matters (round 6): fired on lock ACQUISITION, the
    /// observation would complete before any fence was touched, so an
    /// implementation that split the decrement and the pending install into
    /// separate critical sections — the regression under test — would satisfy
    /// it with its first section alone. Fired between the two operations, the
    /// observation is protected only if one critical section spans both.
    ///
    /// One-shot: consumed by the settle that fires it, so an unrelated later
    /// settle cannot re-enter the test's handshake.
    #[cfg(test)]
    pub(crate) fn on_next_settle_boundary(&self, hook: Box<dyn FnOnce() + Send>) {
        *self
            .settle_boundary_hook
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(hook);
    }

    /// A NON-BLOCKING look at `outpoint`'s fence, for an observer that must
    /// distinguish "the transition is in progress" from "the outpoint is free".
    ///
    /// A blocking read cannot make that distinction: correct code holds the
    /// `in_broadcast` lock across the whole dispatching→pending handoff, so an
    /// observer that simply waits for the lock always sees the finished state
    /// and can never tell whether it was granted mid-transition or after it.
    /// Probing with `try_lock` turns "held" into an observable outcome, which is
    /// exactly the invariant the deterministic handoff regression asserts
    /// (`dashpay/platform#4309`, review round 5).
    #[cfg(test)]
    pub(crate) fn try_probe_in_broadcast(&self, outpoint: &OutPoint) -> InBroadcastProbe {
        match self.in_broadcast.try_lock() {
            Err(std::sync::TryLockError::WouldBlock) => InBroadcastProbe::TransitionInProgress,
            Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                Self::probe_entry(poisoned.into_inner().get(outpoint))
            }
            Ok(pinned) => Self::probe_entry(pinned.get(outpoint)),
        }
    }

    #[cfg(test)]
    fn probe_entry(fence: Option<&InBroadcastFence>) -> InBroadcastProbe {
        match fence {
            Some(fence) if fence.blocks(Instant::now()) => InBroadcastProbe::Fenced,
            _ => InBroadcastProbe::Free,
        }
    }

    /// Take and run a hook armed by [`Self::on_next_settle_boundary`], if any.
    ///
    /// Called with the `in_broadcast` lock HELD, which is the point: an
    /// observer that tries to read the fence from another thread while this
    /// runs must find the lock held rather than a half-applied transition.
    #[cfg(test)]
    fn fire_settle_boundary_hook(&self) {
        let hook = self
            .settle_boundary_hook
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }
}

/// What [`WalletGeneration::try_probe_in_broadcast`] saw.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InBroadcastProbe {
    /// The `in_broadcast` lock was held — a settle (or a conflict check) is
    /// mid-flight, so the outpoint cannot be selected by anyone right now.
    TransitionInProgress,
    /// The outpoint carries a live fence.
    Fenced,
    /// Nothing holds the outpoint: a build could select it.
    Free,
}

/// RAII guard for one dispatch's in-broadcast input fence — see
/// [`WalletGeneration::pin_in_broadcast`]. Dropping it (normal return,
/// unwind, or the dispatching future being cancelled mid-await) ends exactly
/// the dispatching hold that call took, count-wise, never another dispatch's.
///
/// The drop fences the outpoints by DEFAULT, as a pending spend. Only
/// [`settle_released`](Self::settle_released) — called on a definitive pre-send
/// rejection, the one outcome that PROVES the transaction is not on the wire —
/// frees them outright.
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
/// # The pending-spend phase waits for evidence, not for a bound to run out
///
/// Fencing by default is only half of it. Three earlier revisions paired that
/// default with a `last_processed_height + N` bound and argued about where to
/// sample the height; all three could be consumed by an ordinary historical
/// catch-up, because those elapsed blocks were mined before the transaction was
/// submitted and say nothing about it (`dashpay/platform#4309`, review round 5).
///
/// The pending-spend phase now ends when the wallet OBSERVES the outpoint spent
/// ([`WalletGeneration::observe_spent`]), with a monotonic-clock orphan backstop
/// so a transaction that is never observed cannot strand its inputs. Both are
/// readable without a lock, a guard or an await, so EVERY exit — normal return,
/// cancellation, unwind — settles the same way and stamps its own deadline. The
/// cancellation path needs no special case at all any more.
pub(crate) struct InBroadcastPin {
    generation: Arc<WalletGeneration>,
    outpoints: Vec<OutPoint>,
    /// How the pending-spend phase settles on drop. Starts
    /// [`PendingSpendSettle::Pending`] — the conservative state — and is
    /// narrowed only by an explicit [`settle_released`](Self::settle_released).
    settle: PendingSpendSettle,
}

impl InBroadcastPin {
    /// End the dispatching phase and open the pending-spend fence, which then
    /// waits for [`WalletGeneration::observe_spent`] and expires only against
    /// the orphan backstop.
    ///
    /// # Why this takes no height, and no guard
    ///
    /// It used to take a `last_processed_height` sampled after the broadcaster
    /// returned, from a wallet-manager guard the caller had to keep held across
    /// the call so no writer could advance the clock between the sample and the
    /// install. Both requirements are gone with the height itself: the orphan
    /// deadline is [`Instant::now`] read INSIDE the `in_broadcast` critical
    /// section that installs it, so there is no second clock, no guard to
    /// release, and no window to protect (`dashpay/platform#4309`).
    ///
    /// # This is equivalent to just dropping the pin
    ///
    /// Kept as an explicit consuming call because it states the dispatch's
    /// verdict at the call site, symmetrically with
    /// [`settle_released`](Self::settle_released) — which is the one that
    /// actually differs from the default. Not calling either is always SAFE:
    /// [`Drop`] settles exactly this way, which is what makes the cancellation
    /// and unwind paths correct without a special case.
    pub(crate) fn settle_pending_spend(self) {
        drop(self);
    }

    /// Free the outpoints outright, consuming the pin, instead of leaving the
    /// pending-spend fence it installs by default.
    ///
    /// Call ONLY on a definitive pre-send rejection — the single outcome that
    /// proves the transaction never reached the network, so there is nothing on
    /// the wire to fence against and an immediate retry may reselect the inputs.
    /// An ambiguous outcome, a cancellation, or an unwind must NOT call this:
    /// see the type docs and the `WalletGeneration::in_broadcast` field docs for
    /// why dispatch return is not, by itself, safe.
    ///
    /// This is the ONLY narrowing of the pin's default. Everything else — an
    /// accepted send, an ambiguous `MaybeSent`, a cancellation, an unwind —
    /// leaves the pending-spend fence in place to await an observed spend.
    pub(crate) fn settle_released(mut self) {
        self.settle = PendingSpendSettle::Released;
        drop(self);
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};

    use dashcore::{OutPoint, Transaction, TxIn, Txid};

    use super::{InBroadcastProbe, WalletGeneration};

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

    /// Settle a pin the way a dispatch that may have reached the network does:
    /// the broadcaster returned something other than a definitive rejection.
    fn settle_dispatched(generation: &Arc<WalletGeneration>, tx: &Transaction) {
        generation.pin_in_broadcast(tx).settle_pending_spend();
    }

    /// A held pin flags every input of the pinned transaction — and only
    /// those — and dropping a pin whose fence was explicitly RELEASED clears
    /// the conflict. Release models the one outcome that proves nothing was
    /// sent: a definitive pre-send rejection.
    #[test]
    fn pin_flags_inputs_until_dropped() {
        let generation = Arc::new(WalletGeneration::new());
        let (a, b, unrelated) = (outpoint(1, 0), outpoint(1, 1), outpoint(2, 0));
        let pinned_tx = spending(&[a, b]);

        let pin = generation.pin_in_broadcast(&pinned_tx);

        assert_eq!(generation.in_broadcast_conflict(&spending(&[a])), Some(a));
        assert_eq!(generation.in_broadcast_conflict(&spending(&[b])), Some(b));
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated, a])),
            Some(a),
            "the conflict is reported for whichever input is fenced"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated])),
            None,
            "an unrelated input is untouched by the pin"
        );

        pin.settle_released();
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[a, b])),
            None,
            "a released pin frees its outpoints outright"
        );
    }

    /// `dashpay/platform#4309`: dropping a pin WITHOUT a definitive rejection
    /// keeps the fence. This is the cancellation / unwind / suspension path,
    /// none of which proves the transaction failed to reach the network.
    #[test]
    fn dropping_an_unreleased_pin_keeps_the_fence() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(3, 0);
        let tx = spending(&[a]);

        drop(generation.pin_in_broadcast(&tx));

        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            Some(a),
            "an un-released pin must leave the outpoint fenced on drop"
        );
    }

    /// THE HEADLINE PROPERTY (`dashpay/platform#4309`, review round 5).
    ///
    /// A pending-spend fence is not consulted against chain height at all, so
    /// no amount of catch-up can retire it. Previous revisions bounded the
    /// fence at `height + IN_BROADCAST_FENCE_BLOCKS` and every one of them lost
    /// the fence to a historical sync that advanced the clock past the bound
    /// over blocks mined BEFORE the dispatch.
    ///
    /// `in_broadcast_conflict` no longer takes a height, so this test states
    /// the property the only way it can still be stated: the fence survives
    /// unboundedly many consultations and any amount of elapsed chain, and only
    /// an observation (or the orphan backstop) clears it.
    #[test]
    fn a_pending_fence_is_immune_to_chain_progress() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(4, 0);
        let tx = spending(&[a]);

        settle_dispatched(&generation, &tx);

        // Stand in for an arbitrarily long catch-up: every build during it
        // consults the fence, and each consultation also reaps. None of them
        // may retire this entry.
        for _ in 0..10_000 {
            assert_eq!(
                generation.in_broadcast_conflict(&tx),
                Some(a),
                "no number of selections — i.e. no amount of chain progress — \
                 may retire a fence that has seen no observed spend"
            );
        }
    }

    /// The fence's real release: the wallet observes the outpoint spent by the
    /// dispatch's OWN transaction.
    #[test]
    fn an_observed_spend_clears_the_fence() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(5, 0);
        let tx = spending(&[a]);

        settle_dispatched(&generation, &tx);
        assert_eq!(generation.in_broadcast_conflict(&tx), Some(a));

        generation.observe_spent([a]);

        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            None,
            "observing the spend is what ends the fence"
        );
    }

    /// A COMPETING spend clears the fence too, and for the same reason: after
    /// it the outpoint is out of this wallet's selectable set, so there is no
    /// re-selection left that could race anything on the wire.
    #[test]
    fn a_competing_spend_also_clears_the_fence() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(6, 0);
        let ours = spending(&[a]);

        settle_dispatched(&generation, &ours);

        // A different transaction spending the same outpoint — the wallet sees
        // it and hands the outpoint over as spent.
        let competing = spending(&[a, outpoint(7, 0)]);
        generation.observe_spent(competing.input.iter().map(|i| i.previous_output));

        assert_eq!(generation.in_broadcast_conflict(&ours), None);
    }

    /// Observing spends the fence never knew about is a harmless no-op — block
    /// processing hands over every spend it sees, and almost none of them
    /// belong to a dispatch.
    #[test]
    fn observing_unfenced_outpoints_is_a_no_op() {
        let generation = Arc::new(WalletGeneration::new());
        let fenced = outpoint(8, 0);
        let tx = spending(&[fenced]);
        settle_dispatched(&generation, &tx);

        generation.observe_spent([outpoint(9, 0), outpoint(9, 1)]);
        generation.observe_spent([]);

        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            Some(fenced),
            "unrelated observations must not disturb a live fence"
        );
    }

    /// The DISPATCHING phase never expires and is never released by an
    /// observation: it tracks a live `InBroadcastPin`, so it ends at that pin's
    /// drop and nowhere else. An observation arriving mid-dispatch (the SPV
    /// path routinely beats the broadcaster's return) still suppresses the
    /// pending phase the settle would otherwise open.
    #[test]
    fn a_mid_dispatch_observation_suppresses_the_pending_phase() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(10, 0);
        let tx = spending(&[a]);

        let pin = generation.pin_in_broadcast(&tx);
        generation.observe_spent([a]);

        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            Some(a),
            "the dispatching hold outlives an observation — the pin is still live"
        );

        pin.settle_pending_spend();

        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            None,
            "an already-observed spend must not be re-fenced by the settle, or \
             the map would carry a dead entry for a whole backstop interval"
        );
        assert_eq!(
            generation.in_broadcast_fence_state(&a),
            None,
            "and the entry is reaped rather than left behind"
        );
    }

    /// The orphan backstop exists so a transaction that is NEVER observed
    /// cannot strand its inputs for the life of the process.
    #[test]
    fn the_orphan_backstop_eventually_frees_an_unobserved_fence() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(11, 0);
        let tx = spending(&[a]);

        settle_dispatched(&generation, &tx);
        assert_eq!(generation.in_broadcast_conflict(&tx), Some(a));

        assert!(
            generation.test_elapse_orphan_backstop(&a),
            "the settled fence must carry a backstop deadline to elapse"
        );

        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            None,
            "an unobserved fence must eventually lapse rather than strand the input"
        );
    }

    /// The backstop is on a MONOTONIC clock, so it is not reachable from chain
    /// state — which is the whole point of moving off `last_processed_height`.
    /// A dispatching pin is not subject to it either.
    #[test]
    fn the_backstop_does_not_apply_while_dispatching() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(12, 0);
        let tx = spending(&[a]);

        let pin = generation.pin_in_broadcast(&tx);
        assert!(
            !generation.test_elapse_orphan_backstop(&a),
            "a dispatching pin has no backstop deadline — it cannot expire at all"
        );
        assert_eq!(generation.in_broadcast_conflict(&tx), Some(a));

        drop(pin);
    }

    /// Two concurrent dispatches of the same transaction: the pin is COUNTED,
    /// so the first completion must not unpin the second's in-flight send.
    #[test]
    fn pins_are_counted_per_outpoint() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(13, 0);
        let tx = spending(&[a]);

        let first = generation.pin_in_broadcast(&tx);
        let second = generation.pin_in_broadcast(&tx);

        first.settle_released();
        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            Some(a),
            "one dispatch's rejection must not free another's in-flight inputs"
        );

        second.settle_released();
        assert_eq!(generation.in_broadcast_conflict(&tx), None);
    }

    /// A settle must never SHORTEN a deadline another dispatch already
    /// installed: both dispatches have to stay covered.
    #[test]
    fn the_longer_pending_fence_wins() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(14, 0);
        let tx = spending(&[a]);

        settle_dispatched(&generation, &tx);
        let first_deadline = generation
            .in_broadcast_fence_state(&a)
            .and_then(|(_, until, _)| until)
            .expect("a settled fence carries a deadline");

        // A second, later dispatch of the same transaction.
        settle_dispatched(&generation, &tx);
        let second_deadline = generation
            .in_broadcast_fence_state(&a)
            .and_then(|(_, until, _)| until)
            .expect("still fenced");

        assert!(
            second_deadline >= first_deadline,
            "the later dispatch's deadline must win, never be rolled back"
        );
    }

    /// Fences are per GENERATION: a wallet re-created under the same id gets a
    /// fresh map and cannot be blocked by the previous instance's dispatches.
    #[test]
    fn pins_do_not_cross_generations() {
        let first = Arc::new(WalletGeneration::new());
        let second = Arc::new(WalletGeneration::new());
        let a = outpoint(15, 0);
        let tx = spending(&[a]);

        let _pin = first.pin_in_broadcast(&tx);

        assert_eq!(first.in_broadcast_conflict(&tx), Some(a));
        assert_eq!(
            second.in_broadcast_conflict(&tx),
            None,
            "a different generation's fence must not block this one's builds"
        );
    }

    /// Lapsed entries are reaped on read, so the map stays bounded by the
    /// outpoints dispatched since the last build without any background task.
    #[test]
    fn lapsed_fences_are_reaped_on_read() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(16, 0);
        let tx = spending(&[a]);

        settle_dispatched(&generation, &tx);
        assert!(generation.test_elapse_orphan_backstop(&a));

        // Reading about an UNRELATED transaction still reaps.
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[outpoint(17, 0)])),
            None
        );
        assert_eq!(
            generation.in_broadcast_fence_state(&a),
            None,
            "the lapsed entry must be gone from the map, not merely inert"
        );
    }

    /// `dashpay/platform#4309`, REVIEW ROUND 5 SUGGESTION: THE
    /// DISPATCHING→PENDING HANDOFF REGRESSION, MADE DETERMINISTIC.
    ///
    /// The transition lifts the dispatching hold and opens the pending-spend
    /// phase. If those two ever land in separate critical sections, an observer
    /// in between sees the outpoint held by NOTHING and a build can select an
    /// input whose transaction may be on the wire.
    ///
    /// The previous regression parked a manager writer and hoped the scheduler
    /// granted it the lock inside a window a handful of instructions wide. It
    /// did not reliably do so — the reviewer showed it stays green against the
    /// pre-fix code — so it proved nothing.
    ///
    /// This one is deterministic. [`WalletGeneration::on_next_settle_boundary`]
    /// runs the observer AT the midpoint by construction — after the
    /// dispatching hold is lifted, before the pending phase opens, so the
    /// probe lands inside the torn state itself rather than before any fence
    /// was touched (round 6) — and the settling thread BLOCKS until the
    /// observer has published what it saw, so there is no race to lose. The
    /// observer probes with `try_lock`
    /// ([`WalletGeneration::try_probe_in_broadcast`]) rather than blocking,
    /// because a blocking read cannot distinguish "held across the whole
    /// transition" — the property under test — from "granted after it".
    ///
    /// Legal: `TransitionInProgress`. Illegal: `Free`, which is exactly what a
    /// split transition would expose.
    #[test]
    fn the_settle_handoff_is_never_observable_half_done() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(18, 0);
        let tx = spending(&[a]);

        let pin = generation.pin_in_broadcast(&tx);

        // Hand the observer its own generation handle; it runs on another
        // thread, woken exactly at the midpoint.
        let (at_midpoint_tx, at_midpoint_rx) = mpsc::channel::<()>();
        let (observed_tx, observed_rx) = mpsc::channel::<InBroadcastProbe>();
        let observer = std::thread::spawn({
            let generation = Arc::clone(&generation);
            move || {
                at_midpoint_rx.recv().expect("midpoint signal");
                let probe = generation.try_probe_in_broadcast(&a);
                observed_tx.send(probe).expect("publish observation");
            }
        });

        // At the midpoint: wake the observer and do not proceed until it has
        // published. That is what removes the scheduling race — the settle is
        // provably still in progress while the observation is taken.
        generation.on_next_settle_boundary(Box::new(move || {
            at_midpoint_tx.send(()).expect("wake observer");
            let probe = observed_rx.recv().expect("observation");
            assert_ne!(
                probe,
                InBroadcastProbe::Free,
                "the dispatching→pending handoff was observable half-done: the \
                 outpoint was held by nothing mid-transition, so a build could \
                 have selected an input whose transaction may be on the wire"
            );
        }));

        pin.settle_pending_spend();
        observer.join().expect("observer thread");

        // And the end state is a live fence, not merely an unobservable
        // transition into nothing.
        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            Some(a),
            "the settled fence must be live once the transition completes"
        );
    }

    /// The settle-boundary hook is ONE-SHOT, so a test's handshake cannot be
    /// re-entered by an unrelated later settle on the same generation.
    #[test]
    fn the_settle_boundary_hook_fires_once() {
        let generation = Arc::new(WalletGeneration::new());
        let fired = Arc::new(AtomicUsize::new(0));
        let tx = spending(&[outpoint(19, 0)]);

        generation.on_next_settle_boundary(Box::new({
            let fired = Arc::clone(&fired);
            move || {
                fired.fetch_add(1, Ordering::SeqCst);
            }
        }));

        settle_dispatched(&generation, &tx);
        settle_dispatched(&generation, &tx);

        assert_eq!(fired.load(Ordering::SeqCst), 1);
    }
}
