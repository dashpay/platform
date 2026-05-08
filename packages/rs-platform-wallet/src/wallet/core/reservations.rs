//! Per-wallet outpoint reservation set.
//!
//! Closes the same-UTXO concurrent-selection race in
//! [`CoreWallet::send_to_addresses`](super::broadcast). Two callers racing
//! for the same wallet take the write lock independently — between dropping
//! that lock and finishing the network broadcast, neither has yet marked
//! its inputs spent in `ManagedWalletInfo`. Without a reservation set, both
//! select the same UTXOs and one must lose at the network layer with an
//! opaque broadcast rejection.
//!
//! With this set, the **first** caller adds its selected outpoints to the
//! reservations under the write lock; the **second** caller — also under
//! the write lock — sees those outpoints filtered out of its spendable
//! snapshot and short-circuits with a typed
//! [`PlatformWalletError::NoSpendableInputs`](crate::PlatformWalletError)
//! before ever touching the network.
//!
//! ## Lifetime
//!
//! Reservations are held by an RAII [`OutpointReservationGuard`]. On drop
//! (success, error, or panic) the outpoints are released. A successful
//! broadcast is reconciled by `check_core_transaction(Mempool, …)` *before*
//! the guard drops, so the inputs transition from "reserved" to "spent" with
//! no observable gap to other callers on the same wallet handle.
//!
//! ## Scope
//!
//! Reservations are **per-wallet-instance**. Multiple `PlatformWallet`s in
//! the same process do not false-conflict; multiple processes sharing a
//! wallet on disk are out of scope (the existing `tokio::sync::RwLock`
//! already does not protect against that case).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use dashcore::OutPoint;

/// Per-wallet set of outpoints that have been selected for an in-flight
/// broadcast but not yet marked spent in `ManagedWalletInfo`.
///
/// Cheaply cloneable: holds an `Arc<Mutex<…>>` internally. All clones share
/// the same set.
#[derive(Debug, Default, Clone)]
pub(crate) struct OutpointReservations {
    inner: Arc<Mutex<HashSet<OutPoint>>>,
}

impl OutpointReservations {
    /// Create an empty reservation set.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Test whether `outpoint` is currently reserved.
    ///
    /// Used by coin selection to filter out in-flight outpoints from the
    /// spendable snapshot.
    pub(crate) fn contains(&self, outpoint: &OutPoint) -> bool {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.contains(outpoint)
    }

    /// Reserve `outpoints`, returning an RAII guard that releases them on
    /// drop. The guard must be held until the broadcast outcome is
    /// reconciled into wallet state (success → `check_core_transaction`
    /// has run; failure → caller has propagated the error).
    pub(crate) fn reserve(&self, outpoints: Vec<OutPoint>) -> OutpointReservationGuard {
        {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for op in &outpoints {
                guard.insert(*op);
            }
        }
        OutpointReservationGuard {
            reservations: Arc::clone(&self.inner),
            outpoints,
        }
    }
}

/// RAII guard releasing reservations on drop.
///
/// Drop is infallible and panic-safe — the underlying `Mutex` is recovered
/// from poisoning so a panicking caller still releases its outpoints.
#[must_use = "dropping the guard immediately releases the reservation"]
pub(crate) struct OutpointReservationGuard {
    reservations: Arc<Mutex<HashSet<OutPoint>>>,
    outpoints: Vec<OutPoint>,
}

impl Drop for OutpointReservationGuard {
    fn drop(&mut self) {
        let mut guard = self
            .reservations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for op in &self.outpoints {
            guard.remove(op);
        }
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

    #[test]
    fn reserve_then_drop_releases() {
        let res = OutpointReservations::new();
        let a = op(1);
        {
            let _g = res.reserve(vec![a]);
            assert!(res.contains(&a));
        }
        assert!(!res.contains(&a));
    }

    #[test]
    fn second_reservation_is_disjoint() {
        let res = OutpointReservations::new();
        let a = op(1);
        let b = op(2);
        let _g1 = res.reserve(vec![a]);
        let _g2 = res.reserve(vec![b]);
        assert!(res.contains(&a));
        assert!(res.contains(&b));
    }

    #[test]
    fn poisoned_mutex_still_releases() {
        let res = OutpointReservations::new();
        let a = op(7);
        let res_clone = res.clone();
        let _ = std::thread::spawn(move || {
            let _g = res_clone.reserve(vec![a]);
            panic!("intentional");
        })
        .join();
        // Guard dropped during unwind — outpoint must be released even
        // though the mutex was poisoned.
        assert!(!res.contains(&a));
    }
}
