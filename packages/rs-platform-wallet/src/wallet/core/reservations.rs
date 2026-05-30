//! Generic key reservation set with an RAII guard, plus the
//! [`OutpointReservations`] wrapper used by the platform-wallet
//! UTXO race-protection path.
//!
//! [`OutpointReservations`] closes the same-UTXO concurrent-selection
//! race in [`CoreWallet::send_to_addresses`](super::broadcast). It
//! composes a generic `Reservations<OutPoint>` with a parallel
//! `Reservations<Address>` tracking change addresses peeked but not yet
//! committed to a confirmed broadcast.
//!
//! Both inner sets share the [`Reservations<K>`] core: a single
//! `Mutex`-protected `HashSet<K>` with an RAII [`ReservationGuard<K>`]
//! that releases keys on drop, with `release_after_commit` and
//! `leak_until_sync` escape hatches mirroring the broadcast-success /
//! unrecoverable-leak distinction.
//!
//! This is one of two reservation models in platform-wallet, with a
//! distinct reclaim discipline: here reservations are released on guard
//! **drop** (RAII), bound to a single in-flight broadcast. The sibling
//! [`address_reserve`](super::super::platform_addresses::address_reserve)
//! is a process-global table reclaimed by a **TTL sweep**, for receive /
//! change address hand-outs that have no single owning scope. A change to
//! one's reclaim model should be weighed against the other.

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use dashcore::{Address, OutPoint};

/// Inner shared state for [`Reservations`]. Held behind a `Mutex` so
/// reserve / release commit atomically.
#[derive(Debug)]
struct ReservationsInner<K: Eq + Hash> {
    keys: HashSet<K>,
}

impl<K: Eq + Hash> Default for ReservationsInner<K> {
    fn default() -> Self {
        Self {
            keys: HashSet::new(),
        }
    }
}

/// Generic per-wallet set of keys reserved by an in-flight operation but
/// not yet committed to durable wallet state. Cheaply cloneable: holds
/// an `Arc<Mutex<…>>`. All clones share the same set.
///
/// `K` must be hashable and cloneable (the guard stores its own copy of
/// the reserved keys to release on drop) and `Debug` so the guard prints
/// usefully in `tracing` events.
#[derive(Debug)]
pub(crate) struct Reservations<K: Eq + Hash + Clone + std::fmt::Debug> {
    inner: Arc<Mutex<ReservationsInner<K>>>,
}

impl<K: Eq + Hash + Clone + std::fmt::Debug> Default for Reservations<K> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ReservationsInner::default())),
        }
    }
}

impl<K: Eq + Hash + Clone + std::fmt::Debug> Clone for Reservations<K> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<K: Eq + Hash + Clone + std::fmt::Debug> Reservations<K> {
    /// Test whether `key` is currently reserved. Only used by tests and
    /// the [`OutpointReservations`] wrapper's `#[cfg(test)]` accessors —
    /// production paths use [`snapshot`](Self::snapshot) to filter many
    /// candidates under one lock acquisition.
    #[cfg(test)]
    pub(crate) fn contains(&self, key: &K) -> bool {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.keys.contains(key)
    }

    /// Clone the current reservation set under a single lock acquisition.
    /// Callers filter spendable candidates against the returned snapshot
    /// to avoid one mutex lock per candidate.
    pub(crate) fn snapshot(&self) -> HashSet<K> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.keys.clone()
    }

    /// Reserve `keys`, returning an RAII guard that releases them on drop.
    /// The guard must be held until the operation outcome has been
    /// reconciled into durable wallet state.
    pub(crate) fn reserve(&self, keys: Vec<K>) -> ReservationGuard<K> {
        {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for k in &keys {
                guard.keys.insert(k.clone());
            }
        }
        ReservationGuard {
            reservations: Arc::clone(&self.inner),
            keys,
            released: false,
        }
    }
}

/// RAII guard releasing reservations on drop.
///
/// Drop is infallible and panic-safe — the underlying `Mutex` is
/// recovered from poisoning so a panicking caller still releases its
/// keys.
#[must_use = "dropping the guard immediately releases the reservation"]
pub(crate) struct ReservationGuard<K: Eq + Hash + Clone + std::fmt::Debug> {
    reservations: Arc<Mutex<ReservationsInner<K>>>,
    keys: Vec<K>,
    /// Set after a successful `release_after_commit` so `Drop` is a no-op.
    released: bool,
}

