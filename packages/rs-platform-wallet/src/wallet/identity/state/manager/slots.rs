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
use crate::changeset::{
    IdentitySlotLeaseChangeSet, IdentitySlotLeaseEntry, KeyWithBreadcrumb, PlatformWalletChangeSet,
};
use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
use crate::wallet::persister::WalletPersister;
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

/// What [`IdentityManager::place_in_slot`] restores when its store fails.
struct PlacementUndo {
    prior_location: Option<IdentityLocation>,
    prior: Option<ManagedIdentity>,
    lease: Option<SlotLease>,
    marker: bool,
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

    /// [`Self::acquire_identity_slot`] for a new registration: it gets a
    /// token no other registration lease of this manager has.
    pub fn acquire_registration_slot(
        &mut self,
        wallet_id: WalletId,
        funded_registrations: &BTreeSet<RegistrationIndex>,
    ) -> Result<(SlotHolder, SlotAcquisition), PlatformWalletError> {
        let holder = SlotHolder::Registration {
            token: self.next_registration_token,
        };
        let acquisition = self.acquire_identity_slot(wallet_id, holder, funded_registrations)?;
        self.next_registration_token = self.next_registration_token.wrapping_add(1);
        Ok((holder, acquisition))
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

    /// Put `lease` back on `wallet_id`'s slot `identity_index`, as it was
    /// before a release whose store failed.
    pub(crate) fn restore_slot_lease(
        &mut self,
        wallet_id: WalletId,
        identity_index: RegistrationIndex,
        lease: SlotLease,
    ) {
        self.slot_leases
            .entry(wallet_id)
            .or_default()
            .insert(identity_index, lease);
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

    /// [`Self::place_in_slot_unstored`], with `keys` layered onto the
    /// identity, stored durably as one changeset: the identity, `keys` with
    /// their breadcrumbs and, when `releasing` is a claim, its lease as it now
    /// stands. Memory keeps the placement only once the store is durable; on a
    /// failed store it is undone and the store's error returned, so no caller
    /// reports an identity as placed that a restart would not find.
    pub(crate) fn place_in_slot(
        &mut self,
        identity: Identity,
        identity_index: RegistrationIndex,
        wallet_id: WalletId,
        releasing: Option<&SlotHolder>,
        keys: Vec<KeyWithBreadcrumb>,
        persister: &WalletPersister,
    ) -> Result<SlotPlacement, PlatformWalletError> {
        let identity_id = identity.id();
        let undo = PlacementUndo {
            prior_location: self.location_index().get(&identity_id).copied(),
            prior: self.managed_identity(&identity_id).cloned(),
            lease: self
                .slot_leases
                .get(&wallet_id)
                .and_then(|leases| leases.get(&identity_index))
                .copied(),
            marker: self.retained_private_payment_markers.contains(&identity_id),
        };
        let placement =
            self.place_in_slot_unstored(identity, identity_index, wallet_id, releasing)?;

        let managed = self
            .managed_identity_mut(&identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?;
        let keys_cs = managed.layer_keys(keys);
        let mut changeset = PlatformWalletChangeSet {
            identities: Some(managed.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        if let Some(SlotHolder::InvitationClaim { invitation }) = releasing {
            changeset.identity_slot_leases =
                Some(self.claim_lease_changeset(&wallet_id, *invitation));
        }

        if let Err(not_durable) = persister.store_durably(changeset) {
            self.undo_placement(identity_id, wallet_id, identity_index, undo);
            return Err(persister.classify_not_durable(not_durable));
        }
        Ok(placement)
    }

    /// Put memory back as it was before a [`Self::place_in_slot`] whose
    /// store failed.
    fn undo_placement(
        &mut self,
        identity_id: Identifier,
        wallet_id: WalletId,
        identity_index: RegistrationIndex,
        undo: PlacementUndo,
    ) {
        if let Some(slots) = self.wallet_identities.get_mut(&wallet_id) {
            slots.remove(&identity_index);
            if slots.is_empty() {
                self.wallet_identities.remove(&wallet_id);
            }
        }
        self.location_index_remove(&identity_id);
        if let (Some(prior), Some(location)) = (undo.prior, undo.prior_location) {
            match location {
                IdentityLocation::InWallet {
                    wallet_id,
                    registration_index,
                } => {
                    self.wallet_identities
                        .entry(wallet_id)
                        .or_default()
                        .insert(registration_index, prior);
                }
                IdentityLocation::OutOfWallet => {
                    self.out_of_wallet_identities.insert(identity_id, prior);
                }
            }
            self.location_index_insert(identity_id, location);
        }
        if let Some(lease) = undo.lease {
            self.slot_leases
                .entry(wallet_id)
                .or_default()
                .insert(identity_index, lease);
        }
        if undo.marker {
            self.retained_private_payment_markers.insert(identity_id);
        }
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

    /// The stored form of `invitation`'s claim lease as it is now: its entry,
    /// or its release when it leases nothing.
    pub(crate) fn claim_lease_changeset(
        &self,
        wallet_id: &WalletId,
        invitation: OutPoint,
    ) -> IdentitySlotLeaseChangeSet {
        let holder = SlotHolder::InvitationClaim { invitation };
        let mut changeset = IdentitySlotLeaseChangeSet::default();
        match self.leased_slot(wallet_id, &holder) {
            Some(slot) => {
                changeset.leases.insert(
                    invitation,
                    IdentitySlotLeaseEntry {
                        identity_index: slot.identity_index,
                        phase: slot.phase,
                    },
                );
            }
            None => {
                changeset.released.insert(invitation);
            }
        }
        changeset
    }

    /// Replay stored claim leases onto `wallet_id`. An entry moves its claim
    /// to the entry's index; a release drops it. Idempotent.
    pub(crate) fn apply_claim_lease_changeset(
        &mut self,
        wallet_id: WalletId,
        changeset: IdentitySlotLeaseChangeSet,
    ) {
        for (invitation, entry) in changeset.leases {
            let holder = SlotHolder::InvitationClaim { invitation };
            self.release_identity_slot(&wallet_id, &holder);
            let leases = self.slot_leases.entry(wallet_id).or_default();
            if let Some(previous) = leases.get(&entry.identity_index) {
                if previous.holder != holder {
                    tracing::warn!(
                        identity_index = entry.identity_index,
                        "two stored identity slot leases name one index; keeping the later one"
                    );
                }
            }
            leases.insert(
                entry.identity_index,
                SlotLease {
                    holder,
                    phase: entry.phase,
                },
            );
        }
        for invitation in changeset.released {
            self.release_identity_slot(&wallet_id, &SlotHolder::InvitationClaim { invitation });
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

    /// A backend whose store commits inline, recording each changeset, or
    /// failing every store when `fail` is set.
    #[derive(Default)]
    struct Recording {
        fail: bool,
        stored: std::sync::Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl crate::changeset::PlatformWalletPersistence for Recording {
        fn store_commits_inline(&self) -> bool {
            true
        }
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), crate::changeset::PersistenceError> {
            if self.fail {
                return Err(crate::changeset::PersistenceError::backend(
                    "the host refused the write",
                ));
            }
            self.stored.lock().unwrap().push(changeset);
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), crate::changeset::PersistenceError> {
            Ok(())
        }
        fn load(
            &self,
        ) -> Result<crate::changeset::ClientStartState, crate::changeset::PersistenceError>
        {
            Ok(Default::default())
        }
    }

    fn persister_of(backend: std::sync::Arc<Recording>) -> WalletPersister {
        WalletPersister::new(WALLET, backend)
    }

    fn identity_with_key(byte: u8) -> Identity {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
        let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: vec![byte; 20].into(),
            disabled_at: None,
        });
        Identity::V0(IdentityV0 {
            id: Identifier::from([byte; 32]),
            public_keys: BTreeMap::from([(0, key)]),
            balance: 0,
            revision: 0,
        })
    }

    fn keys_of(identity: &Identity, index: RegistrationIndex) -> Vec<KeyWithBreadcrumb> {
        identity
            .public_keys()
            .iter()
            .map(|(key_id, key)| KeyWithBreadcrumb {
                key: key.clone(),
                breadcrumb: Some((WALLET, index, *key_id)),
            })
            .collect()
    }

    #[test]
    fn should_store_the_identity_its_keys_and_the_claim_release_in_one_write() {
        let backend = std::sync::Arc::new(Recording::default());
        let persister = persister_of(backend.clone());
        let mut manager = IdentityManager::new();
        let acquired = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        let landed = identity_with_key(1);
        let keys = keys_of(&landed, acquired.identity_index);

        manager
            .place_in_slot(
                landed,
                acquired.identity_index,
                WALLET,
                Some(&claim(1)),
                keys,
                &persister,
            )
            .unwrap();

        let stored = backend.stored.lock().unwrap();
        assert_eq!(stored.len(), 1);
        let write = &stored[0];
        let id = Identifier::from([1u8; 32]);
        assert!(write
            .identities
            .as_ref()
            .unwrap()
            .identities
            .contains_key(&id));
        let key_row = &write.identity_keys.as_ref().unwrap().upserts[&(id, 0)];
        assert_eq!(key_row.wallet_id, Some(WALLET));
        assert_eq!(
            key_row.derivation_indices.as_ref().unwrap().identity_index,
            acquired.identity_index
        );
        let SlotHolder::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        assert_eq!(
            write.identity_slot_leases.as_ref().unwrap().released,
            [invitation].into()
        );
    }

    #[test]
    fn should_publish_nothing_when_the_placement_is_not_stored() {
        let persister = persister_of(std::sync::Arc::new(Recording {
            fail: true,
            ..Default::default()
        }));
        let mut manager = IdentityManager::new();
        let id = Identifier::from([1u8; 32]);
        manager.retain_private_payment_markers([id]);
        let acquired = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        let landed = identity_with_key(1);
        let keys = keys_of(&landed, acquired.identity_index);

        let placed = manager.place_in_slot(
            landed,
            acquired.identity_index,
            WALLET,
            Some(&claim(1)),
            keys,
            &persister,
        );

        assert!(placed.is_err());
        assert!(manager.identity(&id).is_none());
        assert_eq!(manager.identity_index(&id), None);
        // The claim still holds its slot, and the hold still waits for the
        // identity, so a retry finds both as they were.
        assert_eq!(
            manager
                .leased_slot(&WALLET, &claim(1))
                .map(|slot| slot.identity_index),
            Some(acquired.identity_index)
        );
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET, None)
            .unwrap();
        assert!(manager
            .managed_identity(&id)
            .unwrap()
            .dashpay()
            .private_payments_unrestored());
    }

    #[test]
    fn should_leave_an_observed_identity_observed_when_its_placement_is_not_stored() {
        let persister = persister_of(std::sync::Arc::new(Recording {
            fail: true,
            ..Default::default()
        }));
        let mut manager = IdentityManager::new();
        let observer = WalletPersister::new(
            WALLET,
            std::sync::Arc::new(crate::wallet::persister::NoPlatformPersistence),
        );
        manager
            .add_out_of_wallet_identity(identity(1), &observer)
            .unwrap();
        let landed = identity_with_key(1);
        let keys = keys_of(&landed, 2);

        let placed = manager.place_in_slot(landed, 2, WALLET, None, keys, &persister);

        assert!(placed.is_err());
        let id = Identifier::from([1u8; 32]);
        assert!(manager.out_of_wallet_identities.contains_key(&id));
        assert_eq!(manager.identity_index(&id), None);
        assert_eq!(manager.slot_state(&WALLET, 2), SlotState::Free);
        assert!(manager
            .managed_identity(&id)
            .unwrap()
            .identity
            .public_keys()
            .is_empty());
    }

    #[test]
    fn should_restore_a_stored_claim_lease_after_a_restart() {
        let mut before = IdentityManager::new();
        before
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 1 }, &none())
            .unwrap();
        before
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        before.set_slot_lease_phase(&WALLET, &claim(1), SlotLeasePhase::Submitted);
        let SlotHolder::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        let stored = before.claim_lease_changeset(&WALLET, invitation);

        let after = IdentityManager::from(crate::changeset::IdentityManagerStartState {
            claim_leases: [(WALLET, stored.leases)].into(),
            ..Default::default()
        });

        // The registration's lease was in memory only; the claim's came back
        // at its index and phase, and a new registration skips it.
        let restored = after.leased_slot(&WALLET, &claim(1)).unwrap();
        assert_eq!(restored.identity_index, 1);
        assert_eq!(restored.phase, SlotLeasePhase::Submitted);
        let mut after = after;
        let next = after
            .acquire_identity_slot(WALLET, SlotHolder::Registration { token: 2 }, &none())
            .unwrap();
        assert_eq!(next.identity_index, 0);
    }

    #[test]
    fn should_store_a_release_for_a_claim_that_leases_nothing() {
        let mut manager = IdentityManager::new();
        manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        manager.release_identity_slot(&WALLET, &claim(1));
        let SlotHolder::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };

        let stored = manager.claim_lease_changeset(&WALLET, invitation);

        assert!(stored.leases.is_empty());
        assert_eq!(stored.released, [invitation].into());
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
