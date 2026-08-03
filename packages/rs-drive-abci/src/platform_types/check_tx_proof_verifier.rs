use std::sync::atomic::{AtomicUsize, Ordering};

/// Node-local admission control for expensive proof verification in CheckTx.
///
/// Consensus block processing deliberately does not use this limiter, so
/// public mempool work cannot reserve all proof-verification capacity needed
/// by proposal validation and finalization.
pub struct CheckTxProofVerifier {
    in_flight_weight: AtomicUsize,
    limit: usize,
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
}