impl<K: Eq + Hash + Clone + std::fmt::Debug> ReservationGuard<K> {
    /// Release the reserved keys now, marking the guard inert so its
    /// `Drop` is a no-op. Called by the caller path after the wallet
    /// state has transitioned the keys from "reserved" to "committed"
    /// (e.g., an outpoint marked spent, an address marked used).
    ///
    /// The deliberate release point exists so the same code path that
    /// *succeeded* the operation also relinquishes the reservation —
    /// separating it from the panic/drop path keeps post-failure
    /// handling on the implicit `Drop` branch.
    pub(crate) fn release_after_commit(mut self) {
        self.do_release();
        self.released = true;
    }

    /// Keep the reservation held until the process exits.
    ///
    /// Use this when the operation succeeded but wallet state could not
    /// be reconciled, OR when the keys must remain reserved across an
    /// asynchronous follow-up event (chain sync, balance flip) that
    /// will eventually commit them out-of-band.
    ///
    /// **No in-process reclaim path exists today.** Keys stay pinned in
    /// [`Reservations`] for the lifetime of the holding manager; only
    /// a process restart (which drops the whole reservations set)
    /// releases them.
    ///
    /// TODO(@thepastaclaw, PR #3585): a periodic-sync-driven
    /// `Reservations::reclaim_committed(&[K])` API would let the next
    /// confirmation / balance-sync pass reconcile leaked reservations
    /// without a restart. Tracked on the PR review thread.
    pub(crate) fn leak_until_sync(self) {
        // `mem::forget` skips `Drop`, leaving the reservation entries
        // in the shared set. The guard's only owned heap allocation is
        // the `Vec<K>`, which is small and never freed for the lifetime
        // of the process — acceptable given this is a rare error / async
        // commit branch.
        std::mem::forget(self);
    }

    fn do_release(&mut self) {
        let mut inner = self
            .reservations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for k in &self.keys {
            inner.keys.remove(k);
        }
    }
}

impl<K: Eq + Hash + Clone + std::fmt::Debug> Drop for ReservationGuard<K> {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        self.do_release();
    }
}

// =====================================================================
// Domain wrapper
// =====================================================================

/// Per-wallet set of outpoints reserved by an in-flight
/// `send_to_addresses` plus any change addresses peeked but not yet
/// reconciled with a confirmed broadcast. Cheaply cloneable.
///
/// Composes a [`Reservations<OutPoint>`] (outpoint slot) with a
/// [`Reservations<Address>`] (pending change address). Two distinct
/// generic instances are used rather than one keyed by an enum because
/// the call-sites filter on each independently (`snapshot` returns just
/// the outpoints, `pending_change_snapshot` just the addresses) and the
/// two sets never overlap. Holding them under separate mutexes is safe
/// — `reserve` always takes them in `(outpoints, change)` order, the
/// only acquisition order in the module.
#[derive(Debug, Default, Clone)]
pub(crate) struct OutpointReservations {
    outpoints: Reservations<OutPoint>,
    pending_change: Reservations<Address>,
}

impl OutpointReservations {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Test whether `outpoint` is currently reserved.
    #[cfg(test)]
    pub(crate) fn contains(&self, outpoint: &OutPoint) -> bool {
        self.outpoints.contains(outpoint)
    }

    /// Test whether a change address is currently pending.
    #[cfg(test)]
    pub(crate) fn change_address_pending(&self, addr: &Address) -> bool {
        self.pending_change.contains(addr)
    }

    /// Clone the current outpoint reservation set under a single lock
    /// acquisition.
    pub(crate) fn snapshot(&self) -> HashSet<OutPoint> {
        self.outpoints.snapshot()
    }

    /// Clone the current pending-change-address set so callers can skip
    /// past in-flight peeks without holding the reservation mutex.
    pub(crate) fn pending_change_snapshot(&self) -> HashSet<Address> {
        self.pending_change.snapshot()
    }

    /// Reserve `outpoints` and (optionally) a chosen change address,
    /// returning an RAII guard that releases both on drop. The guard
    /// must be held until the broadcast outcome is reconciled into
    /// wallet state.
    pub(crate) fn reserve(
        &self,
        outpoints: Vec<OutPoint>,
        change_address: Option<Address>,
    ) -> OutpointReservationGuard {
        let outpoint_guard = self.outpoints.reserve(outpoints);
        let change_guard = change_address.map(|addr| self.pending_change.reserve(vec![addr]));
        OutpointReservationGuard {
            outpoint_guard,
            change_guard,
        }
    }
}

/// RAII guard releasing outpoint + (optional) change-address
/// reservations together. Mirrors the `release_after_commit` /
/// `leak_until_sync` API of the generic guard but composes two inner
/// guards under one type so callers don't have to thread both.
#[must_use = "dropping the guard immediately releases the reservation"]
pub(crate) struct OutpointReservationGuard {
    outpoint_guard: ReservationGuard<OutPoint>,
    change_guard: Option<ReservationGuard<Address>>,
}

