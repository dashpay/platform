use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use dpp::identity::identity_nonce::{validate_identity_nonce_update, validate_new_identity_nonce};
use dpp::platform_value::Identifier;

// Node-local resource controls, not consensus limits. Weighted permits bound
// concurrent proof verification, while ordinary inactive-entry LRU eviction
// bounds identity-cache memory without imposing a separate newcomer quota.
// Cached successes perform no proof work.
const MAX_TRACKED_IDENTITIES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerifiedIdentityProof {
    transaction_hash: [u8; 32],
    protocol_version: u32,
}

#[derive(Debug)]
struct IdentityNonceAttempts {
    protocol_version: u32,
    committed_nonce: Option<u64>,
    attempted_nonces: BTreeMap<u64, Option<VerifiedIdentityProof>>,
    in_flight: usize,
    last_access: Instant,
}

#[derive(Default)]
struct IdentityNonceCache {
    identities: HashMap<[u8; 32], IdentityNonceAttempts>,
}

/// Node-local admission control for expensive proof verification in CheckTx.
///
/// Consensus block processing deliberately does not use this limiter, so
/// public mempool work cannot reserve all proof-verification capacity needed
/// by proposal validation and finalization.
pub struct CheckTxProofVerifier {
    in_flight_weight: AtomicUsize,
    limit: usize,
    identity_cache_limit: usize,
    identity_nonce_attempts: Mutex<IdentityNonceCache>,
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

pub(crate) enum IdentityProofVerification<'a> {
    Cached,
    Required(CheckTxProofVerifierPermit<'a>),
}

impl CheckTxProofVerifier {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            in_flight_weight: AtomicUsize::new(0),
            limit: limit.max(1),
            identity_cache_limit: MAX_TRACKED_IDENTITIES,
            identity_nonce_attempts: Mutex::new(IdentityNonceCache::default()),
        }
    }

    pub(crate) fn try_acquire(
        &self,
        action_count: usize,
    ) -> Option<CheckTxProofVerifierPermit<'_>> {
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
                        identity: None,
                    })
                }
                Err(observed) => current = observed,
            }
        }
    }

    /// Reuse only an exact, previously successful proof. The caller must still
    /// validate current signatures, nonce, state and fees before using this cache.
    /// Failed and in-flight attempts reserve their nonce while the identity is
    /// retained. Inactive eviction can reopen later verification, but every
    /// such attempt must still acquire a weighted concurrent proof permit.
    pub(crate) fn try_acquire_identity_nonce(
        &self,
        identity_id: [u8; 32],
        committed_nonce: Option<u64>,
        nonce: u64,
        transaction_hash: [u8; 32],
        action_count: usize,
        protocol_version: u32,
    ) -> Option<IdentityProofVerification<'_>> {
        self.try_acquire_identity_nonce_at(
            identity_id,
            committed_nonce,
            nonce,
            VerifiedIdentityProof {
                transaction_hash,
                protocol_version,
            },
            action_count,
            Instant::now(),
        )
    }

    fn try_acquire_identity_nonce_at(
        &self,
        identity_id: [u8; 32],
        committed_nonce: Option<u64>,
        nonce: u64,
        proof: VerifiedIdentityProof,
        action_count: usize,
        now: Instant,
    ) -> Option<IdentityProofVerification<'_>> {
        let identifier = Identifier::new(identity_id);
        let nonce_valid = |attempted| {
            committed_nonce
                .map(|committed| validate_identity_nonce_update(committed, attempted, identifier))
                .unwrap_or_else(|| validate_new_identity_nonce(attempted, identifier))
                .is_valid()
        };
        // Also enforce the finite nonce window here so every insertion preserves
        // the memory bound, independently of the caller's admission checks.
        if !nonce_valid(nonce) {
            return None;
        }
        let mut cache = self
            .identity_nonce_attempts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(attempts) = cache.identities.get_mut(&identity_id) {
            if proof.protocol_version < attempts.protocol_version {
                return None;
            }
            if proof.protocol_version > attempts.protocol_version {
                if attempts.in_flight != 0 {
                    return None;
                }
                // A new active protocol may change proof rules. Reverify once
                // under that version; stale callers cannot toggle back and
                // repeatedly reopen the old nonce window.
                attempts.protocol_version = proof.protocol_version;
                attempts.attempted_nonces.clear();
            }
            if attempts.committed_nonce != committed_nonce {
                attempts
                    .attempted_nonces
                    .retain(|attempted, _| nonce_valid(*attempted));
                attempts.committed_nonce = committed_nonce;
            }
            if let Some(verified) = attempts.attempted_nonces.get(&nonce) {
                if *verified == Some(proof) {
                    attempts.last_access = now;
                    return Some(IdentityProofVerification::Cached);
                }
                // Rejected repeats must not keep an attacker's failed entry hot.
                return None;
            }
        }

        // Reject busy work or an all-in-flight cache without consuming a nonce.
        // The temporary permit is not identity-bound yet.
        let mut permit = self.try_acquire(action_count)?;
        let victim = if !cache.identities.contains_key(&identity_id)
            && cache.identities.len() >= self.identity_cache_limit
        {
            Some(
                cache
                    .identities
                    .iter()
                    .filter(|(_, attempts)| attempts.in_flight == 0)
                    .min_by_key(|(_, attempts)| attempts.last_access)
                    .map(|(id, _)| *id)?,
            )
        } else {
            None
        };
        if let Some(victim) = victim {
            cache.identities.remove(&victim);
        }
        let attempts =
            cache
                .identities
                .entry(identity_id)
                .or_insert_with(|| IdentityNonceAttempts {
                    protocol_version: proof.protocol_version,
                    committed_nonce,
                    attempted_nonces: BTreeMap::new(),
                    in_flight: 0,
                    last_access: now,
                });
        attempts.last_access = now;
        attempts.attempted_nonces.insert(nonce, None);
        attempts.in_flight += 1;
        permit.identity = Some((identity_id, nonce, proof));
        Some(IdentityProofVerification::Required(permit))
    }
}

