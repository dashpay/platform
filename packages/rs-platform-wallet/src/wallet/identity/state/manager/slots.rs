//! Identity slots: which `(wallet_id, identity_index)` an operation may derive
//! identity keys at.
//!
//! Two flows derive keys at an index — a normal registration and an invitation
//! claim — and the same index yields the same keys, so Platform refuses the
//! second one (duplicate key hashes) after its funding is spent. A flow takes
//! a lease on its slot before any key is derived; [`IdentityManager`] chooses
//! the index, so hosts never do.
//!
//! A slot is in one of three states ([`SlotState`]): it holds a wallet-owned
//! identity, a holder leases it, or it is free. An unconsumed
//! `IdentityRegistration` asset lock also holds its index; the manager does
//! not track asset locks, so the caller passes those indices to
//! [`IdentityManager::acquire_identity_slot`].

use super::{IdentityLocation, IdentityManager, RegistrationIndex};
use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
use crate::wallet::platform_wallet::WalletId;
use dashcore::OutPoint;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use std::collections::BTreeSet;

/// Who holds a leased slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SlotHolder {
    /// A normal registration, until it lands or is abandoned. `token` is
    /// unique per acquisition, so two registrations never share a slot.
    Registration { token: u64 },
    /// An invitation claim, keyed by the invitation's asset-lock outpoint. A
    /// retry of the same claim re-enters this lease, so it derives the keys
    /// at the slot its earlier attempt may already have landed.
    InvitationClaim { invitation: OutPoint },
}

/// How far the holder of a lease got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SlotLeasePhase {
    /// Keys may be derived, nothing was sent: releasing the lease is safe.
    Reserved,
    /// A transition using the slot's keys may have reached Platform: the
    /// lease is kept until its outcome is known.
    Submitted,
}

/// A lease on one slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotLease {
    pub holder: SlotHolder,
    pub phase: SlotLeasePhase,
}

/// What a slot holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotState {
    Free,
    Leased(SlotLease),
    Occupied(Identifier),
}

/// Result of [`IdentityManager::acquire_identity_slot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotAcquisition {
    pub identity_index: RegistrationIndex,
    /// The holder already leased this slot (a claim retried), and the lease
    /// keeps the phase it had.
    pub reentered: bool,
    pub phase: SlotLeasePhase,
}

/// Result of [`IdentityManager::place_in_slot_unstored`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotPlacement {
    /// The identity now holds the slot.
    Placed,
    /// The identity already held the slot; nothing changed.
    AlreadyPlaced,
}

impl IdentityManager {
    /// What `wallet_id`'s slot `identity_index` holds. An identity wins over a
    /// lease: a lease whose slot is occupied is stale.
    pub fn slot_state(&self, wallet_id: &WalletId, identity_index: RegistrationIndex) -> SlotState {
        if let Some(occupant) = self
            .wallet_identities
            .get(wallet_id)
            .and_then(|slots| slots.get(&identity_index))
        {
            return SlotState::Occupied(occupant.identity.id());
        }
        match self
            .slot_leases
            .get(wallet_id)
            .and_then(|leases| leases.get(&identity_index))
        {
            Some(lease) => SlotState::Leased(*lease),
            None => SlotState::Free,
        }
    }

    /// The slot `holder` leases in `wallet_id`, if any.
    pub fn leased_slot(
        &self,
        wallet_id: &WalletId,
        holder: &SlotHolder,
    ) -> Option<SlotAcquisition> {
        self.slot_leases
            .get(wallet_id)?
            .iter()
            .find_map(|(index, lease)| {
                (lease.holder == *holder).then_some(SlotAcquisition {
                    identity_index: *index,
                    reentered: true,
                    phase: lease.phase,
                })
            })
    }