impl OutpointReservationGuard {
    /// Release outpoints and any pending change index now, marking the
    /// guard inert so its `Drop` is a no-op. Called by the broadcast
    /// path after `check_core_transaction` has transitioned the inputs
    /// from "reserved" to "spent" and the change index has been
    /// committed via `next_change_address(..., true)`.
    pub(crate) fn release_after_commit(self) {
        self.outpoint_guard.release_after_commit();
        if let Some(g) = self.change_guard {
            g.release_after_commit();
        }
    }

    /// Keep the reservation held until the process exits. See
    /// [`ReservationGuard::leak_until_sync`] for the rationale.
    pub(crate) fn leak_until_sync(self) {
        self.outpoint_guard.leak_until_sync();
        if let Some(g) = self.change_guard {
            g.leak_until_sync();
        }
    }

    /// Total slots this guard holds reserved (outpoints plus an optional
    /// change address). Lets the leak path report how many reservations
    /// are pinned until restart.
    pub(crate) fn reserved_count(&self) -> usize {
        self.outpoint_guard.reserved_count()
            + self
                .change_guard
                .as_ref()
                .map_or(0, ReservationGuard::reserved_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::hashes::Hash;
    use dashcore::Txid;

    fn op(n: u32) -> OutPoint {
        OutPoint::new(Txid::all_zeros(), n)
    }

    fn addr(byte: u8) -> Address {
        use dashcore::secp256k1::{PublicKey, Secp256k1, SecretKey};
        let secp = Secp256k1::new();
        let sk = SecretKey::from_slice(&[byte; 32]).expect("valid sk");
        let pk = PublicKey::from_secret_key(&secp, &sk);
        let cpk = dashcore::PublicKey::new(pk);
        Address::p2pkh(&cpk, dashcore::Network::Testnet)
    }

    #[test]
    fn reserve_then_drop_releases() {
        let res = OutpointReservations::new();
        let a = op(1);
        {
            let _g = res.reserve(vec![a], None);
            assert!(res.contains(&a));
        }
        assert!(!res.contains(&a));
    }

    #[test]
    fn second_reservation_is_disjoint() {
        let res = OutpointReservations::new();
        let a = op(1);
        let b = op(2);
        let _g1 = res.reserve(vec![a], None);
        let _g2 = res.reserve(vec![b], None);
        assert!(res.contains(&a));
        assert!(res.contains(&b));
    }

    #[test]
    fn poisoned_mutex_still_releases() {
        let res = OutpointReservations::new();
        let a = op(7);
        let res_clone = res.clone();
        let _ = std::thread::spawn(move || {
            let _g = res_clone.reserve(vec![a], None);
            panic!("intentional");
        })
        .join();
        // Guard dropped during unwind — outpoint must be released even
        // though the mutex was poisoned.
        assert!(!res.contains(&a));
    }

    #[test]
    fn change_address_reserved_and_released_on_drop() {
        let res = OutpointReservations::new();
        let ch = addr(0x42);
        {
            let _g = res.reserve(vec![op(1)], Some(ch.clone()));
            assert!(res.change_address_pending(&ch));
        }
        assert!(!res.change_address_pending(&ch));
    }

    #[test]
    fn pending_change_snapshot_reflects_reservations() {
        let res = OutpointReservations::new();
        let ch1 = addr(0x11);
        let ch2 = addr(0x22);
        let _g1 = res.reserve(vec![op(1)], Some(ch1.clone()));
        let _g2 = res.reserve(vec![op(2)], Some(ch2.clone()));
        let snap = res.pending_change_snapshot();
        assert!(snap.contains(&ch1));
        assert!(snap.contains(&ch2));
    }

    #[test]
    fn release_after_commit_is_drop_noop() {
        let res = OutpointReservations::new();
        let a = op(11);
        let ch = addr(0x55);
        let g = res.reserve(vec![a], Some(ch.clone()));
        assert!(res.contains(&a));
        assert!(res.change_address_pending(&ch));
        g.release_after_commit();
        assert!(!res.contains(&a));
        assert!(!res.change_address_pending(&ch));
    }

    #[test]
    fn leak_until_sync_keeps_reservation_held() {
        let res = OutpointReservations::new();
        let a = op(13);
        let g = res.reserve(vec![a], None);
        g.leak_until_sync();
        assert!(
            res.contains(&a),
            "leak_until_sync must keep the outpoint reserved until process restart"
        );
    }
}