pub(crate) struct CheckTxProofVerifierPermit<'a> {
    verifier: &'a CheckTxProofVerifier,
    weight: usize,
    identity: Option<([u8; 32], u64, VerifiedIdentityProof)>,
}

impl CheckTxProofVerifierPermit<'_> {
    /// Call only after Orchard verification succeeded. Dropping an unmarked
    /// permit retains a failed attempt, never a successful cached result.
    pub(crate) fn mark_verified(&self) {
        if let Some((identity_id, nonce, proof)) = self.identity {
            let mut cache = self
                .verifier
                .identity_nonce_attempts
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(attempt) = cache
                .identities
                .get_mut(&identity_id)
                .and_then(|attempts| attempts.attempted_nonces.get_mut(&nonce))
            {
                *attempt = Some(proof);
            }
        }
    }
}

impl Drop for CheckTxProofVerifierPermit<'_> {
    fn drop(&mut self) {
        if let Some((identity_id, _, _)) = self.identity {
            let mut cache = self
                .verifier
                .identity_nonce_attempts
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(attempts) = cache.identities.get_mut(&identity_id) {
                attempts.in_flight -= 1;
            }
        }
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
            .try_acquire_identity_nonce(identity_id, Some(3), 4, [0; 32], 1, 1)
            .expect("first identity nonce attempt");
        drop(first);

        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, [0; 32], 1, 1)
                .is_none(),
            "the same identity nonce must not repeatedly consume proof capacity"
        );
        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 5, [0; 32], 1, 1)
                .is_some(),
            "an advanced nonce remains admissible"
        );
        assert!(
            verifier
                .try_acquire_identity_nonce([8; 32], Some(3), 4, [0; 32], 1, 1)
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
                .try_acquire_identity_nonce(identity_id, Some(3), 4, [0; 32], 1, 1)
                .expect("first future nonce"),
        );
        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 5, [0; 32], 1, 1)
                .expect("second future nonce"),
        );

        assert!(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, [0; 32], 1, 1)
                .is_none(),
            "alternating back to an earlier attempted nonce must not repeat proof work"
        );

        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(5), 6, [0; 32], 1, 1)
                .expect("new nonce after committed state advances"),
        );
        let attempts = verifier
            .identity_nonce_attempts
            .lock()
            .expect("attempts lock");
        assert_eq!(
            attempts
                .identities
                .get(&identity_id)
                .expect("identity attempts")
                .attempted_nonces,
            BTreeMap::from([(6, None)]),
            "nonces that committed state now rejects must be pruned"
        );
    }

    #[test]
    fn retains_attempted_missing_nonce_that_committed_state_still_accepts() {
        let verifier = CheckTxProofVerifier::new(3);
        let identity_id = [7; 32];

        drop(
            verifier
                .try_acquire_identity_nonce(identity_id, Some(3), 4, [0; 32], 1, 1)
                .expect("future nonce"),
        );

        let committed_with_four_missing = 5 | (1 << 40);
        assert!(
            verifier
                .try_acquire_identity_nonce(
                    identity_id,
                    Some(committed_with_four_missing),
                    4,
                    [0; 32],
                    1,
                    1
                )
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
                        verifier.try_acquire_identity_nonce([7; 32], Some(3), 4, [0; 32], 1, 1)
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
            .try_acquire_identity_nonce([10; 32], None, 1, [0; 32], 1, 1)
            .is_none());
        drop(occupied);
        assert!(
            verifier
                .try_acquire_identity_nonce([10; 32], None, 1, [0; 32], 1, 1)
                .is_some(),
            "capacity rejection must not consume the identity nonce attempt"
        );
    }
    #[test]
    fn should_reuse_only_exact_successful_proofs_without_capacity() {
        let verifier = CheckTxProofVerifier::new(1);
        let version = dpp::version::PlatformVersion::latest().protocol_version;
        let required = verifier
            .try_acquire_identity_nonce([1; 32], None, 1, [2; 32], 2, version)
            .unwrap();
        let IdentityProofVerification::Required(permit) = required else {
            panic!("first proof must verify")
        };
        permit.mark_verified();
        drop(permit);
        let occupied = verifier.try_acquire(2).unwrap();
        assert!(matches!(
            verifier.try_acquire_identity_nonce([1; 32], None, 1, [2; 32], 2, version),
            Some(IdentityProofVerification::Cached)
        ));
        assert!(
            verifier
                .try_acquire_identity_nonce([1; 32], None, 1, [3; 32], 2, version)
                .is_none(),
            "different bytes must not reuse success"
        );
        assert!(
            verifier
                .try_acquire_identity_nonce([1; 32], None, 1, [2; 32], 2, version + 1)
                .is_none(),
            "another protocol version must not reuse success"
        );
        assert!(
            verifier
                .try_acquire_identity_nonce([1; 32], Some(1), 1, [2; 32], 2, version)
                .is_none(),
            "committed nonce must still reject exact bytes"
        );
        drop(occupied);
    }

    #[test]
    fn should_admit_sequential_newcomers_while_proof_capacity_is_idle() {
        let mut verifier = CheckTxProofVerifier::new(1);
        verifier.identity_cache_limit = 2;
        let proof = VerifiedIdentityProof {
            transaction_hash: [2; 32],
            protocol_version: dpp::version::PlatformVersion::latest().protocol_version,
        };
        let now = Instant::now();
        // More than the former four-wave burst remains admissible because each
        // preceding proof permit has already released the actual CPU capacity.
        for id in 1..=10 {
            drop(
                verifier
                    .try_acquire_identity_nonce_at([id; 32], None, 1, proof, 1, now)
                    .expect("idle proof capacity must admit each newcomer"),
            );
        }
        assert_eq!(
            verifier
                .identity_nonce_attempts
                .lock()
                .unwrap()
                .identities
                .len(),
            2
        );
    }

    #[test]
    fn should_not_refresh_failed_entries_and_should_reuse_cached_successes() {
        let mut verifier = CheckTxProofVerifier::new(1);
        verifier.identity_cache_limit = 2;
        let proof = VerifiedIdentityProof {
            transaction_hash: [2; 32],
            protocol_version: dpp::version::PlatformVersion::latest().protocol_version,
        };
        let now = Instant::now();
        drop(
            verifier
                .try_acquire_identity_nonce_at([1; 32], None, 1, proof, 1, now)
                .unwrap(),
        );
        let IdentityProofVerification::Required(success) = verifier
            .try_acquire_identity_nonce_at(
                [2; 32],
                None,
                1,
                proof,
                1,
                now + Duration::from_millis(1),
            )
            .unwrap()
        else {
            panic!("first proof")
        };
        success.mark_verified();
        drop(success);
        assert!(verifier
            .try_acquire_identity_nonce_at(
                [1; 32],
                None,
                1,
                proof,
                1,
                now + Duration::from_millis(2)
            )
            .is_none());
        drop(
            verifier
                .try_acquire_identity_nonce_at(
                    [3; 32],
                    None,
                    1,
                    proof,
                    1,
                    now + Duration::from_millis(3),
                )
                .unwrap(),
        );
        assert!(!verifier
            .identity_nonce_attempts
            .lock()
            .unwrap()
            .identities
            .contains_key(&[1; 32]));
        drop(
            verifier
                .try_acquire_identity_nonce_at(
                    [3; 32],
                    None,
                    2,
                    proof,
                    1,
                    now + Duration::from_millis(4),
                )
                .unwrap(),
        );
        let occupied = verifier.try_acquire(1).unwrap();
        assert!(matches!(
            verifier.try_acquire_identity_nonce_at(
                [2; 32],
                None,
                1,
                proof,
                1,
                now + Duration::from_millis(5)
            ),
            Some(IdentityProofVerification::Cached)
        ));
        drop(occupied);
    }

    #[test]
    fn should_not_evict_in_flight_identity_when_proof_capacity_is_busy() {
        let mut verifier = CheckTxProofVerifier::new(2);
        verifier.identity_cache_limit = 1;
        let proof = VerifiedIdentityProof {
            transaction_hash: [2; 32],
            protocol_version: dpp::version::PlatformVersion::latest().protocol_version,
        };
        let now = Instant::now();
        let first = verifier
            .try_acquire_identity_nonce_at([1; 32], None, 1, proof, 1, now)
            .unwrap();
        assert!(
            verifier
                .try_acquire_identity_nonce_at([2; 32], None, 1, proof, 1, now)
                .is_none(),
            "active identity cannot be evicted"
        );
        drop(first);
        let occupied = verifier.try_acquire(4).unwrap();
        assert!(verifier
            .try_acquire_identity_nonce_at([2; 32], None, 1, proof, 1, now)
            .is_none());
        drop(occupied);
        assert!(
            verifier
                .try_acquire_identity_nonce_at([2; 32], None, 1, proof, 1, now)
                .is_some(),
            "admission must resume as soon as proof capacity is released"
        );
    }
    #[test]
    fn should_reverify_after_protocol_advance_without_allowing_version_oscillation() {
        let verifier = CheckTxProofVerifier::new(1);
        let version = dpp::version::PlatformVersion::latest().protocol_version;
        for active_version in [version, version + 1] {
            let Some(IdentityProofVerification::Required(permit)) =
                verifier.try_acquire_identity_nonce([1; 32], None, 1, [2; 32], 1, active_version)
            else {
                panic!("each new protocol must verify")
            };
            permit.mark_verified();
            drop(permit);
            assert!(matches!(
                verifier.try_acquire_identity_nonce([1; 32], None, 1, [2; 32], 1, active_version),
                Some(IdentityProofVerification::Cached)
            ));
        }
        assert!(verifier
            .try_acquire_identity_nonce([1; 32], None, 1, [2; 32], 1, version)
            .is_none());
    }
}
