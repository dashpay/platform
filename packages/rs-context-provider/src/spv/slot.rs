use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use arc_swap::ArcSwapOption;

use crate::{ContextProvider, ContextProviderError};

/// Whether a published SPV client is synced enough to answer quorum lookups.
pub type Readiness = Arc<dyn Fn() -> Result<bool, ContextProviderError> + Send + Sync>;

/// A published SPV client: its quorum lookups and whether it is synced.
#[derive(Clone)]
pub(crate) struct SpvContextSource {
    pub(crate) provider: Arc<dyn ContextProvider>,
    is_ready: Readiness,
}

/// One lease's hold on the slot; `source` is set once the holder publishes.
struct SlotClaim {
    lease_id: u64,
    source: Option<SpvContextSource>,
}

/// The slot an SPV client publishes its quorum keys into.
///
/// Only one lease holds the slot at a time, so two SPV clients can never both
/// answer for the same SDK.
pub struct SpvSourceSlot {
    claim: ArcSwapOption<SlotClaim>,
    next_lease_id: AtomicU64,
}

impl Default for SpvSourceSlot {
    fn default() -> Self {
        Self {
            claim: ArcSwapOption::empty(),
            next_lease_id: AtomicU64::new(1),
        }
    }
}

/// Returned by [`SpvSourceSlot::claim`] when another lease holds the
/// slot.
#[derive(Debug, thiserror::Error)]
#[error("another SPV client already serves this SDK's quorum keys")]
pub struct SpvSourceSlotTaken;

impl SpvSourceSlot {
    /// Claim the slot. Only an empty slot can be claimed; the lease empties it
    /// again when dropped.
    pub fn claim(self: &Arc<Self>) -> Result<SpvSourceLease, SpvSourceSlotTaken> {
        let lease_id = self.next_lease_id.fetch_add(1, Ordering::Relaxed);
        let claim = Arc::new(SlotClaim {
            lease_id,
            source: None,
        });
        let previous = self
            .claim
            .compare_and_swap(std::ptr::null::<SlotClaim>(), Some(claim));
        if previous.is_some() {
            return Err(SpvSourceSlotTaken);
        }
        Ok(SpvSourceLease {
            slot: Arc::downgrade(self),
            lease_id,
        })
    }

    fn publish(&self, lease_id: u64, source: SpvContextSource) {
        self.claim.rcu(|current| match current {
            Some(claim) if claim.lease_id == lease_id => Some(Arc::new(SlotClaim {
                lease_id,
                source: Some(source.clone()),
            })),
            _ => current.clone(),
        });
    }

    fn release(&self, lease_id: u64) {
        self.claim.rcu(|current| match current {
            Some(claim) if claim.lease_id == lease_id => None,
            _ => current.clone(),
        });
    }

    /// The published source, if it is synced.
    ///
    /// Every "no usable source" outcome, an unknown readiness included, is
    /// [`ContextProviderError::QuorumSourceUnavailable`]: it is this side's
    /// state, so the SDK must not ban the node whose proof it could not check.
    pub(crate) fn ready_source(&self) -> Result<SpvContextSource, ContextProviderError> {
        let source = self
            .claim
            .load_full()
            .and_then(|claim| claim.source.clone())
            .ok_or_else(|| {
                ContextProviderError::QuorumSourceUnavailable(
                    "SPV context source is not active".to_string(),
                )
            })?;
        match (source.is_ready)() {
            Ok(true) => Ok(source),
            Ok(false) => Err(ContextProviderError::QuorumSourceUnavailable(
                "SPV context source is not ready".to_string(),
            )),
            Err(error) => Err(ContextProviderError::QuorumSourceUnavailable(format!(
                "SPV context source readiness is unknown: {error}"
            ))),
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.ready_source().is_ok()
    }
}

/// A claim on an [`SpvSourceSlot`] slot. Dropping it empties the slot,
/// unless a later lease holds it by then.
pub struct SpvSourceLease {
    slot: Weak<SpvSourceSlot>,
    lease_id: u64,
}

impl SpvSourceLease {
    /// Serve `provider`'s quorum keys while `is_ready` reports it synced.
    pub fn publish(&self, provider: Arc<dyn ContextProvider>, is_ready: Readiness) {
        if let Some(slot) = self.slot.upgrade() {
            slot.publish(self.lease_id, SpvContextSource { provider, is_ready });
        }
    }
}

impl Drop for SpvSourceLease {
    fn drop(&mut self) {
        if let Some(slot) = self.slot.upgrade() {
            slot.release(self.lease_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::SpvSourceSlot;
    use crate::test_support::{publish, quorums};

    #[test]
    fn should_let_one_lease_hold_the_slot_until_it_is_dropped() {
        let slot = Arc::new(SpvSourceSlot::default());
        let lease = publish(&slot, quorums("spv", 0x22), true);
        assert!(slot.claim().is_err());

        drop(lease);
        let _replacement = publish(&slot, quorums("other", 0x33), true);
        assert_eq!(
            slot.ready_source()
                .unwrap()
                .provider
                .get_quorum_public_key(1, [2; 32], 3)
                .unwrap(),
            [0x33; 48]
        );
    }

    /// The empty-slot swap is the only thing keeping two SPV clients from
    /// answering for one SDK, so concurrent claims must yield exactly one
    /// lease.
    #[test]
    fn should_grant_exactly_one_of_concurrent_claims() {
        let slot = Arc::new(SpvSourceSlot::default());
        let barrier = Arc::new(std::sync::Barrier::new(8));
        let claims: Vec<_> = (0..8)
            .map(|_| {
                let slot = Arc::clone(&slot);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    slot.claim().ok()
                })
            })
            .collect();
        let leases: Vec<_> = claims
            .into_iter()
            .filter_map(|claim| claim.join().expect("claim thread"))
            .collect();
        assert_eq!(leases.len(), 1);
    }

    /// An SDK may be destroyed before the wallet manager holding its slot's
    /// lease; publishing through or dropping that lease must then do nothing.
    #[test]
    fn should_ignore_a_lease_whose_slot_is_gone() {
        let slot = Arc::new(SpvSourceSlot::default());
        let lease = slot.claim().expect("empty slot");
        drop(slot);

        lease.publish(quorums("spv", 0x22), Arc::new(|| Ok(true)));
        drop(lease);
    }
}