    /// Lease a slot of `wallet_id` for `holder`, in memory only.
    ///
    /// A holder that already leases a slot gets it back (`reentered`).
    /// Otherwise the lowest index that holds no identity, no lease and no
    /// index in `funded_registrations` (the indices of unconsumed
    /// `IdentityRegistration` asset locks) is leased in the `Reserved` phase.
    /// The lowest one rather than one past the highest, so an abandoned
    /// registration leaves no hole the identity discovery gap limit could
    /// stop at; an on-chain identity this wallet does not know at that index
    /// is the caller's pre-funding check to catch.
    pub fn acquire_identity_slot(
        &mut self,
        wallet_id: WalletId,
        holder: SlotHolder,
        funded_registrations: &BTreeSet<RegistrationIndex>,
    ) -> Result<SlotAcquisition, PlatformWalletError> {
        if let Some(existing) = self.leased_slot(&wallet_id, &holder) {
            return Ok(existing);
        }
        let identity_index = (0..=RegistrationIndex::MAX)
            .find(|index| {
                !funded_registrations.contains(index)
                    && self.slot_state(&wallet_id, *index) == SlotState::Free
            })
            .ok_or_else(|| {
                PlatformWalletError::InvalidIdentityData(
                    "every identity index of this wallet is taken".to_string(),
                )
            })?;
        self.slot_leases.entry(wallet_id).or_default().insert(
            identity_index,
            SlotLease {
                holder,
                phase: SlotLeasePhase::Reserved,
            },
        );
        Ok(SlotAcquisition {
            identity_index,
            reentered: false,
            phase: SlotLeasePhase::Reserved,
        })
    }

    /// Move `holder`'s lease to `phase`. Returns the leased index, or `None`
    /// when `holder` leases nothing in `wallet_id`.
    pub fn set_slot_lease_phase(
        &mut self,
        wallet_id: &WalletId,
        holder: &SlotHolder,
        phase: SlotLeasePhase,
    ) -> Option<RegistrationIndex> {
        let leases = self.slot_leases.get_mut(wallet_id)?;
        let (index, lease) = leases
            .iter_mut()
            .find(|(_, lease)| lease.holder == *holder)?;
        lease.phase = phase;
        Some(*index)
    }

    /// Drop `holder`'s lease in `wallet_id`, in memory only. Returns the index
    /// it leased.
    pub fn release_identity_slot(
        &mut self,
        wallet_id: &WalletId,
        holder: &SlotHolder,
    ) -> Option<RegistrationIndex> {
        let leases = self.slot_leases.get_mut(wallet_id)?;
        let index = leases
            .iter()
            .find_map(|(index, lease)| (lease.holder == *holder).then_some(*index))?;
        leases.remove(&index);
        if leases.is_empty() {
            self.slot_leases.remove(wallet_id);
        }
        Some(index)
    }

    /// Put a wallet-owned identity into `wallet_id`'s slot `identity_index`,
    /// in memory only: the caller stores it. The one path every add of a
    /// wallet-owned identity takes, so they agree on occupancy and holds.
    ///
    /// - The slot holds this identity already: [`SlotPlacement::AlreadyPlaced`].
    /// - The slot holds another identity: refused with
    ///   [`PlatformWalletError::IdentitySlotOccupied`]; nothing changes.
    /// - The identity is held elsewhere in the wallet (another slot): refused
    ///   with [`PlatformWalletError::IdentityAlreadyExists`].
    /// - The identity is observed (out-of-wallet): it moves into the slot,
    ///   keeping its state.
    /// - Otherwise it is inserted.
    ///
    /// A lease on the slot does not stop the placement: an identity whose keys
    /// are derived at this index is what the slot is for, whoever leased it.
    /// `releasing`, when given, is dropped if it is the slot's lease holder.
    /// A retained private-payment hold is applied whichever way it arrives.
    pub(crate) fn place_in_slot_unstored(
        &mut self,
        identity: Identity,
        identity_index: RegistrationIndex,
        wallet_id: WalletId,
        releasing: Option<&SlotHolder>,
    ) -> Result<SlotPlacement, PlatformWalletError> {
        let identity_id = identity.id();
        match self.slot_state(&wallet_id, identity_index) {
            SlotState::Occupied(occupant) if occupant == identity_id => {
                self.release_matching_lease(&wallet_id, identity_index, releasing);
                return Ok(SlotPlacement::AlreadyPlaced);
            }
            SlotState::Occupied(occupant) => {
                return Err(PlatformWalletError::IdentitySlotOccupied {
                    identity_index,
                    occupant,
                });
            }
            SlotState::Leased(_) | SlotState::Free => {}
        }

        let mut managed = match self.location_index().get(&identity_id).copied() {
            Some(IdentityLocation::InWallet { .. }) => {
                return Err(PlatformWalletError::IdentityAlreadyExists(identity_id));
            }
            Some(IdentityLocation::OutOfWallet) => self
                .out_of_wallet_identities
                .remove(&identity_id)
                .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?,
            None => ManagedIdentity::new(identity, identity_index),
        };
        managed.identity_index = Some(identity_index);
        // Denormalize the bucket keys onto the value so callers that iterate
        // values know which wallet + slot they came from.
        managed.wallet_id = Some(wallet_id);
        // Held before anything can send or accept for it, when a persister
        // kept its record marked.
        if self.retained_private_payment_markers.remove(&identity_id) {
            managed.hold_for_retained_private_payment_marker();
        }

        self.wallet_identities
            .entry(wallet_id)
            .or_default()
            .insert(identity_index, managed);
        self.location_index_insert(
            identity_id,
            IdentityLocation::InWallet {
                wallet_id,
                registration_index: identity_index,
            },
        );
        self.release_matching_lease(&wallet_id, identity_index, releasing);
        Ok(SlotPlacement::Placed)
    }

