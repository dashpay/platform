//! Per-wallet outpoint reservation set for [`CoreWallet::send_to_addresses`](super::broadcast).
//!
//! Closes the same-UTXO concurrent-selection race: the first caller reserves its selected
//! outpoints under the write lock; subsequent callers filter them out and short-circuit with
//! [`PlatformWalletError::NoSpendableInputs`](crate::PlatformWalletError) before hitting the
//! network. Reservations are released by an RAII guard on success, error, or panic.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use dashcore::{Address, OutPoint};

/// Inner state shared between an [`OutpointReservations`] handle and every
/// outstanding [`OutpointReservationGuard`]. Held behind a single `Mutex`
/// so reservation + change-address tracking commit atomically.
#[derive(Debug, Default)]
struct ReservationsInner {
    outpoints: HashSet<OutPoint>,
    /// Change addresses already committed (`advance=true`) by an
    /// in-flight `send_to_addresses` whose broadcast has not yet
    /// completed. Concurrent senders that peek a change address still
    /// present here advance past it under the same write lock so two
    /// disjoint-UTXO sends do not both broadcast with the same change
    /// address (privacy regression — CMT-006). Address-keyed rather than
    /// `(account, index)` because the upstream pool API exposes addresses
    /// but not indices.
    pending_change: HashSet<Address>,
}

/// Per-wallet set of outpoints that have been selected for an in-flight
/// broadcast but not yet marked spent in `ManagedWalletInfo`, plus any
/// change addresses peeked but not yet reconciled with a confirmed
/// broadcast.
///
/// Cheaply cloneable: holds an `Arc<Mutex<…>>` internally. All clones share
/// the same set.
#[derive(Debug, Default, Clone)]
pub(crate) struct OutpointReservations {
    inner: Arc<Mutex<ReservationsInner>>,
}

impl OutpointReservations {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Test whether `outpoint` is currently reserved.
    #[cfg(test)]
    pub(crate) fn contains(&self, outpoint: &OutPoint) -> bool {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.outpoints.contains(outpoint)
    }

    /// Test whether a change address is currently pending.
    #[cfg(test)]
    pub(crate) fn change_address_pending(&self, addr: &Address) -> bool {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.pending_change.contains(addr)
    }

    /// Clone the current outpoint reservation set under a single lock
    /// acquisition. Callers filter spendable UTXOs against the returned
    /// snapshot to avoid one mutex lock per candidate outpoint.
    pub(crate) fn snapshot(&self) -> HashSet<OutPoint> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.outpoints.clone()
    }

    /// Clone the current pending-change-address set so callers can skip
    /// past in-flight peeks without holding the reservation mutex.
    pub(crate) fn pending_change_snapshot(&self) -> HashSet<Address> {
        let guard = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.pending_change.clone()
    }

    /// Reserve `outpoints` and (optionally) a chosen change address in
    /// the same lock acquisition, returning an RAII guard that releases
    /// both on drop. The guard must be held until the broadcast outcome
    /// is reconciled into wallet state.
    pub(crate) fn reserve(
        &self,
        outpoints: Vec<OutPoint>,
        change_address: Option<Address>,
    ) -> OutpointReservationGuard {
        {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for op in &outpoints {
                guard.outpoints.insert(*op);
            }
            if let Some(addr) = &change_address {
                guard.pending_change.insert(addr.clone());
            }
        }
        OutpointReservationGuard {
            reservations: Arc::clone(&self.inner),
            outpoints,
            pending_change: change_address,
            released: false,
        }
    }
}

/// RAII guard releasing reservations on drop.
///
/// Drop is infallible and panic-safe — the underlying `Mutex` is recovered
/// from poisoning so a panicking caller still releases its outpoints
/// and pending change index (if any).
#[must_use = "dropping the guard immediately releases the reservation"]
pub(crate) struct OutpointReservationGuard {
    reservations: Arc<Mutex<ReservationsInner>>,
    outpoints: Vec<OutPoint>,
    /// Pending change address reserved for this in-flight send (if any).
    pending_change: Option<Address>,
    /// Set after a successful `release_after_commit` so `Drop` is a no-op.
    released: bool,
}

impl OutpointReservationGuard {
    /// Release outpoints and any pending change index now, marking the
    /// guard inert so its `Drop` is a no-op. Called by the broadcast
    /// path after `check_core_transaction` has transitioned the inputs
    /// from "reserved" to "spent" and the change index has been
    /// committed via `next_change_address(..., true)`. The deliberate
    /// release point exists so the same code path that *succeeded* the
    /// broadcast also relinquishes the reservation — separating it from
    /// the panic/drop path keeps post-broadcast-failure handling
    /// (CMT-003) on the implicit `Drop` branch.
    pub(crate) fn release_after_commit(mut self) {
        self.do_release();
        self.released = true;
    }

    /// Keep the reservation held for the lifetime of the process by
    /// leaking the guard. Use this when the broadcast succeeded but
    /// wallet state could not be reconciled (e.g., own-built tx not
    /// recognised by `check_core_transaction`, or the wallet handle
    /// went stale post-broadcast). Releasing the outpoints in that
    /// scenario would let a concurrent caller select the same UTXO and
    /// produce a double-spend the network would reject — keeping the
    /// reservation is the safer of two bad outcomes; a wallet restart
    /// or full sync will reconcile.
    pub(crate) fn leak_until_sync(self) {
        // `Box::leak` is the standard way to drop the ownership without
        // running `Drop`. We don't actually heap-allocate — `mem::forget`
        // is equivalent and avoids the allocation.
        std::mem::forget(self);
    }

    fn do_release(&mut self) {
        let mut inner = self
            .reservations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for op in &self.outpoints {
            inner.outpoints.remove(op);
        }
        if let Some(addr) = self.pending_change.take() {
            inner.pending_change.remove(&addr);
        }
    }
}

impl Drop for OutpointReservationGuard {
    fn drop(&mut self) {
        if self.released {
            return;
        }
        self.do_release();
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
