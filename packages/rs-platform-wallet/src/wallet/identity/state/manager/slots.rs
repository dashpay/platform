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

/// An invitation claim's lease ([`IdentityManager::claim_leases`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClaimLease {
    pub(crate) identity_index: RegistrationIndex,
    pub(crate) invitation: OutPoint,
    pub(crate) phase: SlotLeasePhase,
    pub(crate) slot_occupied: bool,
    /// Restored from the store and not touched since by an operation of
    /// this process.
    pub(crate) restored: bool,
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
    /// Otherwise the lowest index that holds no identity, no lease, no
    /// index in `funded_registrations` (the indices of unconsumed
    /// `IdentityRegistration` asset locks) and no shielded identity create
    /// ([`Self::hold_for_shielded_create`]) is leased in the `Reserved` phase.
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
            // An operation of this process holds it now.
            self.forget_restored_claim(&wallet_id, &holder);
            return Ok(existing);
        }
        let identity_index = (0..=RegistrationIndex::MAX)
            .find(|index| {
                !funded_registrations.contains(index) && self.slot_allocatable(&wallet_id, *index)
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

    /// Whether `wallet_id`'s slot `identity_index` holds no identity, no
    /// lease and no pending shielded identity create: what an acquisition
    /// needs, besides no unconsumed registration asset lock there.
    pub fn slot_allocatable(
        &self,
        wallet_id: &WalletId,
        identity_index: RegistrationIndex,
    ) -> bool {
        !self
            .shielded_create_holds
            .get(wallet_id)
            .is_some_and(|held| held.contains(&identity_index))
            && self.slot_state(wallet_id, identity_index) == SlotState::Free
    }

    /// Hold `wallet_id`'s slot `identity_index` for an identity create from
    /// the shielded pool that may still execute: no slot is leased there
    /// until [`Self::release_shielded_create_hold`]. The create's saved
    /// record keeps the index across a restart, which holds it again.
    pub fn hold_for_shielded_create(
        &mut self,
        wallet_id: WalletId,
        identity_index: RegistrationIndex,
    ) {
        self.shielded_create_holds
            .entry(wallet_id)
            .or_default()
            .insert(identity_index);
    }

    /// Drop the hold [`Self::hold_for_shielded_create`] put on `wallet_id`'s
    /// slot `identity_index`.
    pub fn release_shielded_create_hold(
        &mut self,
        wallet_id: &WalletId,
        identity_index: RegistrationIndex,
    ) {
        if let Some(held) = self.shielded_create_holds.get_mut(wallet_id) {
            held.remove(&identity_index);
            if held.is_empty() {
                self.shielded_create_holds.remove(wallet_id);
            }
        }
    }

    /// [`Self::acquire_identity_slot`] for a new registration: it gets a
    /// token no other registration lease of this manager has.
    pub fn acquire_registration_slot(
        &mut self,
        wallet_id: WalletId,
        funded_registrations: &BTreeSet<RegistrationIndex>,
    ) -> Result<(SlotHolder, SlotAcquisition), PlatformWalletError> {
        // Tokens start at 1: a host's zero or default token never names a
        // live lease.
        let token = self.next_registration_token.wrapping_add(1).max(1);
        let holder = SlotHolder::Registration { token };
        let acquisition = self.acquire_identity_slot(wallet_id, holder, funded_registrations)?;
        self.next_registration_token = token;
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
        self.forget_restored_claim(wallet_id, holder);
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
        self.forget_restored_claim(wallet_id, holder);
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
    /// The lease ends with the placement: an occupied slot holds none.
    /// A retained private-payment hold is applied whichever way it arrives.
    pub(crate) fn place_in_slot_unstored(
        &mut self,
        identity: Identity,
        identity_index: RegistrationIndex,
        wallet_id: WalletId,
    ) -> Result<SlotPlacement, PlatformWalletError> {
        let identity_id = identity.id();
        match self.slot_state(&wallet_id, identity_index) {
            SlotState::Occupied(occupant) if occupant == identity_id => {
                self.end_lease_at(&wallet_id, identity_index);
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
        self.end_lease_at(&wallet_id, identity_index);
        Ok(SlotPlacement::Placed)
    }

    /// [`Self::place_in_slot_unstored`], with `keys` (from `identity`, a
    /// fetch) folded by revision onto the identity as a load or discovery
    /// folds them, stored durably as one changeset: the identity, `keys` with
    /// their breadcrumbs and, when the slot was a claim's, the end of its
    /// lease. Memory keeps the placement only once the store is durable, or
    /// while a later flush may still write it
    /// ([`NotDurable::may_still_be_written`](crate::wallet::persister::NotDurable::may_still_be_written));
    /// otherwise it is undone. The store's error is returned either way, so no
    /// caller reports an identity as placed that a restart may not find.
    pub(crate) fn place_in_slot(
        &mut self,
        identity: Identity,
        identity_index: RegistrationIndex,
        wallet_id: WalletId,
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
        let fetched = identity.clone();
        let placement = self.place_in_slot_unstored(identity, identity_index, wallet_id)?;

        let managed = self
            .managed_identity_mut(&identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?;
        // An identity already held (in this slot, or observed) keeps its
        // cached state: `keys` come from a fetch that may predate an update
        // the cache holds, so they are folded by revision first.
        let keys = managed.key_decisions_for_fetch(&fetched, keys);
        let keys_cs = managed.layer_keys(keys);
        let mut changeset = PlatformWalletChangeSet {
            identities: Some(managed.snapshot_changeset()),
            identity_keys: Some(keys_cs),
            ..Default::default()
        };
        if let Some(SlotLease {
            holder: SlotHolder::InvitationClaim { invitation },
            ..
        }) = undo.lease
        {
            changeset.identity_slot_leases =
                Some(self.claim_lease_changeset(&wallet_id, invitation));
        }

        if let Err(not_durable) = persister.store_durably(changeset) {
            // Rows a later flush may still write stay placed in memory too.
            if !not_durable.may_still_be_written() {
                self.undo_placement(identity_id, wallet_id, identity_index, undo);
            }
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

    /// End the lease on `wallet_id`'s slot `identity_index`, whoever holds
    /// it: an occupied slot holds no lease. The holder's operation is over
    /// (its identity landed there) or cannot land there any more (the keys
    /// derived at the index belong to the identity now in it).
    fn end_lease_at(&mut self, wallet_id: &WalletId, identity_index: RegistrationIndex) {
        let Some(leases) = self.slot_leases.get_mut(wallet_id) else {
            return;
        };
        let ended = leases.remove(&identity_index);
        if let Some(ended) = ended {
            self.forget_restored_claim(wallet_id, &ended.holder);
        }
        let Some(leases) = self.slot_leases.get_mut(wallet_id) else {
            return;
        };
        if leases.is_empty() {
            self.slot_leases.remove(wallet_id);
        }
    }

    /// An operation of this process now holds `holder`'s lease, or it ended:
    /// it is no longer one only restored from the store.
    fn forget_restored_claim(&mut self, wallet_id: &WalletId, holder: &SlotHolder) {
        let SlotHolder::InvitationClaim { invitation } = holder else {
            return;
        };
        if let Some(restored) = self.restored_claim_leases.get_mut(wallet_id) {
            restored.remove(invitation);
            if restored.is_empty() {
                self.restored_claim_leases.remove(wallet_id);
            }
        }
    }

    /// `wallet_id`'s invitation claim leases: each one's slot, invitation and
    /// phase, and whether an identity occupies its slot (a lease restored
    /// from the store for a slot filled since it was stored).
    pub(crate) fn claim_leases(&self, wallet_id: &WalletId) -> Vec<ClaimLease> {
        let Some(leases) = self.slot_leases.get(wallet_id) else {
            return Vec::new();
        };
        leases
            .iter()
            .filter_map(|(identity_index, lease)| match lease.holder {
                SlotHolder::InvitationClaim { invitation } => Some(ClaimLease {
                    identity_index: *identity_index,
                    invitation,
                    phase: lease.phase,
                    slot_occupied: self
                        .wallet_identities
                        .get(wallet_id)
                        .is_some_and(|slots| slots.contains_key(identity_index)),
                    restored: self
                        .restored_claim_leases
                        .get(wallet_id)
                        .is_some_and(|restored| restored.contains(&invitation)),
                }),
                SlotHolder::Registration { .. } => None,
            })
            .collect()
    }

    /// End `invitation`'s claim lease, when it still stands as `expected`
    /// (the same slot and phase, as read before the caller's awaits), and
    /// return the stored form of its end with the lease as it was (to put
    /// back when the store fails). `None` when the claim leases nothing in
    /// `wallet_id`, or a live operation has changed its lease since.
    pub(crate) fn end_claim_lease(
        &mut self,
        wallet_id: &WalletId,
        expected: &ClaimLease,
    ) -> Option<(RegistrationIndex, SlotLease, IdentitySlotLeaseChangeSet)> {
        let invitation = expected.invitation;
        let holder = SlotHolder::InvitationClaim { invitation };
        let leased = self.leased_slot(wallet_id, &holder)?;
        if leased.identity_index != expected.identity_index || leased.phase != expected.phase {
            return None;
        }
        self.release_identity_slot(wallet_id, &holder);
        let lease = SlotLease {
            holder,
            phase: leased.phase,
        };
        Some((
            leased.identity_index,
            lease,
            self.claim_lease_changeset(wallet_id, invitation),
        ))
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
            self.restored_claim_leases
                .entry(wallet_id)
                .or_default()
                .insert(invitation);
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
            .place_in_slot(landed, acquired.identity_index, WALLET, keys, &persister)
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

    /// A placement answered by a node behind the cache keeps a key the cache
    /// holds disabled disabled, in memory and in the stored row.
    #[test]
    fn should_keep_a_cached_disable_when_an_older_fetch_places_the_identity() {
        use dpp::identity::accessors::IdentitySettersV0;
        use dpp::identity::identity_public_key::accessors::v0::{
            IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
        };
        let backend = std::sync::Arc::new(Recording::default());
        let persister = persister_of(backend.clone());
        let mut manager = IdentityManager::new();
        let mut cached = identity_with_key(1);
        cached.set_revision(2);
        let mut disabled = cached.public_keys()[&0].clone();
        disabled.set_disabled_at(1_000);
        cached.set_public_keys(BTreeMap::from([(0, disabled)]));
        manager.place_in_slot_unstored(cached, 2, WALLET).unwrap();
        let older = identity_with_key(1);
        let keys = keys_of(&older, 2);

        let placed = manager.place_in_slot(older, 2, WALLET, keys, &persister);

        assert_eq!(placed.unwrap(), SlotPlacement::AlreadyPlaced);
        let id = Identifier::from([1u8; 32]);
        let held = &manager.managed_identity(&id).unwrap().identity;
        assert_eq!(held.public_keys()[&0].disabled_at(), Some(1_000));
        let stored = backend.stored.lock().unwrap();
        let rows = &stored[0].identity_keys.as_ref().unwrap().upserts;
        assert!(rows
            .get(&(id, 0))
            .is_none_or(|row| row.public_key.disabled_at() == Some(1_000)));
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

        let placed =
            manager.place_in_slot(landed, acquired.identity_index, WALLET, keys, &persister);

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
            .place_in_slot_unstored(identity(1), 0, WALLET)
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

        let placed = manager.place_in_slot(landed, 2, WALLET, keys, &persister);

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

    /// Settling ends a claim's lease only as it was read: once a live claim
    /// moved it (submitted it), the read is stale and nothing ends.
    #[test]
    fn should_end_a_claim_lease_only_as_it_was_read() {
        let mut manager = IdentityManager::new();
        manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        let read = manager.claim_leases(&WALLET)[0];
        assert!(!read.restored, "leased by this process");
        manager.set_slot_lease_phase(&WALLET, &claim(1), SlotLeasePhase::Submitted);

        assert!(manager.end_claim_lease(&WALLET, &read).is_none());
        assert!(matches!(
            manager.slot_state(&WALLET, read.identity_index),
            SlotState::Leased(SlotLease {
                phase: SlotLeasePhase::Submitted,
                ..
            })
        ));
        let current = manager.claim_leases(&WALLET)[0];
        assert!(manager.end_claim_lease(&WALLET, &current).is_some());
    }

    /// A lease replayed from the store reads as restored until an operation
    /// of this process takes it up again.
    #[test]
    fn should_read_a_replayed_claim_lease_as_restored_until_retaken() {
        let mut manager = IdentityManager::new();
        manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        let SlotHolder::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        let stored = manager.claim_lease_changeset(&WALLET, invitation);
        let mut restarted = IdentityManager::new();
        restarted.apply_claim_lease_changeset(WALLET, stored);
        assert!(restarted.claim_leases(&WALLET)[0].restored);

        restarted
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        assert!(!restarted.claim_leases(&WALLET)[0].restored);
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
    fn should_skip_a_slot_a_shielded_identity_create_holds_until_it_is_dropped() {
        let mut manager = IdentityManager::new();
        manager.hold_for_shielded_create(WALLET, 0);
        manager.hold_for_shielded_create([8u8; 32], 1);

        let (_, first) = manager.acquire_registration_slot(WALLET, &none()).unwrap();
        assert_eq!(first.identity_index, 1, "index 0 is held for the create");

        manager.release_shielded_create_hold(&WALLET, 0);
        let (_, second) = manager.acquire_registration_slot(WALLET, &none()).unwrap();
        assert_eq!(second.identity_index, 0);
    }

    #[test]
    fn should_lease_the_lowest_free_index() {
        let mut manager = IdentityManager::new();
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET)
            .unwrap();
        manager
            .place_in_slot_unstored(identity(2), 2, WALLET)
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
            .place_in_slot_unstored(identity(1), 0, WALLET)
            .unwrap();

        let refused = manager.place_in_slot_unstored(identity(2), 0, WALLET);

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
            .place_in_slot_unstored(identity(1), 0, WALLET)
            .unwrap();

        let again = manager.place_in_slot_unstored(identity(1), 0, WALLET);

        assert_eq!(again.unwrap(), SlotPlacement::AlreadyPlaced);
    }

    #[test]
    fn should_refuse_an_identity_the_wallet_holds_in_another_slot() {
        let mut manager = IdentityManager::new();
        manager
            .place_in_slot_unstored(identity(1), 0, WALLET)
            .unwrap();

        let refused = manager.place_in_slot_unstored(identity(1), 1, WALLET);

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
            .place_in_slot_unstored(identity(1), acquired.identity_index, WALLET)
            .unwrap();

        assert!(manager.leased_slot(&WALLET, &claim(1)).is_none());
        assert_eq!(
            manager.slot_state(&WALLET, acquired.identity_index),
            SlotState::Occupied(Identifier::from([1u8; 32]))
        );
    }

    #[test]
    fn should_end_any_lease_on_the_slot_an_identity_is_placed_in() {
        let mut manager = IdentityManager::new();
        manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();

        manager
            .place_in_slot_unstored(identity(1), 0, WALLET)
            .unwrap();

        // The keys derived at index 0 belong to the identity now in it, so
        // the claim cannot land there any more: its lease ends, and a retry
        // of the claim is leased another slot.
        assert_eq!(
            manager.slot_state(&WALLET, 0),
            SlotState::Occupied(Identifier::from([1u8; 32]))
        );
        assert!(manager.leased_slot(&WALLET, &claim(1)).is_none());
        let retried = manager
            .acquire_identity_slot(WALLET, claim(1), &none())
            .unwrap();
        assert_eq!(retried.identity_index, 1);
        assert!(!retried.reentered);
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
            .place_in_slot_unstored(identity(1), 3, WALLET)
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
            .place_in_slot_unstored(identity(1), 0, WALLET)
            .unwrap();

        assert!(manager
            .managed_identity(&id)
            .unwrap()
            .dashpay()
            .private_payments_unrestored());
    }
}