    fn release_matching_lease(
        &mut self,
        wallet_id: &WalletId,
        identity_index: RegistrationIndex,
        releasing: Option<&SlotHolder>,
    ) {
        let Some(releasing) = releasing else {
            return;
        };
        let Some(leases) = self.slot_leases.get_mut(wallet_id) else {
            return;
        };
        if leases.get(&identity_index).map(|lease| &lease.holder) == Some(releasing) {
            leases.remove(&identity_index);
            if leases.is_empty() {
                self.slot_leases.remove(wallet_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::Txid;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    const WALLET: WalletId = [7u8; 32];

    fn identity(byte: u8) -> Identity {
        Identity::V0(IdentityV0 {
            id: Identifier::from([byte; 32]),
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        })
    }

    fn claim(byte: u8) -> SlotHolder {
        SlotHolder::InvitationClaim {
            invitation: OutPoint::new(Txid::from_byte_array([byte; 32]), 0),
        }
    }

    fn none() -> BTreeSet<RegistrationIndex> {
        BTreeSet::new()
    }

    #[test]
    fn should_lease_the_lowest_free_index() {
        let mut manager = IdentityManager::new();
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();
        manager
            .place_in_slot_unstored(identity(2), 2, WALLET, None)
            .unwrap();

        let first = manager
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 1 }, &none())
            .unwrap();
        let second = manager
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 2 }, &none())
            .unwrap();

        assert_eq!(first.identity_index, 1);
        assert_eq!(second.identity_index, 3);
        assert!(!first.reentered);
    }

    #[test]
    fn should_skip_indices_of_unconsumed_registration_asset_locks() {
        let mut manager = IdentityManager::new();
        let funded = BTreeSet::from([0, 1]);

        let acquired = manager
            .acquire_identity_slot(WALLET, claim(1), &funded)
            .unwrap();

        assert_eq!(acquired.identity_index, 2);
    }

