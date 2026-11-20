//! Durable invitation-claim slot leases.
//!
//! An invitation claim derives its identity keys at an identity index of the
//! wallet. Its lease on that index
//! ([`IdentityManager::acquire_identity_slot`](crate::wallet::identity::IdentityManager::acquire_identity_slot))
//! is stored before any key is derived, so a claim retried after an app exit
//! returns to the slot whose keys its earlier attempt may have landed, and a
//! normal registration never takes it. A normal registration's lease is not
//! stored: until its asset lock is built nothing is spent, and from then on the
//! tracked asset lock holds the index.
//!
//! Keyed by the invitation's asset-lock outpoint, which also fixes the identity
//! id the claim lands.

use std::collections::{BTreeMap, BTreeSet};

use dashcore::OutPoint;

use super::Merge;
use crate::wallet::identity::SlotLeasePhase;

/// One stored claim lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentitySlotLeaseEntry {
    pub identity_index: u32,
    pub phase: SlotLeasePhase,
}

/// Claim leases taken, updated or released in one round. An outpoint is never
/// in both maps: the later write wins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct IdentitySlotLeaseChangeSet {
    pub leases: BTreeMap<OutPoint, IdentitySlotLeaseEntry>,
    pub released: BTreeSet<OutPoint>,
}

impl Merge for IdentitySlotLeaseChangeSet {
    fn merge(&mut self, other: Self) {
        for (invitation, entry) in other.leases {
            self.released.remove(&invitation);
            self.leases.insert(invitation, entry);
        }
        for invitation in other.released {
            self.leases.remove(&invitation);
            self.released.insert(invitation);
        }
    }

    fn is_empty(&self) -> bool {
        self.leases.is_empty() && self.released.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::Txid;

    fn invitation(byte: u8) -> OutPoint {
        OutPoint::new(Txid::from_byte_array([byte; 32]), 0)
    }

    fn entry(identity_index: u32, phase: SlotLeasePhase) -> IdentitySlotLeaseEntry {
        IdentitySlotLeaseEntry {
            identity_index,
            phase,
        }
    }

    #[test]
    fn should_let_a_later_release_cancel_an_earlier_lease() {
        let mut round = IdentitySlotLeaseChangeSet::default();
        round
            .leases
            .insert(invitation(1), entry(2, SlotLeasePhase::Reserved));

        round.merge(IdentitySlotLeaseChangeSet {
            released: [invitation(1)].into(),
            ..Default::default()
        });

        assert!(round.leases.is_empty());
        assert_eq!(round.released, [invitation(1)].into());
    }

    #[test]
    fn should_let_a_later_lease_cancel_an_earlier_release() {
        let mut round = IdentitySlotLeaseChangeSet {
            released: [invitation(1)].into(),
            ..Default::default()
        };

        round.merge(IdentitySlotLeaseChangeSet {
            leases: [(invitation(1), entry(4, SlotLeasePhase::Submitted))].into(),
            ..Default::default()
        });

        assert!(round.released.is_empty());
        assert_eq!(
            round.leases.get(&invitation(1)),
            Some(&entry(4, SlotLeasePhase::Submitted))
        );
    }
}
