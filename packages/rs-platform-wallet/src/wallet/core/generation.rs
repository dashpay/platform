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
    /// Outpoints currently **pinned by an in-flight broadcast dispatch**
    /// ([`pin_in_broadcast`](Self::pin_in_broadcast)), counted per outpoint.
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
    /// another payment. This map is the non-expiring pin that outlives the
    /// dropped guard: every coin-selection choke point
    /// (`CoreWallet::finalize_transaction`, the contact-payment build, the
    /// asset-lock build) checks its freshly reserved selection against it —
    /// still under the manager write lock, the same synchronization height
    /// advancement and the TTL sweep run under — and refuses a build whose
    /// selection picked a pinned input, closing the sweep + re-reserve window
    /// for as long as the dispatch is in flight.
    ///
    /// A *count* per outpoint rather than a set: `broadcast_finalized_transaction`
    /// takes `&SignedCoreTransaction`, so a direct Rust caller can dispatch the
    /// same transaction twice concurrently (idempotent on the wire — same txid).
    /// Counting keeps the pin held until the LAST dispatch returns instead of
    /// letting the first completion unpin the other's in-flight send.
    ///
    /// A `std::sync::Mutex` like key-wallet's own `ReservationSet`: critical
    /// sections are a few hash operations, never held across an await, and the
    /// sync lock is what lets [`InBroadcastPin::drop`] unpin from a plain
    /// (non-async) `Drop` — which is also what makes the pin
    /// cancellation-safe when the dispatching future is dropped mid-await.
    /// Never persisted: after a restart nothing is mid-dispatch.
    in_broadcast: Mutex<HashMap<OutPoint, u32>>,
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
    fn in_broadcast_lock(&self) -> MutexGuard<'_, HashMap<OutPoint, u32>> {
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
    /// The pin has **no TTL** — a suspended dispatch keeps its inputs fenced no
    /// matter how far catch-up advances the clock — and is released only by
    /// dropping the returned guard object, which happens even when the
    /// dispatching future is cancelled mid-await (`Drop` runs on unwind and on
    /// future drop alike).
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
                *pinned.entry(*outpoint).or_insert(0) += 1;
            }
        }
        InBroadcastPin {
            generation: Arc::clone(self),
            outpoints,
        }
    }

    /// The first of `transaction`'s inputs that is currently pinned by an
    /// in-flight broadcast dispatch, or `None` when the selection is clear.
    ///
    /// Called by every coin-selection choke point immediately after it built
    /// and reserved a selection, while it still holds the wallet-manager WRITE
    /// guard: a hit means this build's own selection swept an aged reservation
    /// whose transaction is mid-dispatch and re-reserved its input — completing
    /// the build would race that transaction on the wire, so the caller must
    /// release its fresh reservation (exact under the still-held write guard)
    /// and refuse the build. In the normal case a pinned input is still
    /// *reserved* and never reaches selection at all; this check is the
    /// backstop for exactly the post-sweep window.
    pub(crate) fn in_broadcast_conflict(&self, transaction: &Transaction) -> Option<OutPoint> {
        let pinned = self.in_broadcast_lock();
        transaction
            .input
            .iter()
            .map(|input| input.previous_output)
            .find(|outpoint| pinned.contains_key(outpoint))
    }

    /// Drop one pin count for each of `outpoints` — the [`InBroadcastPin`]
    /// release half of [`pin_in_broadcast`](Self::pin_in_broadcast).
    fn unpin_in_broadcast(&self, outpoints: &[OutPoint]) {
        let mut pinned = self.in_broadcast_lock();
        for outpoint in outpoints {
            match pinned.get_mut(outpoint) {
                Some(count) if *count > 1 => *count -= 1,
                Some(_) => {
                    pinned.remove(outpoint);
                }
                // Unreachable by construction — every pin increments before its
                // guard can decrement — but a miscount must not panic a Drop.
                None => debug_assert!(false, "unpin of an outpoint that was never pinned"),
            }
        }
    }
}

/// RAII guard for one dispatch's in-broadcast input pins — see
/// [`WalletGeneration::pin_in_broadcast`]. Dropping it (normal return,
/// unwind, or the dispatching future being cancelled mid-await) releases
/// exactly the pins that call took, count-wise, never another dispatch's.
pub(crate) struct InBroadcastPin {
    generation: Arc<WalletGeneration>,
    outpoints: Vec<OutPoint>,
}

impl Drop for InBroadcastPin {
    fn drop(&mut self) {
        self.generation.unpin_in_broadcast(&self.outpoints);
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

    /// A held pin flags every input of the pinned transaction — and only
    /// those — and dropping the pin clears the conflict. This is the RAII
    /// contract the dispatch relies on for cancellation-safety: a dispatching
    /// future dropped mid-await unpins exactly the same way.
    #[test]
    fn pin_flags_inputs_until_dropped() {
        let generation = Arc::new(WalletGeneration::new());
        let a = outpoint(0x01, 0);
        let b = outpoint(0x02, 1);
        let unrelated = outpoint(0x03, 0);

        let pin = generation.pin_in_broadcast(&spending(&[a, b]));

        // Both pinned inputs conflict; an unrelated selection does not.
        assert_eq!(generation.in_broadcast_conflict(&spending(&[a])), Some(a));
        assert_eq!(generation.in_broadcast_conflict(&spending(&[b])), Some(b));
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated, a])),
            Some(a),
            "a mixed selection must surface its pinned input"
        );
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[unrelated])),
            None
        );

        drop(pin);
        assert_eq!(
            generation.in_broadcast_conflict(&spending(&[a, b])),
            None,
            "dropping the pin must clear the conflict"
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

        let first = generation.pin_in_broadcast(&tx);
        let second = generation.pin_in_broadcast(&tx);

        drop(first);
        assert_eq!(
            generation.in_broadcast_conflict(&tx),
            Some(a),
            "one dispatch still in flight must keep the outpoint fenced"
        );

        drop(second);
        assert_eq!(generation.in_broadcast_conflict(&tx), None);
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
            new_generation.in_broadcast_conflict(&spending(&[a])),
            None,
            "a fresh generation must not inherit the old generation's pins"
        );
    }
}
