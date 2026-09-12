use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use dpp::identity::identity_nonce::{validate_identity_nonce_update, validate_new_identity_nonce};
use dpp::platform_value::Identifier;

#[derive(Debug)]
struct IdentityNonceAttempts {
    committed_nonce: Option<u64>,
    attempted_nonces: BTreeSet<u64>,
}

/// Node-local admission control for expensive proof verification in CheckTx.
///
/// Consensus block processing deliberately does not use this limiter, so
/// public mempool work cannot reserve all proof-verification capacity needed
/// by proposal validation and finalization.
pub struct CheckTxProofVerifier {
    in_flight_weight: AtomicUsize,
    limit: usize,
    identity_nonce_attempts: Mutex<HashMap<[u8; 32], IdentityNonceAttempts>>,
}

impl Default for CheckTxProofVerifier {
    fn default() -> Self {
        let available_cores = std::thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .unwrap_or(1);
        let limit = (available_cores / 4).clamp(1, 4);

        Self::new(limit)
    }
}

impl CheckTxProofVerifier {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            in_flight_weight: AtomicUsize::new(0),
            limit: limit.max(1),
            identity_nonce_attempts: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn try_acquire(
        &self,
        action_count: usize,
    ) -> Option<CheckTxProofVerifierPermit<'_>> {
        // Verification cost grows with the bundle. Charge one local capacity
        // unit per two actions, while allowing a single proof to fit on nodes
        // whose conservative default budget is one unit.
        let weight = action_count.max(1).div_ceil(2).min(self.limit);
        let mut current = self.in_flight_weight.load(Ordering::Acquire);

        loop {
            if current.saturating_add(weight) > self.limit {
                return None;
            }

            match self.in_flight_weight.compare_exchange_weak(
                current,
                current + weight,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(CheckTxProofVerifierPermit {
                        verifier: self,
                        weight,
                    })
                }
                Err(observed) => current = observed,
            }
        }
    }

    /// Admit at most one ShieldFromIdentity proof attempt per identity nonce
    /// while that nonce remains valid against committed state. Platform accepts
    /// a window of missing and future identity nonces, so every attempted nonce
    /// in that window must be retained; remembering only the latest value would
    /// allow a caller to alternate two nonces and repeat proof work indefinitely.
    ///
    /// When committed state changes, attempts that nonce validation now rejects
    /// are pruned. The key is reserved only after global proof capacity is
    /// available, so a busy node cannot strand an honest request.
    pub(crate) fn try_acquire_identity_nonce(
        &self,
        identity_id: [u8; 32],
        committed_nonce: Option<u64>,
        nonce: u64,
        action_count: usize,
    ) -> Option<CheckTxProofVerifierPermit<'_>> {
        let permit = self.try_acquire(action_count)?;
        let mut attempts = self
            .identity_nonce_attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let identity_attempts =
            attempts
                .entry(identity_id)
                .or_insert_with(|| IdentityNonceAttempts {
                    committed_nonce,
                    attempted_nonces: BTreeSet::new(),
                });
        if identity_attempts.committed_nonce != committed_nonce {
            let identifier = Identifier::new(identity_id);
            identity_attempts.attempted_nonces.retain(|attempted| {
                committed_nonce
                    .map(|committed| {
                        validate_identity_nonce_update(committed, *attempted, identifier)
                    })
                    .unwrap_or_else(|| validate_new_identity_nonce(*attempted, identifier))
                    .errors
                    .is_empty()
            });
            identity_attempts.committed_nonce = committed_nonce;
        }

        if !identity_attempts.attempted_nonces.insert(nonce) {
            return None;
        }
        Some(permit)
    }
}

pub(crate) struct CheckTxProofVerifierPermit<'a> {
    verifier: &'a CheckTxProofVerifier,
    weight: usize,
}