    #[test]
    fn should_return_a_claim_to_its_own_slot_and_keep_its_phase() {
        let mut manager = IdentityManager::new();
        let first = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        manager.set_slot_lease_phase(&WALLET, &claim(1), SlotLeasePhase::Submitted);
        manager
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 9 }, &none())
            .unwrap();

        let retried = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();

        assert_eq!(retried.identity_index, first.identity_index);
        assert!(retried.reentered);
        assert_eq!(retried.phase, SlotLeasePhase::Submitted);
    }

    #[test]
    fn should_keep_a_registration_off_a_claim_held_slot() {
        let mut manager = IdentityManager::new();
        let claimed = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();

        let registered = manager
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 1 }, &none())
            .unwrap();

        assert_ne!(registered.identity_index, claimed.identity_index);
    }

    #[test]
    fn should_reuse_a_released_slot() {
        let mut manager = IdentityManager::new();
        let holder = SlotHolder::Registration { token: 1 };
        manager
            .acquire_identity_slot(WALLET, holder, &none())
            .unwrap();

        assert_eq!(manager.release_identity_slot(&WALLET, &holder), Some(0));
        let next = manager
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 2 }, &none())
            .unwrap();

        assert_eq!(next.identity_index, 0);
        assert_eq!(
            manager.slot_state(&WALLET, 0),
            SlotState::Leased(SlotLease {
                holder: SlotHolder::Registration { token: 2 },
                phase: SlotLeasePhase::Reserved,
            })
        );
    }

    #[test]
    fn should_scope_slots_per_wallet() {
        let mut manager = IdentityManager::new();
        manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();

        let other = manager
            .acquire_identity_slot([8u8; 32], claim(1), &none())
            .unwrap();

        assert_eq!(other.identity_index, 0);
        assert!(!other.reentered);
    }

    #[test]
    fn should_refuse_to_place_into_a_slot_another_identity_holds() {
        let mut manager = IdentityManager::new();
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();

        let refused = manager.place_in_slot_unstored(identity(2), 0, WALLET, None);

        assert!(matches!(
            refused,
            Err(PlatformWalletError::IdentitySlotOccupied { identity_index: 0, occupant })
                if occupant == Identifier::from([1u8; 32])
        ));
        assert_eq!(
            manager.slot_state(&WALLET, 0),
            SlotState::Occupied(Identifier::from([1u8; 32]))
        );
        assert!(manager.identity(&Identifier::from([2u8; 32])).is_none());
    }

    #[test]
    fn should_treat_placing_the_same_identity_again_as_done() {
        let mut manager = IdentityManager::new();
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();

        let again = manager.place_in_slot_unstored(identity(1), 0, WALLET, None);

        assert_eq!(again.unwrap(), SlotPlacement::AlreadyPlaced);
    }

    #[test]
    fn should_refuse_an_identity_the_wallet_holds_in_another_slot() {
        let mut manager = IdentityManager::new();
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();

        let refused = manager.place_in_slot_unstored(identity(1), 1, WALLET, None);

        assert!(matches!(
            refused,
            Err(PlatformWalletError::IdentityAlreadyExists(_))
        ));
        assert_eq!(manager.slot_state(&WALLET, 1), SlotState::Free);
    }

    #[test]
    fn should_release_the_landing_holders_lease_when_placing() {
        let mut manager = IdentityManager::new();
        let acquired = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();

        manager
            .place_in_slot_unstored(
                identity(1),
                acquired.identity_index,
                WALLET,
                Some(&claim(1)),
            )
            .unwrap();

        assert!(manager.leased_slot(&WALLET, &claim(1)).is_none());
        assert_eq!(
            manager.slot_state(&WALLET, acquired.identity_index),
            SlotState::Occupied(Identifier::from([1u8; 32]))
        );
    }

    #[test]
    fn should_place_into_a_slot_another_holder_leases_and_keep_that_lease() {
        let mut manager = IdentityManager::new();
        manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();

        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();

        // The slot is the identity's now; the claim's lease is stale but kept,
        // so its holder learns of the conflict instead of moving silently.
        assert_eq!(
            manager.slot_state(&WALLET, 0),
            SlotState::Occupied(Identifier::from([1u8; 32]))
        );
        assert!(manager.leased_slot(&WALLET, &claim(1)).is_some());
        let next = manager
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 1 }, &none())
            .unwrap();
        assert_eq!(next.identity_index, 1);
    }

    #[test]
    fn should_move_an_observed_identity_into_the_slot() {
        let mut manager = IdentityManager::new();
        let persister = crate::wallet::persister::WalletPersister::new(
            WALLET,
            std::sync::Arc::new(crate::wallet::persister::NoPlatformPersistence),
        );
        manager
            .add_out_of_wallet_identity(identity(1), &persister)
            .unwrap();

        manager
            .place_in_slot_unstored(identity(1), 3, WALLET, None)
            .unwrap();

        let id = Identifier::from([1u8; 32]);
        assert_eq!(manager.identity_index(&id), Some(3));
        assert!(!manager.out_of_wallet_identities.contains_key(&id));
        assert_eq!(manager.slot_state(&WALLET, 3), SlotState::Occupied(id));
    }

    #[test]
    fn should_apply_a_retained_private_payment_hold_on_placement() {
        let mut manager = IdentityManager::new();
        let id = Identifier::from([1u8; 32]);
        manager.retain_private_payment_markers([id]);

        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();

        assert!(manager
            .managed_identity(&id)
            .unwrap()
            .dashpay()
            .private_payments_unrestored());
    }
}
