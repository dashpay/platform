//! Per-wallet-*generation* shared state: the identity marker every handle to
//! one generation shares, and that generation's lifecycle gate.

use std::ops::Deref;
use std::sync::Arc;

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
}

impl Default for WalletGeneration {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletGeneration {
    /// A fresh generation: zeroed balance, uncontended gate.
    pub fn new() -> Self {
        Self {
            balance: WalletBalance::new(),
            lifecycle: Arc::new(RwLock::new(())),
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
}

impl Deref for WalletGeneration {
    type Target = WalletBalance;

    fn deref(&self) -> &WalletBalance {
        &self.balance
    }
}