impl Drop for CheckTxProofVerifierPermit<'_> {
    fn drop(&mut self) {
        let previous = self
            .verifier
            .in_flight_weight
            .fetch_sub(self.weight, Ordering::AcqRel);
        debug_assert!(
            previous >= self.weight,
            "proof verifier permit counter underflow"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn bounds_and_releases_admitted_work() {
        let verifier = CheckTxProofVerifier::new(3);
        let first = verifier.try_acquire(1).expect("first permit");
        let second = verifier.try_acquire(4).expect("weighted second permit");

        assert!(verifier.try_acquire(1).is_none());

        drop(first);
        assert!(verifier.try_acquire(1).is_some());

        drop(second);
    }

    #[test]
    fn bounds_concurrent_admission_and_releases_all_capacity() {
        const LIMIT: usize = 3;
        const CALLERS: usize = 24;

        let verifier = Arc::new(CheckTxProofVerifier::new(LIMIT));
        let start = Arc::new(Barrier::new(CALLERS));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));

        thread::scope(|scope| {
            for _ in 0..CALLERS {
                let verifier = Arc::clone(&verifier);
                let start = Arc::clone(&start);
                let active = Arc::clone(&active);
                let maximum = Arc::clone(&maximum);
                scope.spawn(move || {
                    start.wait();
                    if let Some(permit) = verifier.try_acquire(1) {
                        let simultaneous = active.fetch_add(1, Ordering::AcqRel) + 1;
                        maximum.fetch_max(simultaneous, Ordering::AcqRel);
                        thread::sleep(Duration::from_millis(10));
                        active.fetch_sub(1, Ordering::AcqRel);
                        drop(permit);
                    }
                });
            }
        });

        assert!(maximum.load(Ordering::Acquire) <= LIMIT);
        assert_eq!(active.load(Ordering::Acquire), 0);

        let permits: Vec<_> = (0..LIMIT)
            .map(|_| verifier.try_acquire(1).expect("released capacity"))
            .collect();
        assert!(verifier.try_acquire(1).is_none());
        drop(permits);
        assert!(verifier.try_acquire(LIMIT * 2).is_some());
    }

    #[test]
    fn rejects_repeated_identity_nonce_attempts_without_blocking_new_nonces() {
        let verifier = CheckTxProofVerifier::new(3);
        let identity_id = [7; 32];

        let first = verifier
            .try_acquire_identity_nonce(identity_id, Some(3), 4, 1)
            .expect("first identity nonce attempt");
        drop(first);

        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, 1)
                .is_none(),
            "the same identity nonce must not repeatedly consume proof capacity"
        );
        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 5, 1)
                .is_some(),
            "an advanced nonce remains admissible"
        );
        assert!(
            verifier
                .try_acquire_identity_nonce([8; 32], Some(3), 4, 1)
                .is_some(),
            "another identity remains admissible"
        );
    }

    #[test]
    fn rejects_alternating_future_nonces_until_committed_state_invalidates_them() {
        let verifier = CheckTxProofVerifier::new(3);
        let identity_id = [7; 32];

        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, 1)
                .expect("first future nonce"),
        );
        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 5, 1)
                .expect("second future nonce"),
        );

        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, 1)
                .is_none(),
            "alternating back to an earlier attempted nonce must not repeat proof work"
        );

        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(5), 6, 1)
                .expect("new nonce after committed state advances"),
        );
        let attempts = verifier
            .identity_nonce_attempts
            .lock()
            .expect("attempts lock");
        assert_eq!(
            attempts
                .get(&identity_id)
                .expect("identity attempts")
                .attempted_nonces,
            BTreeSet::from([6]),
            "nonces that committed state now rejects must be pruned"
        );
    }

    #[test]
    fn retains_attempted_missing_nonce_that_committed_state_still_accepts() {
        let verifier = CheckTxProofVerifier::new(3);
        let identity_id = [7; 32];

        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, 1)
                .expect("future nonce"),
        );

        let committed_with_four_missing = 5 | (1 << 40);
        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(committed_with_four_missing), 4, 1,)
                .is_none(),
            "a still-admissible missing nonce must remain protected after state advances"
        );
    }

    #[test]
    fn admits_only_one_concurrent_attempt_for_an_identity_nonce() {
        const CALLERS: usize = 16;
        let verifier = Arc::new(CheckTxProofVerifier::new(CALLERS));
        let start = Arc::new(Barrier::new(CALLERS));
        let admitted = Arc::new(AtomicUsize::new(0));

        thread::scope(|scope| {
            for _ in 0..CALLERS {
                let verifier = Arc::clone(&verifier);
                let start = Arc::clone(&start);
                let admitted = Arc::clone(&admitted);
                scope.spawn(move || {
                    start.wait();
                    if let Some(permit) =
                        verifier.try_acquire_identity_nonce([7; 32], Some(3), 4, 1)
                    {
                        admitted.fetch_add(1, Ordering::AcqRel);
                        drop(permit);
                    }
                });
            }
        });

        assert_eq!(admitted.load(Ordering::Acquire), 1);
    }

    #[test]
    fn busy_capacity_does_not_reserve_identity_nonce() {
        let verifier = CheckTxProofVerifier::new(1);
        let occupied = verifier.try_acquire(1).expect("occupied capacity");

        assert!(verifier
            .try_acquire_identity_nonce([10; 32], None, 1, 1)
            .is_none());
        drop(occupied);
        assert!(
            verifier
                .try_acquire_identity_nonce([10; 32], None, 1, 1)
                .is_some(),
            "capacity rejection must not consume the identity nonce attempt"
        );
    }
}
