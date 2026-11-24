//! Identity slots for the host: which `identity_index` a registration or an
//! invitation claim derives its keys at.
//!
//! The wallet chooses the index and leases it to the operation before any
//! key is derived (see [`IdentityManager`](crate::wallet::identity::IdentityManager)'s
//! slot model); the host derives the keys there and passes the holder back
//! with them. An invitation claim's lease is stored durably, so a claim
//! retried after a restart derives at the slot its earlier attempt used; a
//! registration's lease lives in memory until its asset lock is tracked,
//! whose index then keeps the slot.

use std::collections::{BTreeMap, BTreeSet};

use dash_sdk::platform::dashpay::identity_id_for_out_point;
use dash_sdk::platform::types::identity::PublicKeyHash;
use dash_sdk::platform::Fetch;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::prelude::Identifier;

use dashcore::OutPoint;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

use crate::changeset::{KeyWithBreadcrumb, PersistenceCapabilities, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::asset_lock::tracked::AssetLockStatus;
use crate::wallet::identity::crypto::ParsedInvitation;
use crate::wallet::identity::state::{
    ClaimLease, RegistrationIndex, SlotHolder, SlotLease, SlotLeasePhase, SlotState,
};
use crate::wallet::platform_wallet::PlatformWalletInfo;

use super::invitation::InvitationClaimantKey;
use super::registration::derived_keys;
use super::*;

/// The operation asking for a slot.
#[derive(Debug, Clone, Copy)]
pub enum SlotRequest<'a> {
    /// A new identity registration. Each request gets its own slot.
    Registration,
    /// A claim of `invitation`. A retry of the same claim gets the slot its
    /// earlier attempt leased.
    InvitationClaim(&'a ParsedInvitation),
}

/// [`SlotRequest`] with the invitation resolved to its funding outpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedSlotRequest {
    Registration,
    InvitationClaim { invitation: OutPoint },
}

/// What [`IdentityWallet::resolve_submitted_claim_leases`] did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClaimLeaseResolution {
    /// Claims found landed: their identity now holds the slot.
    pub(crate) landed: usize,
    /// Leases ended: the claim can never execute, or its slot was filled.
    pub(crate) ended: usize,
    /// Leases left held: the claim may still execute, or it is unknown.
    pub(crate) held: usize,
    /// An identity was found whose owner only this wallet's master key could
    /// tell, and none was given.
    pub(crate) needs_key: bool,
}

/// A slot leased to an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentitySlot {
    /// The index the operation derives its identity keys at.
    pub identity_index: RegistrationIndex,
    /// Who holds the slot; the operation passes it back with its keys.
    pub holder: SlotHolder,
    /// The holder leased this slot already (a claim retried).
    pub reentered: bool,
    /// How far the holder got.
    pub phase: SlotLeasePhase,
}

impl IdentityWallet {
    /// Lease an identity slot of this wallet for `request`.
    ///
    /// The lowest index that holds no identity, no lease and no unconsumed
    /// registration asset lock is leased in the `Reserved` phase; a claim
    /// that already leases a slot gets it back as it is. An invitation
    /// claim's new lease is stored durably before the index is returned, and
    /// is refused with a persister that cannot keep it
    /// ([`PersistenceCapabilities::IDENTITY_SLOT_LEASE_RESTART`]): a claim
    /// retried after a restart would otherwise derive at another index than
    /// the one its earlier attempt may have landed at. On a failed store the
    /// index is free again, unless a later flush may still write the lease
    /// ([`NotDurable::may_still_be_written`](crate::wallet::persister::NotDurable::may_still_be_written)): then the claim keeps it.
    ///
    /// A claim costs one funding-tx fetch: its lease is keyed by the
    /// invitation's funding outpoint, which the link does not carry.
    pub async fn acquire_identity_slot(
        &self,
        request: SlotRequest<'_>,
    ) -> Result<IdentitySlot, PlatformWalletError> {
        let resolved = match request {
            SlotRequest::Registration => ResolvedSlotRequest::Registration,
            SlotRequest::InvitationClaim(invitation) => {
                self.require_slot_lease_persistence()?;
                ResolvedSlotRequest::InvitationClaim {
                    invitation: self.invitation_funding_out_point(invitation).await?,
                }
            }
        };
        self.acquire_resolved_slot(resolved).await
    }

    /// [`Self::acquire_identity_slot`] once the request is resolved.
    pub(crate) async fn acquire_resolved_slot(
        &self,
        request: ResolvedSlotRequest,
    ) -> Result<IdentitySlot, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        let funded = funded_registration_indices(info);
        let manager = &mut info.identity_manager;
        match request {
            ResolvedSlotRequest::Registration => {
                let (holder, acquired) =
                    manager.acquire_registration_slot(self.wallet_id, &funded)?;
                Ok(IdentitySlot {
                    identity_index: acquired.identity_index,
                    holder,
                    reentered: acquired.reentered,
                    phase: acquired.phase,
                })
            }
            ResolvedSlotRequest::InvitationClaim { invitation } => {
                let holder = SlotHolder::InvitationClaim { invitation };
                let acquired = manager.acquire_identity_slot(self.wallet_id, holder, &funded)?;
                if !acquired.reentered {
                    let changeset = PlatformWalletChangeSet {
                        identity_slot_leases: Some(
                            manager.claim_lease_changeset(&self.wallet_id, invitation),
                        ),
                        ..Default::default()
                    };
                    if let Err(not_durable) = self.persister.store_durably(changeset) {
                        // A lease row a later flush may still write keeps
                        // its index: no other holder may lease it meanwhile.
                        if !not_durable.may_still_be_written() {
                            manager.release_identity_slot(&self.wallet_id, &holder);
                        }
                        return Err(self.persister.classify_not_durable(not_durable));
                    }
                }
                Ok(IdentitySlot {
                    identity_index: acquired.identity_index,
                    holder,
                    reentered: acquired.reentered,
                    phase: acquired.phase,
                })
            }
        }
    }

    /// Give up `holder`'s slot: the operation was abandoned before anything
    /// was sent. Returns the index it held, or `None` when it held none.
    ///
    /// A lease that is `Submitted` is refused: a claim's may have reached
    /// Platform, and only its outcome frees the slot; a registration's has an
    /// operation in flight ([`Self::begin_registration`]). A claim's
    /// release is stored durably; on a failed store the lease stays, unless a
    /// later flush may still write the release
    /// ([`NotDurable::may_still_be_written`](crate::wallet::persister::NotDurable::may_still_be_written)): then the slot stays free and
    /// the error is returned.
    pub async fn release_identity_slot(
        &self,
        holder: SlotHolder,
    ) -> Result<Option<RegistrationIndex>, PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        let manager = &mut info.identity_manager;
        let Some(leased) = manager.leased_slot(&self.wallet_id, &holder) else {
            return Ok(None);
        };
        if leased.phase == SlotLeasePhase::Submitted {
            return Err(PlatformWalletError::IdentitySlotClaimSubmitted {
                identity_index: leased.identity_index,
            });
        }
        let SlotHolder::InvitationClaim { invitation } = holder else {
            return Ok(manager.release_identity_slot(&self.wallet_id, &holder));
        };
        manager.release_identity_slot(&self.wallet_id, &holder);
        let changeset = PlatformWalletChangeSet {
            identity_slot_leases: Some(manager.claim_lease_changeset(&self.wallet_id, invitation)),
            ..Default::default()
        };
        if let Err(not_durable) = self.persister.store_durably(changeset) {
            if !not_durable.may_still_be_written() {
                manager.restore_slot_lease(
                    self.wallet_id,
                    leased.identity_index,
                    SlotLease {
                        holder,
                        phase: leased.phase,
                    },
                );
            }
            return Err(self.persister.classify_not_durable(not_durable));
        }
        Ok(Some(leased.identity_index))
    }

    /// Refuse with [`PlatformWalletError::IdentitySlotNotHeld`] unless
    /// `holder` leases this wallet's slot `identity_index`.
    pub(crate) async fn require_slot_lease(
        &self,
        identity_index: RegistrationIndex,
        holder: &SlotHolder,
    ) -> Result<(), PlatformWalletError> {
        let wm = self.wallet_manager.read().await;
        let info = self.current_wallet_info(&wm)?;
        match info
            .identity_manager
            .slot_state(&self.wallet_id, identity_index)
        {
            SlotState::Leased(lease) if lease.holder == *holder => Ok(()),
            SlotState::Leased(_) | SlotState::Occupied(_) | SlotState::Free => {
                Err(PlatformWalletError::IdentitySlotNotHeld { identity_index })
            }
        }
    }

    /// Take `holder`'s registration lease on slot `identity_index` for one
    /// in-flight operation: `Reserved` becomes `Submitted` (in memory; a
    /// registration's lease is not stored). Refused with
    /// [`PlatformWalletError::IdentitySlotNotHeld`] unless `holder` leases that
    /// slot and no other operation runs on the lease, so one lease funds at
    /// most one registration at a time; while it runs, its release is refused
    /// ([`Self::release_identity_slot`]). End it with
    /// [`Self::end_registration`].
    pub(crate) async fn begin_registration(
        &self,
        identity_index: RegistrationIndex,
        holder: &SlotHolder,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        let manager = &mut info.identity_manager;
        match manager.slot_state(&self.wallet_id, identity_index) {
            SlotState::Leased(SlotLease {
                holder: leased,
                phase: SlotLeasePhase::Reserved,
            }) if leased == *holder && matches!(holder, SlotHolder::Registration { .. }) => {
                manager.set_slot_lease_phase(&self.wallet_id, holder, SlotLeasePhase::Submitted);
                Ok(())
            }
            SlotState::Leased(_) | SlotState::Occupied(_) | SlotState::Free => {
                Err(PlatformWalletError::IdentitySlotNotHeld { identity_index })
            }
        }
    }

    /// End the operation [`Self::begin_registration`] started, by its
    /// `result`. A lease the operation still holds goes back to `Reserved`,
    /// for the host to retry or release, unless the operation may have
    /// reached Platform without its outcome being known: then it stays
    /// `Submitted`, unreleasable for the life of this process. A landed
    /// registration's placement has ended the lease already.
    pub(crate) async fn end_registration<T>(
        &self,
        identity_index: RegistrationIndex,
        holder: &SlotHolder,
        result: &Result<T, PlatformWalletError>,
    ) {
        if result.as_ref().is_err_and(outcome_unknown) {
            return;
        }
        let mut wm = self.wallet_manager.write().await;
        let Ok(info) = self.current_wallet_info_mut(&mut wm) else {
            return;
        };
        let manager = &mut info.identity_manager;
        if let SlotState::Leased(lease) = manager.slot_state(&self.wallet_id, identity_index) {
            if lease.holder == *holder {
                manager.set_slot_lease_phase(&self.wallet_id, holder, SlotLeasePhase::Reserved);
            }
        }
    }

    /// Move `invitation`'s claim lease on slot `identity_index` to
    /// `Submitted`, stored durably, before its claim is sent: from then on
    /// only the claim's outcome frees the slot. `Ok(false)` when an earlier
    /// attempt already submitted it: that attempt's claim may still land, so
    /// this one may not give the slot back on a refusal of its own. Refused with
    /// [`PlatformWalletError::IdentitySlotNotHeld`] unless the claim leases
    /// that slot; on a failed store the lease keeps its phase, unless a later
    /// flush may still write the new one ([`NotDurable::may_still_be_written`](crate::wallet::persister::NotDurable::may_still_be_written)),
    /// which memory then keeps.
    pub(crate) async fn submit_claim_lease(
        &self,
        identity_index: RegistrationIndex,
        invitation: OutPoint,
    ) -> Result<bool, PlatformWalletError> {
        self.set_claim_lease_phase(identity_index, invitation, SlotLeasePhase::Submitted)
            .await
    }

    /// Move `invitation`'s claim lease on slot `identity_index` back to
    /// `Reserved`, stored durably: its claim was refused in a way that can
    /// never execute, so the host may retry or release it. Refused like
    /// [`Self::submit_claim_lease`]; on a failed store the lease stays
    /// `Submitted`, unless a later flush may still write `Reserved`.
    pub(crate) async fn unsubmit_claim_lease(
        &self,
        identity_index: RegistrationIndex,
        invitation: OutPoint,
    ) -> Result<bool, PlatformWalletError> {
        self.set_claim_lease_phase(identity_index, invitation, SlotLeasePhase::Reserved)
            .await
    }

    /// Move the claim lease to `phase`, stored durably. `Ok(true)` when this
    /// call moved it, `Ok(false)` when it already stood there.
    async fn set_claim_lease_phase(
        &self,
        identity_index: RegistrationIndex,
        invitation: OutPoint,
        phase: SlotLeasePhase,
    ) -> Result<bool, PlatformWalletError> {
        let holder = SlotHolder::InvitationClaim { invitation };
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        let manager = &mut info.identity_manager;
        let lease = match manager.slot_state(&self.wallet_id, identity_index) {
            SlotState::Leased(lease) if lease.holder == holder => lease,
            SlotState::Leased(_) | SlotState::Occupied(_) | SlotState::Free => {
                return Err(PlatformWalletError::IdentitySlotNotHeld { identity_index });
            }
        };
        if lease.phase == phase {
            return Ok(false);
        }
        manager.set_slot_lease_phase(&self.wallet_id, &holder, phase);
        let changeset = PlatformWalletChangeSet {
            identity_slot_leases: Some(manager.claim_lease_changeset(&self.wallet_id, invitation)),
            ..Default::default()
        };
        if let Err(not_durable) = self.persister.store_durably(changeset) {
            if !not_durable.may_still_be_written() {
                manager.set_slot_lease_phase(&self.wallet_id, &holder, lease.phase);
            }
            return Err(self.persister.classify_not_durable(not_durable));
        }
        Ok(true)
    }

    /// Before anything is funded at `identity_index`: the identity Platform
    /// holds under `keys_map[0]` (the master key derived there), if any.
    /// Found, and `expected` (when given) is its id, it is recorded in slot
    /// `identity_index`, ending the slot's lease in the same write, and
    /// [`PlatformWalletError::IdentitySlotTakenOnChain`] is returned for the
    /// caller to hand back. A failed lookup fails closed.
    ///
    /// Each key of the found identity whose id and data match `keys_map` is
    /// stored with its derivation breadcrumb: those are the keys derived at
    /// this index. Any other key is stored watch-only.
    pub(crate) async fn slot_taken_on_chain(
        &self,
        identity_index: RegistrationIndex,
        keys_map: &BTreeMap<KeyID, IdentityPublicKey>,
        expected: Option<Identifier>,
    ) -> Result<Option<PlatformWalletError>, PlatformWalletError> {
        let Some(master) = keys_map.get(&0) else {
            return Ok(None);
        };
        let hash = master.public_key_hash().map_err(|e| {
            PlatformWalletError::InvalidIdentityData(format!(
                "the master key derived at identity index {identity_index} has no key hash: {e}"
            ))
        })?;
        let found = Identity::fetch(&self.sdk, PublicKeyHash(hash))
            .await
            .map_err(|e| {
                PlatformWalletError::InvalidIdentityData(format!(
                    "could not check Platform for an identity at identity index \
                     {identity_index}: {e}"
                ))
            })?;
        let Some(found) = found else {
            return Ok(None);
        };
        let identity_id = found.id();
        if expected.is_some_and(|expected| expected != identity_id) {
            return Ok(None);
        }
        let keys = found
            .public_keys()
            .iter()
            .map(|(key_id, key)| {
                let derived = keys_map.get(key_id).is_some_and(|ours| {
                    ours.data() == key.data() && ours.key_type() == key.key_type()
                });
                KeyWithBreadcrumb {
                    key: key.clone(),
                    breadcrumb: derived.then_some((self.wallet_id, identity_index, *key_id)),
                }
            })
            .collect();
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        info.identity_manager.place_in_slot(
            found,
            identity_index,
            self.wallet_id,
            keys,
            &self.persister,
        )?;
        tracing::warn!(
            identity_index,
            %identity_id,
            "the identity keys derived at this index already belong to an identity on \
             Platform; recorded it in the slot"
        );
        Ok(Some(PlatformWalletError::IdentitySlotTakenOnChain {
            identity_index,
            identity_id,
        }))
    }

    /// Record `identity`, registered at `identity_index` with keys derived
    /// there, in that slot, ending the slot's lease in the same durable
    /// write ([`IdentityManager::place_in_slot`](crate::wallet::identity::IdentityManager::place_in_slot)).
    pub(crate) async fn record_registered_identity(
        &self,
        identity: Identity,
        identity_index: RegistrationIndex,
    ) -> Result<(), PlatformWalletError> {
        let keys = derived_keys(&identity, self.wallet_id, identity_index);
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        info.identity_manager.place_in_slot(
            identity,
            identity_index,
            self.wallet_id,
            keys,
            &self.persister,
        )?;
        Ok(())
    }

    /// Hold slot `identity_index` for an identity create from the shielded
    /// pool that may still execute
    /// ([`IdentityManager::hold_for_shielded_create`](crate::wallet::identity::IdentityManager::hold_for_shielded_create)),
    /// or drop that hold
    /// once it resolved.
    #[cfg(feature = "shielded")]
    pub(crate) async fn set_shielded_create_hold(
        &self,
        identity_index: RegistrationIndex,
        held: bool,
    ) -> Result<(), PlatformWalletError> {
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        if held {
            info.identity_manager
                .hold_for_shielded_create(self.wallet_id, identity_index);
        } else {
            info.identity_manager
                .release_shielded_create_hold(&self.wallet_id, identity_index);
        }
        Ok(())
    }

    /// Settle this wallet's invitation claim leases whose claim may have
    /// reached Platform (`Submitted`), from what Platform holds at each
    /// claim's identity id (derived from its invitation outpoint alone), and
    /// end every claim lease restored for a slot an identity has filled
    /// since, and every `Reserved` one (nothing was sent for it, and the
    /// operation that leased it ended with its process). Run at startup,
    /// before any claim of this process leases a slot: a claim interrupted or
    /// left unanswered by an earlier process otherwise holds its slot for
    /// good.
    ///
    /// Per `Submitted` lease:
    /// - no identity at the id, or Platform not answering: it may still
    ///   execute, so the slot stays held;
    /// - an identity holding this wallet's master key at the slot: the claim
    ///   landed; the identity is recorded in the slot, ending the lease;
    /// - an identity that does not hold it: the invitation was claimed
    ///   otherwise (by another wallet, or by this one at another slot), so
    ///   this claim can never execute; the lease ends;
    /// - whose identity it is cannot be told (`key` is
    ///   [`InvitationClaimantKey::Unavailable`], or the derivation fails):
    ///   the slot stays held, and [`ClaimLeaseResolution::needs_key`] says a
    ///   key would settle it.
    ///
    /// Every end is stored durably; a failed store keeps the lease. A lease
    /// that stays is left `Submitted` for a later start (or a retried claim)
    /// to settle.
    pub(crate) async fn resolve_submitted_claim_leases(
        &self,
        key: InvitationClaimantKey<'_>,
    ) -> Result<ClaimLeaseResolution, PlatformWalletError> {
        let leases = {
            let wm = self.wallet_manager.read().await;
            self.current_wallet_info(&wm)?
                .identity_manager
                .claim_leases(&self.wallet_id)
        };
        let mut resolution = ClaimLeaseResolution::default();
        for lease in leases {
            if lease.slot_occupied {
                if self.end_claim_lease(&lease).await {
                    resolution.ended += 1;
                }
                continue;
            }
            // Nothing was sent for a `Reserved` claim restored from the
            // store, and the operation that leased it ended with the process
            // that ran it: no claim can land there, and nothing else would
            // release the lease. One this process leased or retried since is
            // a live claim's, left to it.
            if lease.phase != SlotLeasePhase::Submitted {
                if lease.restored && self.end_claim_lease(&lease).await {
                    resolution.ended += 1;
                }
                continue;
            }
            let identity_id = identity_id_for_out_point(lease.invitation);
            let found = match Identity::fetch(&self.sdk, identity_id).await {
                Ok(Some(found)) => found,
                Ok(None) => {
                    resolution.held += 1;
                    continue;
                }
                Err(e) => {
                    tracing::warn!(
                        identity_index = lease.identity_index,
                        error = %e,
                        "could not ask Platform whether an invitation claim landed; its \
                         slot stays held"
                    );
                    resolution.held += 1;
                    continue;
                }
            };
            match self
                .invitation_claimed_by_this_wallet(Some(&found), lease.identity_index, key)
                .await
            {
                Some(true) => {
                    let recovered = self
                        .record_found_identity(lease.identity_index, identity_id, key)
                        .await;
                    match recovered {
                        // Placing the identity in the slot ended the lease in
                        // memory, which is stored here; an identity this
                        // wallet held at another slot leaves it to end.
                        Ok(true) => {
                            if !self.end_claim_lease(&lease).await {
                                self.store_claim_lease(lease.invitation).await;
                            }
                            resolution.landed += 1;
                        }
                        Ok(false) => resolution.held += 1,
                        Err(e) => {
                            tracing::warn!(
                                identity_index = lease.identity_index,
                                error = %e,
                                "an invitation claim landed but its identity could not be \
                                 recorded; its slot stays held"
                            );
                            resolution.held += 1;
                        }
                    }
                }
                Some(false) => {
                    if self.end_claim_lease(&lease).await {
                        resolution.ended += 1;
                    } else {
                        resolution.held += 1;
                    }
                }
                None => {
                    resolution.held += 1;
                    resolution.needs_key |= matches!(key, InvitationClaimantKey::Unavailable);
                }
            }
        }
        Ok(resolution)
    }

    /// Record `identity_id`, found holding this wallet's master key at
    /// `identity_index`, in that slot by loading the identity registered
    /// there. `Ok(true)` once this wallet holds it in one of its slots;
    /// `Ok(false)` when it does not, without `key`, or when it is only an
    /// observed identity (moving one of those into its slot is left to
    /// claim recovery).
    async fn record_found_identity(
        &self,
        identity_index: u32,
        identity_id: Identifier,
        key: InvitationClaimantKey<'_>,
    ) -> Result<bool, PlatformWalletError> {
        if self.holds_identity(&identity_id).await {
            return Ok(true);
        }
        match key {
            InvitationClaimantKey::Master(master) => {
                self.load_identity_by_index_from_master(identity_index, master)
                    .await?;
            }
            InvitationClaimantKey::ResidentWallet => {
                self.load_identity_by_index(identity_index).await?;
            }
            InvitationClaimantKey::Unavailable => return Ok(false),
        }
        Ok(self.holds_identity(&identity_id).await)
    }

    /// Settle this wallet's registration asset locks that may have created
    /// their identity without this wallet recording it: a submit whose
    /// answer was lost, or one that landed but whose identity was not stored
    /// in its slot. Platform derives the identity id from the lock's
    /// outpoint alone, so each lock that left the wallet costs one lookup:
    /// - an identity at the id holding this wallet's master key at the
    ///   lock's index: recorded in that slot, and the lock marked consumed;
    /// - none, or no answer: the lock stays as it is (still resumable);
    /// - whose identity it is cannot be told: it stays, and
    ///   [`ClaimLeaseResolution::needs_key`] says a key would settle it.
    ///
    /// Counted in the same [`ClaimLeaseResolution`] as the claim leases.
    pub(crate) async fn resolve_registration_locks(
        &self,
        key: InvitationClaimantKey<'_>,
    ) -> Result<ClaimLeaseResolution, PlatformWalletError> {
        let locks: Vec<(OutPoint, RegistrationIndex)> = {
            let wm = self.wallet_manager.read().await;
            self.current_wallet_info(&wm)?
                .tracked_asset_locks
                .values()
                .filter(|lock| {
                    lock.funding_type == AssetLockFundingType::IdentityRegistration
                        && !lock.status.is_consumed()
                        && lock.status != AssetLockStatus::Built
                })
                .map(|lock| (lock.out_point, lock.identity_index))
                .collect()
        };
        let mut resolution = ClaimLeaseResolution::default();
        for (out_point, identity_index) in locks {
            let identity_id = identity_id_for_out_point(out_point);
            let found = match Identity::fetch(&self.sdk, identity_id).await {
                Ok(Some(found)) => found,
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(
                        %out_point,
                        error = %e,
                        "could not ask Platform whether a registration lock created its \
                         identity"
                    );
                    resolution.held += 1;
                    continue;
                }
            };
            let recovered = match self
                .invitation_claimed_by_this_wallet(Some(&found), identity_index, key)
                .await
            {
                Some(true) => {
                    self.record_found_identity(identity_index, identity_id, key)
                        .await
                }
                Some(false) => {
                    tracing::warn!(
                        %out_point,
                        identity_index,
                        "a registration lock's identity does not hold this wallet's master key \
                         at the lock's index; left as it is"
                    );
                    resolution.held += 1;
                    continue;
                }
                None => {
                    resolution.held += 1;
                    resolution.needs_key |= matches!(key, InvitationClaimantKey::Unavailable);
                    continue;
                }
            };
            match recovered {
                Ok(true) => {
                    if let Err(e) = self.asset_locks.consume_asset_lock(&out_point).await {
                        tracing::warn!(
                            %out_point,
                            error = %e,
                            "a registration lock's identity was recorded, but the lock could \
                             not be marked consumed"
                        );
                    }
                    resolution.landed += 1;
                }
                Ok(false) => resolution.held += 1,
                Err(e) => {
                    tracing::warn!(
                        %out_point,
                        error = %e,
                        "a registration lock created its identity, but it could not be recorded"
                    );
                    resolution.held += 1;
                }
            }
        }
        Ok(resolution)
    }

    /// End `invitation`'s claim lease, stored durably. `false` when it
    /// leases nothing, when this handle's wallet is no longer the one the
    /// manager holds, or when the store fails (the lease is put back).
    async fn end_claim_lease(&self, lease: &ClaimLease) -> bool {
        let mut wm = self.wallet_manager.write().await;
        let Ok(info) = self.current_wallet_info_mut(&mut wm) else {
            return false;
        };
        let manager = &mut info.identity_manager;
        let Some((identity_index, lease, ended)) = manager.end_claim_lease(&self.wallet_id, lease)
        else {
            return false;
        };
        let changeset = PlatformWalletChangeSet {
            identity_slot_leases: Some(ended),
            ..Default::default()
        };
        if let Err(not_durable) = self.persister.store_durably(changeset) {
            if not_durable.may_still_be_written() {
                tracing::warn!(
                    identity_index,
                    "the end of an invitation claim's lease is not stored yet; a later flush \
                     writes it"
                );
                return true;
            }
            manager.restore_slot_lease(self.wallet_id, identity_index, lease);
            tracing::warn!(
                identity_index,
                error = %self.persister.classify_not_durable(not_durable),
                "the end of an invitation claim's lease was not stored; it stays held"
            );
            return false;
        }
        true
    }

    /// Store `invitation`'s claim lease as it now stands in memory. A failed
    /// store leaves the stored lease behind, for the next start to end.
    async fn store_claim_lease(&self, invitation: OutPoint) {
        let wm = self.wallet_manager.read().await;
        let Ok(info) = self.current_wallet_info(&wm) else {
            return;
        };
        let changeset = PlatformWalletChangeSet {
            identity_slot_leases: Some(
                info.identity_manager
                    .claim_lease_changeset(&self.wallet_id, invitation),
            ),
            ..Default::default()
        };
        if let Err(not_durable) = self.persister.store_durably(changeset) {
            tracing::warn!(
                error = %self.persister.classify_not_durable(not_durable),
                "the end of a landed invitation claim's lease was not stored; the next \
                 start ends it"
            );
        }
    }

    /// Refuse an invitation claim's slot when the persister cannot keep its
    /// lease across a restart.
    fn require_slot_lease_persistence(&self) -> Result<(), PlatformWalletError> {
        let capabilities = self.persister.persistence_capabilities();
        let required = PersistenceCapabilities::IDENTITY_SLOT_LEASE_RESTART;
        if capabilities.contains(required) {
            return Ok(());
        }
        let missing = capabilities.missing(required);
        Err(PlatformWalletError::Persistence(format!(
            "an invitation claim requires persistence capabilities {:?} (missing mask 0x{:x}): \
             without them a claim retried after a restart could derive its keys at another \
             identity index than its earlier attempt",
            missing.names(),
            missing.bits(),
        )))
    }
}

/// The indices of `info`'s registration asset locks not yet consumed: each
/// holds its index for the registration it funds.
fn funded_registration_indices(info: &PlatformWalletInfo) -> BTreeSet<RegistrationIndex> {
    info.tracked_asset_locks
        .values()
        .filter(|lock| {
            lock.funding_type == AssetLockFundingType::IdentityRegistration
                && !lock.status.is_consumed()
        })
        .map(|lock| lock.identity_index)
        .collect()
}

/// Whether `error` leaves a registration's outcome unknown: it may have
/// reached Platform and may still execute.
fn outcome_unknown(error: &PlatformWalletError) -> bool {
    match error {
        PlatformWalletError::Sdk(error) => error.is_state_transition_result_unconfirmed(),
        PlatformWalletError::TransactionBroadcastUnconfirmed(_)
        | PlatformWalletError::ShieldedBroadcastUnconfirmed { .. }
        | PlatformWalletError::ShieldedSpendUnconfirmed { .. } => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dashcore::{Transaction, Txid};
    use key_wallet::wallet::initialization::WalletAccountCreationOptions;
    use key_wallet::Network;

    use super::*;
    use crate::changeset::{ClientStartState, PersistenceError, PlatformWalletPersistence};
    use crate::events::{EventHandler, PlatformEventHandler};
    use crate::wallet::asset_lock::tracked::TrackedAssetLock;
    use crate::wallet::platform_wallet::WalletId;
    use crate::{PlatformWallet, PlatformWalletManager};

    /// A backend committing inline, declaring `capabilities`, recording each
    /// changeset, and failing every store while `fail` is set.
    struct Backend {
        capabilities: PersistenceCapabilities,
        fail: Mutex<bool>,
        /// While set, stores buffer and every flush fails transiently,
        /// keeping the buffer.
        flush_fails_transiently: Mutex<bool>,
        stores: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl Backend {
        fn new(capabilities: PersistenceCapabilities) -> Arc<Self> {
            Arc::new(Self {
                capabilities,
                fail: Mutex::new(false),
                flush_fails_transiently: Mutex::new(false),
                stores: Mutex::new(Vec::new()),
            })
        }

        fn lease_writes(&self) -> Vec<crate::changeset::IdentitySlotLeaseChangeSet> {
            self.stores
                .lock()
                .unwrap()
                .iter()
                .filter_map(|cs| cs.identity_slot_leases.clone())
                .collect()
        }
    }

    impl PlatformWalletPersistence for Backend {
        fn store_commits_inline(&self) -> bool {
            !*self.flush_fails_transiently.lock().unwrap()
        }
        fn persistence_capabilities(&self) -> PersistenceCapabilities {
            self.capabilities
        }
        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if *self.fail.lock().unwrap() {
                return Err(PersistenceError::backend("the host refused the write"));
            }
            self.stores.lock().unwrap().push(changeset);
            Ok(())
        }
        fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
            if *self.flush_fails_transiently.lock().unwrap() {
                return Err(PersistenceError::backend_with_kind(
                    crate::changeset::PersistenceErrorKind::Transient,
                    "the host is busy",
                ));
            }
            Ok(())
        }
        fn load(&self) -> Result<ClientStartState, PersistenceError> {
            Ok(ClientStartState::default())
        }
    }

    async fn wallet(backend: Arc<Backend>) -> Arc<PlatformWallet> {
        wallet_answering(backend, None).await
    }

    /// A wallet whose node answers a lookup of the master key hash
    /// [`master_keys`] holds with `registered`, when given.
    async fn wallet_answering(
        backend: Arc<Backend>,
        registered: Option<Option<Identity>>,
    ) -> Arc<PlatformWallet> {
        struct Events;
        impl EventHandler for Events {}
        impl PlatformEventHandler for Events {}
        let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
        if let Some(registered) = registered {
            sdk.mock()
                .expect_fetch(PublicKeyHash([1; 20]), registered)
                .await
                .unwrap();
        }
        let manager = PlatformWalletManager::new(Arc::new(sdk), backend, Arc::new(Events));
        manager
            .create_wallet_from_seed_bytes(
                Network::Testnet,
                &[9; 64],
                WalletAccountCreationOptions::Default,
                Some(0),
            )
            .await
            .unwrap()
    }

    fn claim(byte: u8) -> ResolvedSlotRequest {
        ResolvedSlotRequest::InvitationClaim {
            invitation: OutPoint::new(Txid::from_byte_array([byte; 32]), 0),
        }
    }

    fn full() -> PersistenceCapabilities {
        PersistenceCapabilities::IDENTITY_SLOT_LEASE_RESTART
    }

    #[tokio::test]
    async fn should_refuse_a_claim_slot_without_lease_persistence() {
        let backend = Backend::new(PersistenceCapabilities::ATOMIC_CHANGESETS);
        let wallet = wallet(backend.clone()).await;

        // Refused before the funding tx is fetched: the mock SDK expects no
        // fetch, so reaching one would fail with another error.
        let invitation = ParsedInvitation {
            voucher_key: dpp::secp256k1::SecretKey::from_secret_bytes([1; 32]).unwrap(),
            voucher_key_network: Network::Testnet,
            funding_txid: "11".repeat(32),
            islock_hex: None,
            inviter: None,
        };
        let refused = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::InvitationClaim(&invitation))
            .await;

        assert!(matches!(refused, Err(PlatformWalletError::Persistence(_))));
        assert!(backend.lease_writes().is_empty());
        // A registration needs no stored lease and still gets slot 0.
        let registered = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        assert_eq!(registered.identity_index, 0);
    }

    #[tokio::test]
    async fn should_store_a_new_claim_lease_once_and_return_it_on_retry() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;

        let first = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let retried = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();

        assert_eq!(first.identity_index, 0);
        assert!(!first.reentered);
        assert_eq!(retried.identity_index, 0);
        assert!(retried.reentered);
        let writes = backend.lease_writes();
        assert_eq!(writes.len(), 1);
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        assert_eq!(writes[0].leases[&invitation].identity_index, 0);
    }

    #[tokio::test]
    async fn should_lease_nothing_when_the_claim_lease_is_not_stored() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        *backend.fail.lock().unwrap() = true;

        let refused = wallet.identity().acquire_resolved_slot(claim(1)).await;

        assert!(refused.is_err());
        *backend.fail.lock().unwrap() = false;
        let next = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        assert_eq!(next.identity_index, 0);
    }

    #[tokio::test]
    async fn should_give_two_registrations_their_own_slots_and_holders() {
        let wallet = wallet(Backend::new(full())).await;

        let first = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        let second = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        assert_ne!(first.identity_index, second.identity_index);
        assert_ne!(first.holder, second.holder);
    }

    #[tokio::test]
    async fn should_skip_the_index_of_an_unconsumed_registration_asset_lock() {
        let wallet = wallet(Backend::new(full())).await;
        {
            let mut wm = wallet.identity().wallet_manager.write().await;
            let info = wm.get_wallet_info_mut(&wallet.wallet_id()).unwrap();
            for (byte, funding_type, status) in [
                (
                    1,
                    AssetLockFundingType::IdentityRegistration,
                    AssetLockStatus::Broadcast,
                ),
                (
                    2,
                    AssetLockFundingType::IdentityTopUp,
                    AssetLockStatus::Broadcast,
                ),
            ] {
                let out_point = OutPoint::new(Txid::from_byte_array([byte; 32]), 0);
                info.tracked_asset_locks.insert(
                    out_point,
                    TrackedAssetLock {
                        operation_id: None,
                        out_point,
                        transaction: Transaction {
                            version: 3,
                            lock_time: 0,
                            input: vec![],
                            output: vec![],
                            special_transaction_payload: None,
                        },
                        account_index: 0,
                        funding_type,
                        identity_index: u32::from(byte) - 1,
                        amount: 1,
                        status,
                        proof: None,
                        shielded_recipient: None,
                    },
                );
            }
        }

        let acquired = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        // Index 0 is the registration lock's; the top-up lock's index 1
        // holds no slot.
        assert_eq!(acquired.identity_index, 1);
    }

    #[tokio::test]
    async fn should_store_a_reserved_claims_release() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();

        let released = wallet.identity().release_identity_slot(slot.holder).await;

        assert_eq!(released.unwrap(), Some(0));
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        assert_eq!(
            backend.lease_writes().last().unwrap().released,
            [invitation].into()
        );
    }

    #[tokio::test]
    async fn should_keep_a_claims_lease_when_its_release_is_not_stored() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        *backend.fail.lock().unwrap() = true;

        let refused = wallet.identity().release_identity_slot(slot.holder).await;

        assert!(refused.is_err());
        *backend.fail.lock().unwrap() = false;
        let retried = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        assert!(retried.reentered);
        assert_eq!(retried.identity_index, slot.identity_index);
    }

    #[tokio::test]
    async fn should_refuse_to_release_a_submitted_claim() {
        let wallet = wallet(Backend::new(full())).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        {
            let mut wm = wallet.identity().wallet_manager.write().await;
            wm.get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .set_slot_lease_phase(&wallet.wallet_id(), &slot.holder, SlotLeasePhase::Submitted);
        }

        let refused = wallet.identity().release_identity_slot(slot.holder).await;

        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::IdentitySlotClaimSubmitted { identity_index })
                    if identity_index == slot.identity_index
            ),
            "{refused:?}"
        );
        let retried = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        assert!(retried.reentered);
    }

    /// Never asked to sign: every call here is refused before signing.
    #[derive(Debug)]
    struct UnusedSigner;

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<dpp::identity::IdentityPublicKey> for UnusedSigner {
        async fn sign(
            &self,
            _key: &dpp::identity::IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("refused before signing")
        }

        async fn sign_create_witness(
            &self,
            _key: &dpp::identity::IdentityPublicKey,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("refused before signing")
        }

        fn can_sign_with(&self, _key: &dpp::identity::IdentityPublicKey) -> bool {
            false
        }
    }

    #[async_trait::async_trait]
    impl dpp::identity::signer::Signer<dpp::address_funds::PlatformAddress> for UnusedSigner {
        async fn sign(
            &self,
            _key: &dpp::address_funds::PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::platform_value::BinaryData, dpp::ProtocolError> {
            unreachable!("refused before signing")
        }

        async fn sign_create_witness(
            &self,
            _key: &dpp::address_funds::PlatformAddress,
            _data: &[u8],
        ) -> Result<dpp::address_funds::AddressWitness, dpp::ProtocolError> {
            unreachable!("refused before signing")
        }

        fn can_sign_with(&self, _key: &dpp::address_funds::PlatformAddress) -> bool {
            false
        }
    }

    /// Register at `identity_index` with `holder`, funded from a Platform
    /// address.
    async fn register_from_addresses_at(
        wallet: &PlatformWallet,
        identity_index: RegistrationIndex,
        holder: SlotHolder,
    ) -> Result<dpp::identity::Identity, PlatformWalletError> {
        let placeholder = Identity::V0(dpp::identity::v0::IdentityV0 {
            id: Identifier::default(),
            public_keys: master_keys(),
            balance: 0,
            revision: 0,
        });
        wallet
            .register_from_addresses(
                &placeholder,
                std::collections::BTreeMap::from([(
                    dpp::address_funds::PlatformAddress::P2pkh([0x11; 20]),
                    1_000_000,
                )]),
                None,
                identity_index,
                holder,
                &UnusedSigner,
                &UnusedSigner,
                None,
            )
            .await
    }

    fn master_keys() -> std::collections::BTreeMap<u32, dpp::identity::IdentityPublicKey> {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{KeyType, Purpose, SecurityLevel};
        let key = IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            key_type: KeyType::ECDSA_HASH160,
            data: vec![1; 20].into(),
            ..Default::default()
        };
        std::collections::BTreeMap::from([(0, key.into())])
    }

    /// Register at `identity_index` with `holder`, funded from the balance.
    async fn register_at(
        wallet: &PlatformWallet,
        identity_index: RegistrationIndex,
        holder: Option<SlotHolder>,
    ) -> Result<dpp::identity::Identity, PlatformWalletError> {
        let asset_lock_signer = crate::test_support::fund_test_platform_wallet(
            &wallet.identity().wallet_manager,
            &wallet.wallet_id(),
            1_000_000,
        )
        .await;
        wallet
            .identity()
            .register_identity_with_funding(
                crate::wallet::asset_lock::AssetLockFunding::FromWalletBalance {
                    amount_duffs: 100_000,
                    account_index: 0,
                },
                identity_index,
                holder,
                master_keys(),
                &UnusedSigner,
                &asset_lock_signer,
                None,
            )
            .await
    }

    async fn tracked_locks(wallet: &PlatformWallet) -> usize {
        let wm = wallet.identity().wallet_manager.read().await;
        wm.get_wallet_info(&wallet.wallet_id())
            .unwrap()
            .tracked_asset_locks
            .len()
    }

    /// A registration lease runs one operation at a time: a second use of
    /// the same lease, and its release, are refused while one runs; a
    /// definite failure gives it back, an unknown outcome keeps it.
    #[tokio::test]
    async fn should_run_one_operation_at_a_time_on_a_registration_lease() {
        let wallet = wallet(Backend::new(full())).await;
        let identity = wallet.identity();
        let slot = identity
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        let index = slot.identity_index;

        identity
            .begin_registration(index, &slot.holder)
            .await
            .unwrap();
        assert!(matches!(
            identity.begin_registration(index, &slot.holder).await,
            Err(PlatformWalletError::IdentitySlotNotHeld { .. })
        ));
        assert!(matches!(
            identity.release_identity_slot(slot.holder).await,
            Err(PlatformWalletError::IdentitySlotClaimSubmitted { .. })
        ));

        let refused: Result<(), _> = Err(PlatformWalletError::InvalidIdentityData("no".into()));
        identity
            .end_registration(index, &slot.holder, &refused)
            .await;
        identity
            .begin_registration(index, &slot.holder)
            .await
            .unwrap();
        let unknown: Result<(), _> = Err(PlatformWalletError::TransactionBroadcastUnconfirmed(
            "lost".into(),
        ));
        identity
            .end_registration(index, &slot.holder, &unknown)
            .await;
        assert!(identity.release_identity_slot(slot.holder).await.is_err());
    }

    /// A registration lock funded outside a lease (the generic asset-lock
    /// exports) goes only to an index the allocator would hand out.
    #[tokio::test]
    async fn should_fund_a_registration_lock_outside_a_lease_only_at_a_free_index() {
        let wallet = wallet(Backend::new(full())).await;
        let leased = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        track_registration_lock_at(&wallet, OutPoint::new(Txid::from_byte_array([4; 32]), 0), 1)
            .await;
        let asset_locks = &wallet.identity().asset_locks;

        for taken in [leased.identity_index, 1] {
            assert!(matches!(
                asset_locks.ensure_registration_index_unallocated(taken).await,
                Err(PlatformWalletError::IdentitySlotNotHeld { identity_index }) if identity_index == taken
            ));
        }
        asset_locks
            .ensure_registration_index_unallocated(2)
            .await
            .unwrap();
        assert_ne!(
            leased.holder,
            SlotHolder::Registration { token: 0 },
            "a zero token never names a live lease"
        );
    }

    /// Two registrations with one lease: the second is refused before
    /// funding while the first runs, so one lease never funds two creates.
    #[tokio::test]
    async fn should_refuse_a_registration_whose_lease_is_in_flight() {
        let wallet = wallet(Backend::new(full())).await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        wallet
            .identity()
            .begin_registration(slot.identity_index, &slot.holder)
            .await
            .unwrap();

        let refused = register_at(&wallet, slot.identity_index, Some(slot.holder)).await;
        let from_addresses =
            register_from_addresses_at(&wallet, slot.identity_index, slot.holder).await;

        for refused in [refused.map(|_| ()), from_addresses.map(|_| ())] {
            assert!(
                matches!(
                    refused,
                    Err(PlatformWalletError::IdentitySlotNotHeld { .. })
                ),
                "{refused:?}"
            );
        }
        assert_eq!(tracked_locks(&wallet).await, 0);
    }

    #[tokio::test]
    async fn should_refuse_a_registration_without_its_slot_before_funding() {
        let wallet = wallet(Backend::new(full())).await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        let other = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        for (index, holder) in [
            (slot.identity_index, None),
            (slot.identity_index, Some(other.holder)),
            (5, Some(slot.holder)),
        ] {
            let refused = register_at(&wallet, index, holder).await;
            assert!(
                matches!(
                    refused,
                    Err(PlatformWalletError::IdentitySlotNotHeld { identity_index }) if identity_index == index
                ),
                "index {index}, holder {holder:?}: {refused:?}"
            );
        }
        assert_eq!(tracked_locks(&wallet).await, 0);
    }

    #[tokio::test]
    async fn should_refuse_an_address_registration_without_its_slot_before_spending() {
        // The node is given no answers: a refusal here never reached it.
        let wallet = wallet(Backend::new(full())).await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        let other = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        let invitation = OutPoint::new(Txid::from_byte_array([1; 32]), 0);

        for (index, holder) in [
            (slot.identity_index, other.holder),
            (
                slot.identity_index,
                SlotHolder::InvitationClaim { invitation },
            ),
            (5, slot.holder),
        ] {
            let refused = register_from_addresses_at(&wallet, index, holder).await;
            assert!(
                matches!(
                    refused,
                    Err(PlatformWalletError::IdentitySlotNotHeld { identity_index }) if identity_index == index
                ),
                "index {index}, holder {holder:?}: {refused:?}"
            );
        }
    }

    #[tokio::test]
    async fn should_send_an_address_registration_elsewhere_when_its_keys_are_taken() {
        let wallet = wallet_answering(
            Backend::new(full()),
            Some(Some(registered_with_our_master())),
        )
        .await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        let refused = register_from_addresses_at(&wallet, slot.identity_index, slot.holder).await;

        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::IdentitySlotTakenOnChain {
                    identity_index: 0,
                    ..
                })
            ),
            "{refused:?}"
        );
    }

    #[tokio::test]
    async fn should_mark_a_claim_submitted_durably_before_it_is_sent() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };

        wallet
            .identity()
            .submit_claim_lease(slot.identity_index, invitation)
            .await
            .unwrap();

        assert_eq!(
            backend.lease_writes().last().unwrap().leases[&invitation].phase,
            SlotLeasePhase::Submitted
        );
        let retried = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        assert_eq!(retried.phase, SlotLeasePhase::Submitted);
    }

    /// A retried claim learns that an earlier attempt submitted the lease,
    /// so a refusal of the retry alone does not give that slot back.
    #[tokio::test]
    async fn should_tell_a_retried_claim_its_lease_was_submitted_earlier() {
        let wallet = wallet(Backend::new(full())).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };

        let first = wallet
            .identity()
            .submit_claim_lease(slot.identity_index, invitation)
            .await
            .unwrap();
        let retried = wallet
            .identity()
            .submit_claim_lease(slot.identity_index, invitation)
            .await
            .unwrap();

        assert!(first);
        assert!(!retried);
    }

    #[tokio::test]
    async fn should_keep_a_claim_reserved_when_its_submission_is_not_stored() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        *backend.fail.lock().unwrap() = true;

        let refused = wallet
            .identity()
            .submit_claim_lease(slot.identity_index, invitation)
            .await;

        assert!(refused.is_err());
        *backend.fail.lock().unwrap() = false;
        let retried = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        assert_eq!(retried.phase, SlotLeasePhase::Reserved);
    }

    /// A lease change whose flush failed transiently may still be written
    /// by a later flush, so memory keeps it: a new claim keeps its index, a
    /// submission stays `Submitted`, and a release leaves the slot free.
    #[tokio::test]
    async fn should_keep_a_lease_change_a_later_flush_may_still_write() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        *backend.flush_fails_transiently.lock().unwrap() = true;

        let refused = wallet.identity().acquire_resolved_slot(claim(1)).await;
        assert!(refused.is_err(), "{refused:?}");
        let next = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        assert_eq!(next.identity_index, 1, "the claim's buffered lease keeps 0");

        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        assert!(wallet
            .identity()
            .submit_claim_lease(0, invitation)
            .await
            .is_err());
        let held = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        assert_eq!(held.phase, SlotLeasePhase::Submitted);

        let reserved = wallet.identity().acquire_resolved_slot(claim(2)).await;
        assert!(reserved.is_err());
        let holder = SlotHolder::InvitationClaim {
            invitation: OutPoint::new(Txid::from_byte_array([2; 32]), 0),
        };
        assert!(wallet
            .identity()
            .release_identity_slot(holder)
            .await
            .is_err());
        let wm = wallet.identity().wallet_manager.read().await;
        let manager = &wm
            .get_wallet_info(&wallet.wallet_id())
            .unwrap()
            .identity_manager;
        assert_eq!(manager.slot_state(&wallet.wallet_id(), 2), SlotState::Free);
    }

    #[tokio::test]
    async fn should_give_a_claim_refused_before_it_could_execute_back_to_the_host() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        wallet
            .identity()
            .submit_claim_lease(slot.identity_index, invitation)
            .await
            .unwrap();

        // A failed store keeps it held.
        *backend.fail.lock().unwrap() = true;
        assert!(wallet
            .identity()
            .unsubmit_claim_lease(slot.identity_index, invitation)
            .await
            .is_err());
        *backend.fail.lock().unwrap() = false;
        assert!(matches!(
            wallet.identity().release_identity_slot(slot.holder).await,
            Err(PlatformWalletError::IdentitySlotClaimSubmitted { .. })
        ));

        wallet
            .identity()
            .unsubmit_claim_lease(slot.identity_index, invitation)
            .await
            .unwrap();

        assert_eq!(
            backend.lease_writes().last().unwrap().leases[&invitation].phase,
            SlotLeasePhase::Reserved
        );
        assert_eq!(
            wallet
                .identity()
                .release_identity_slot(slot.holder)
                .await
                .unwrap(),
            Some(slot.identity_index)
        );
    }

    #[tokio::test]
    async fn should_store_the_end_of_a_submitted_claims_lease_when_its_slot_is_filled() {
        let backend = Backend::new(full());
        let wallet = wallet(backend.clone()).await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(1) else {
            unreachable!()
        };
        wallet
            .identity()
            .submit_claim_lease(slot.identity_index, invitation)
            .await
            .unwrap();

        wallet
            .identity()
            .record_registered_identity(registered_with_our_master(), slot.identity_index)
            .await
            .unwrap();

        let stored = backend.lease_writes();
        let last = stored.last().unwrap();
        assert!(last.leases.is_empty());
        assert_eq!(last.released, [invitation].into());
        assert_eq!(
            wallet
                .identity()
                .release_identity_slot(slot.holder)
                .await
                .unwrap(),
            None,
            "the slot is the identity's; nothing is left to refuse"
        );
    }

    #[tokio::test]
    async fn should_refuse_to_submit_a_claim_at_a_slot_it_does_not_lease() {
        let wallet = wallet(Backend::new(full())).await;
        wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let ResolvedSlotRequest::InvitationClaim { invitation } = claim(2) else {
            unreachable!()
        };

        let refused = wallet.identity().submit_claim_lease(0, invitation).await;

        assert!(matches!(
            refused,
            Err(PlatformWalletError::IdentitySlotNotHeld { identity_index: 0 })
        ));
    }

    /// A resume whose lock is not a tracked registration lock of the index
    /// settles nothing, even with the slot's keys registered: no placement,
    /// and a claim's lease on that index stays.
    #[tokio::test]
    async fn should_settle_nothing_for_a_resume_its_lock_does_not_authorize() {
        let wallet = wallet_answering(
            Backend::new(full()),
            Some(Some(registered_with_our_master())),
        )
        .await;
        let leased = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();
        let untracked = OutPoint::new(Txid::from_byte_array([7; 32]), 0);
        let other_index = OutPoint::new(Txid::from_byte_array([8; 32]), 0);
        track_registration_lock_at(&wallet, other_index, 1).await;

        assert_eq!(leased.identity_index, 0);
        for (out_point, expected_untracked) in [(untracked, true), (other_index, false)] {
            let refused = resume_at(&wallet, out_point).await;
            assert_eq!(
                matches!(refused, Err(PlatformWalletError::AssetLockNotTracked(_))),
                expected_untracked,
                "{refused:?}"
            );
            assert!(refused.is_err());
        }
        let wm = wallet.identity().wallet_manager.read().await;
        let manager = &wm
            .get_wallet_info(&wallet.wallet_id())
            .unwrap()
            .identity_manager;
        assert!(matches!(
            manager.slot_state(&wallet.wallet_id(), leased.identity_index),
            SlotState::Leased(_)
        ));
    }

    /// An identity on Platform holding [`master_keys`]'s master key, plus a
    /// key this wallet did not derive.
    fn registered_with_our_master() -> Identity {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{KeyType, Purpose, SecurityLevel};
        let mut keys = master_keys();
        keys.insert(
            1,
            IdentityPublicKeyV0 {
                id: 1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::HIGH,
                key_type: KeyType::ECDSA_HASH160,
                data: vec![2; 20].into(),
                ..Default::default()
            }
            .into(),
        );
        Identity::V0(dpp::identity::v0::IdentityV0 {
            id: Identifier::from([5; 32]),
            public_keys: keys,
            balance: 0,
            revision: 1,
        })
    }

    /// Track a registration lock at `out_point` for slot 0.
    async fn track_registration_lock(wallet: &PlatformWallet, out_point: OutPoint) {
        track_registration_lock_at(wallet, out_point, 0).await;
    }

    /// Track a registration lock at `out_point` for slot `identity_index`.
    async fn track_registration_lock_at(
        wallet: &PlatformWallet,
        out_point: OutPoint,
        identity_index: RegistrationIndex,
    ) {
        let mut wm = wallet.identity().wallet_manager.write().await;
        let info = wm.get_wallet_info_mut(&wallet.wallet_id()).unwrap();
        info.tracked_asset_locks.insert(
            out_point,
            TrackedAssetLock {
                operation_id: None,
                out_point,
                transaction: Transaction {
                    version: 3,
                    lock_time: 0,
                    input: vec![],
                    output: vec![],
                    special_transaction_payload: None,
                },
                account_index: 0,
                funding_type: AssetLockFundingType::IdentityRegistration,
                identity_index,
                amount: 1,
                status: AssetLockStatus::Broadcast,
                proof: None,
                shielded_recipient: None,
            },
        );
    }

    /// Resume the registration lock at `out_point` at slot 0.
    async fn resume_at(
        wallet: &PlatformWallet,
        out_point: OutPoint,
    ) -> Result<dpp::identity::Identity, PlatformWalletError> {
        let asset_lock_signer = crate::test_support::fund_test_platform_wallet(
            &wallet.identity().wallet_manager,
            &wallet.wallet_id(),
            1_000_000,
        )
        .await;
        wallet
            .identity()
            .register_identity_with_funding(
                crate::wallet::asset_lock::AssetLockFunding::FromExistingAssetLock {
                    out_point,
                    consume_invitation_voucher: false,
                },
                0,
                None,
                master_keys(),
                &UnusedSigner,
                &asset_lock_signer,
                None,
            )
            .await
    }

    /// An earlier submit of the lock landed (its answer was lost): the
    /// resume finds the lock's own identity at the slot, records it and
    /// marks the lock consumed, without submitting again.
    #[tokio::test]
    async fn should_resume_a_landed_registration_lock_as_its_identity() {
        let out_point = OutPoint::new(Txid::from_byte_array([3; 32]), 0);
        let mut landed = registered_with_our_master();
        dpp::identity::accessors::IdentitySettersV0::set_id(
            &mut landed,
            dash_sdk::platform::dashpay::identity_id_for_out_point(out_point),
        );
        let wallet = wallet_answering(Backend::new(full()), Some(Some(landed.clone()))).await;
        track_registration_lock(&wallet, out_point).await;

        let resumed = resume_at(&wallet, out_point).await.unwrap();

        assert_eq!(resumed.id(), landed.id());
        let wm = wallet.identity().wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet.wallet_id()).unwrap();
        assert_eq!(
            info.identity_manager.slot_state(&wallet.wallet_id(), 0),
            SlotState::Occupied(landed.id())
        );
        assert!(info.tracked_asset_locks[&out_point].status.is_consumed());
    }

    /// Another identity of this wallet holds the slot's keys: resuming would
    /// be refused for them and charge the lock a penalty again. The identity
    /// is recorded instead, and the lock stays for a top-up.
    #[tokio::test]
    async fn should_not_resume_a_registration_lock_whose_keys_another_identity_holds() {
        let out_point = OutPoint::new(Txid::from_byte_array([3; 32]), 0);
        let wallet = wallet_answering(
            Backend::new(full()),
            Some(Some(registered_with_our_master())),
        )
        .await;
        track_registration_lock(&wallet, out_point).await;

        let refused = resume_at(&wallet, out_point).await;

        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::IdentitySlotTakenOnChain { identity_index: 0, identity_id })
                    if identity_id == Identifier::from([5; 32])
            ),
            "{refused:?}"
        );
        let wm = wallet.identity().wallet_manager.read().await;
        let info = wm.get_wallet_info(&wallet.wallet_id()).unwrap();
        assert!(!info.tracked_asset_locks[&out_point].status.is_consumed());
    }

    #[tokio::test]
    async fn should_record_an_identity_already_at_the_slot_and_send_the_host_elsewhere() {
        let backend = Backend::new(full());
        let wallet =
            wallet_answering(backend.clone(), Some(Some(registered_with_our_master()))).await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        let refused = register_at(&wallet, slot.identity_index, Some(slot.holder)).await;

        let id = Identifier::from([5; 32]);
        assert!(
            matches!(
                refused,
                Err(PlatformWalletError::IdentitySlotTakenOnChain { identity_index: 0, identity_id }) if identity_id == id
            ),
            "{refused:?}"
        );
        assert_eq!(tracked_locks(&wallet).await, 0);
        {
            let wm = wallet.identity().wallet_manager.read().await;
            let manager = &wm
                .get_wallet_info(&wallet.wallet_id())
                .unwrap()
                .identity_manager;
            assert_eq!(
                manager.slot_state(&wallet.wallet_id(), 0),
                SlotState::Occupied(id)
            );
            assert!(manager
                .leased_slot(&wallet.wallet_id(), &slot.holder)
                .is_none());
        }
        // The derived master key carries its breadcrumb; the other is
        // watch-only.
        {
            let stored = backend.stores.lock().unwrap();
            let rows = stored
                .iter()
                .filter_map(|cs| cs.identity_keys.as_ref())
                .next_back()
                .unwrap();
            assert!(rows.upserts[&(id, 0)].derivation_indices.is_some());
            assert!(rows.upserts[&(id, 1)].derivation_indices.is_none());
        }
        // The next slot is the one past it.
        let next = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();
        assert_eq!(next.identity_index, 1);
    }

    #[tokio::test]
    async fn should_refuse_a_registration_when_platform_cannot_be_asked_about_its_slot() {
        // The node is given no answer for the lookup: the check fails closed.
        let wallet = wallet(Backend::new(full())).await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        let refused = register_at(&wallet, slot.identity_index, Some(slot.holder)).await;

        assert!(
            matches!(refused, Err(PlatformWalletError::InvalidIdentityData(_))),
            "{refused:?}"
        );
        assert_eq!(tracked_locks(&wallet).await, 0);
    }

    #[tokio::test]
    async fn should_record_nothing_when_the_found_identity_is_not_the_expected_one() {
        let wallet = wallet_answering(
            Backend::new(full()),
            Some(Some(registered_with_our_master())),
        )
        .await;
        let slot = wallet
            .identity()
            .acquire_resolved_slot(claim(1))
            .await
            .unwrap();

        let taken = wallet
            .identity()
            .slot_taken_on_chain(
                slot.identity_index,
                &master_keys(),
                Some(Identifier::from([6; 32])),
            )
            .await
            .unwrap();

        assert!(taken.is_none());
        let wm = wallet.identity().wallet_manager.read().await;
        let manager = &wm
            .get_wallet_info(&wallet.wallet_id())
            .unwrap()
            .identity_manager;
        assert!(manager
            .leased_slot(&wallet.wallet_id(), &slot.holder)
            .is_some());
        assert_eq!(
            manager.slot_state(&wallet.wallet_id(), 0),
            SlotState::Leased(SlotLease {
                holder: slot.holder,
                phase: SlotLeasePhase::Reserved,
            })
        );
    }

    #[tokio::test]
    async fn should_go_on_when_no_identity_holds_the_slots_keys() {
        let wallet = wallet_answering(Backend::new(full()), Some(None)).await;
        let slot = wallet
            .identity()
            .acquire_identity_slot(SlotRequest::Registration)
            .await
            .unwrap();

        let taken = wallet
            .identity()
            .slot_taken_on_chain(slot.identity_index, &master_keys(), None)
            .await
            .unwrap();

        assert!(taken.is_none());
    }

    #[tokio::test]
    async fn should_require_a_slot_to_reclaim_a_voucher_but_not_to_resume_a_registration_lock() {
        let wallet = wallet(Backend::new(full())).await;
        let asset_lock_signer = crate::test_support::fund_test_platform_wallet(
            &wallet.identity().wallet_manager,
            &wallet.wallet_id(),
            1_000_000,
        )
        .await;
        let out_point = OutPoint::new(Txid::from_byte_array([3; 32]), 0);

        let mut refusals = Vec::new();
        for consume_invitation_voucher in [true, false] {
            refusals.push(
                wallet
                    .identity()
                    .register_identity_with_funding(
                        crate::wallet::asset_lock::AssetLockFunding::FromExistingAssetLock {
                            out_point,
                            consume_invitation_voucher,
                        },
                        0,
                        None,
                        master_keys(),
                        &UnusedSigner,
                        &asset_lock_signer,
                        None,
                    )
                    .await,
            );
        }

        assert!(
            matches!(
                refusals[0],
                Err(PlatformWalletError::IdentitySlotNotHeld { identity_index: 0 })
            ),
            "{:?}",
            refusals[0]
        );
        // The untracked lock is refused by the resolver, not for its slot.
        assert!(
            !matches!(
                refusals[1],
                Err(PlatformWalletError::IdentitySlotNotHeld { .. })
            ),
            "{:?}",
            refusals[1]
        );
    }

    /// [`IdentityWallet::resolve_submitted_claim_leases`]: what Platform holds
    /// at a claim's identity id decides its lease. The node is a mock that
    /// answers only what each test gives it.
    mod claim_lease_resolution {
        use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
        use dpp::identity::{KeyType, Purpose, SecurityLevel};
        use dpp::platform_value::BinaryData;

        use key_wallet::bip32::ExtendedPrivKey;

        use super::*;
        use crate::wallet::identity::network::identity_handle::{
            derive_ecdsa_identity_auth_keypair_from_master, MASTER_KEY_INDEX,
        };

        fn invitation() -> OutPoint {
            OutPoint::new(Txid::from_byte_array([1; 32]), 0)
        }

        /// The identity a claim of [`invitation`] creates, with `master_key`
        /// as its key 0.
        fn claimed(master_key: Vec<u8>) -> Identity {
            Identity::V0(dpp::identity::v0::IdentityV0 {
                id: identity_id_for_out_point(invitation()),
                public_keys: [(
                    0,
                    IdentityPublicKeyV0 {
                        id: 0,
                        purpose: Purpose::AUTHENTICATION,
                        security_level: SecurityLevel::MASTER,
                        key_type: KeyType::ECDSA_SECP256K1,
                        data: BinaryData::new(master_key),
                        ..Default::default()
                    }
                    .into(),
                )]
                .into(),
                balance: 0,
                revision: 1,
            })
        }

        /// The master xpriv of the wallet from `[9; 64]`. Its seed is not in
        /// process (as in the hosts), so the host resolves this.
        fn master() -> ExtendedPrivKey {
            ExtendedPrivKey::new_master(Network::Testnet, &[9; 64]).unwrap()
        }

        /// The master key that wallet derives at index 0.
        fn our_master_key() -> Vec<u8> {
            derive_ecdsa_identity_auth_keypair_from_master(
                &master(),
                Network::Testnet,
                0,
                MASTER_KEY_INDEX,
            )
            .unwrap()
            .public_key
            .to_vec()
        }

        /// A wallet whose node holds `at_claim_id` at the claim's identity id
        /// (and answers the load's master-key lookup with it), with
        /// [`invitation`]'s claim leasing slot 0 in `phase`.
        async fn wallet_with_claim(
            backend: Arc<Backend>,
            at_claim_id: Option<Option<Identity>>,
            phase: SlotLeasePhase,
        ) -> (Arc<PlatformWallet>, IdentitySlot) {
            struct Events;
            impl EventHandler for Events {}
            impl PlatformEventHandler for Events {}
            let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
            if let Some(at_claim_id) = at_claim_id {
                sdk.mock()
                    .expect_fetch(identity_id_for_out_point(invitation()), at_claim_id.clone())
                    .await
                    .unwrap();
                if let Some(found) = at_claim_id {
                    let hash = found.public_keys()[&0].public_key_hash().unwrap();
                    sdk.mock()
                        .expect_fetch(PublicKeyHash(hash), Some(found))
                        .await
                        .unwrap();
                }
            }
            let manager = PlatformWalletManager::new(Arc::new(sdk), backend, Arc::new(Events));
            let wallet = manager
                .create_wallet_from_seed_bytes(
                    Network::Testnet,
                    &[9; 64],
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
                .unwrap();
            let slot = wallet
                .identity()
                .acquire_resolved_slot(ResolvedSlotRequest::InvitationClaim {
                    invitation: invitation(),
                })
                .await
                .unwrap();
            if phase == SlotLeasePhase::Submitted {
                wallet
                    .identity()
                    .submit_claim_lease(slot.identity_index, invitation())
                    .await
                    .unwrap();
            }
            (wallet, slot)
        }

        async fn slot_state(wallet: &PlatformWallet, identity_index: u32) -> SlotState {
            let wm = wallet.identity().wallet_manager.read().await;
            wm.get_wallet_info(&wallet.wallet_id())
                .unwrap()
                .identity_manager
                .slot_state(&wallet.wallet_id(), identity_index)
        }

        fn ended_last(backend: &Backend) -> bool {
            backend
                .lease_writes()
                .last()
                .is_some_and(|last| last.released.contains(&invitation()))
        }

        #[tokio::test]
        async fn should_record_a_landed_claim_and_store_the_end_of_its_lease() {
            let backend = Backend::new(full());
            let landed = claimed(our_master_key());
            let (wallet, slot) = wallet_with_claim(
                backend.clone(),
                Some(Some(landed.clone())),
                SlotLeasePhase::Submitted,
            )
            .await;

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution.landed, 1, "{resolution:?}");
            assert_eq!(
                slot_state(&wallet, 0).await,
                SlotState::Occupied(landed.id())
            );
            assert!(ended_last(&backend));
            assert_eq!(
                wallet
                    .identity()
                    .release_identity_slot(slot.holder)
                    .await
                    .unwrap(),
                None
            );
        }

        #[tokio::test]
        async fn should_end_the_lease_of_a_claim_another_claimant_won() {
            let backend = Backend::new(full());
            let (wallet, _) = wallet_with_claim(
                backend.clone(),
                Some(Some(claimed(vec![2; 33]))),
                SlotLeasePhase::Submitted,
            )
            .await;

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution.ended, 1, "{resolution:?}");
            assert_eq!(slot_state(&wallet, 0).await, SlotState::Free);
            assert!(ended_last(&backend));
        }

        #[tokio::test]
        async fn should_hold_a_claim_with_no_identity_yet_or_no_answer() {
            for at_claim_id in [Some(None), None] {
                let backend = Backend::new(full());
                let (wallet, _) = wallet_with_claim(
                    backend.clone(),
                    at_claim_id.clone(),
                    SlotLeasePhase::Submitted,
                )
                .await;
                let writes = backend.lease_writes().len();

                let resolution = wallet
                    .identity()
                    .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                    .await
                    .unwrap();

                assert_eq!(resolution.held, 1, "{at_claim_id:?}: {resolution:?}");
                assert!(matches!(
                    slot_state(&wallet, 0).await,
                    SlotState::Leased(SlotLease {
                        phase: SlotLeasePhase::Submitted,
                        ..
                    })
                ));
                assert_eq!(backend.lease_writes().len(), writes, "nothing stored");
            }
        }

        #[tokio::test]
        async fn should_hold_a_found_claim_it_has_no_key_to_tell_the_owner_of() {
            let backend = Backend::new(full());
            let (wallet, _) = wallet_with_claim(
                backend.clone(),
                Some(Some(claimed(our_master_key()))),
                SlotLeasePhase::Submitted,
            )
            .await;

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Unavailable)
                .await
                .unwrap();

            assert_eq!(resolution.held, 1);
            assert!(resolution.needs_key);
            assert!(matches!(slot_state(&wallet, 0).await, SlotState::Leased(_)));
        }

        #[tokio::test]
        async fn should_keep_the_lease_when_its_end_is_not_stored() {
            let backend = Backend::new(full());
            let (wallet, _) = wallet_with_claim(
                backend.clone(),
                Some(Some(claimed(vec![2; 33]))),
                SlotLeasePhase::Submitted,
            )
            .await;
            *backend.fail.lock().unwrap() = true;

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution.held, 1, "{resolution:?}");
            assert!(matches!(slot_state(&wallet, 0).await, SlotState::Leased(_)));
        }

        /// Replay `wallet`'s stored claim leases, as a restart restores them.
        async fn restore_claim_leases(wallet: &PlatformWallet) {
            let mut wm = wallet.identity().wallet_manager.write().await;
            let manager = &mut wm
                .get_wallet_info_mut(&wallet.wallet_id())
                .unwrap()
                .identity_manager;
            let stored = manager.claim_lease_changeset(&wallet.wallet_id(), invitation());
            manager.apply_claim_lease_changeset(wallet.wallet_id(), stored);
        }

        /// A `Reserved` claim this process leased is a live claim's: settling
        /// leaves it, so the claim can still submit.
        #[tokio::test]
        async fn should_leave_a_reserved_claim_lease_this_process_took() {
            let (wallet, slot) =
                wallet_with_claim(Backend::new(full()), None, SlotLeasePhase::Reserved).await;

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution, ClaimLeaseResolution::default());
            assert!(matches!(
                slot_state(&wallet, slot.identity_index).await,
                SlotState::Leased(_)
            ));
            assert!(wallet
                .identity()
                .submit_claim_lease(slot.identity_index, invitation())
                .await
                .unwrap());
        }

        /// Nothing was sent for a `Reserved` claim, and the process that
        /// leased it is gone: its lease ends, stored durably.
        #[tokio::test]
        async fn should_end_a_reserved_claim_lease_an_earlier_process_left() {
            let backend = Backend::new(full());
            let (wallet, slot) =
                wallet_with_claim(backend.clone(), None, SlotLeasePhase::Reserved).await;
            restore_claim_leases(&wallet).await;

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution.ended, 1, "{resolution:?}");
            assert_eq!(
                slot_state(&wallet, slot.identity_index).await,
                SlotState::Free
            );
            assert!(ended_last(&backend));
        }

        /// A wallet whose node holds `at_lock_id` at the identity id of the
        /// registration lock at `lock` (and answers the load's master-key
        /// lookup with it), with that lock tracked for slot 0.
        async fn wallet_with_registration_lock(
            lock: OutPoint,
            at_lock_id: Option<Identity>,
        ) -> Arc<PlatformWallet> {
            struct Events;
            impl EventHandler for Events {}
            impl PlatformEventHandler for Events {}
            let mut sdk = dash_sdk::SdkBuilder::new_mock().build().unwrap();
            sdk.mock()
                .expect_fetch(identity_id_for_out_point(lock), at_lock_id.clone())
                .await
                .unwrap();
            if let Some(found) = at_lock_id {
                let hash = found.public_keys()[&0].public_key_hash().unwrap();
                sdk.mock()
                    .expect_fetch(PublicKeyHash(hash), Some(found))
                    .await
                    .unwrap();
            }
            let manager =
                PlatformWalletManager::new(Arc::new(sdk), Backend::new(full()), Arc::new(Events));
            let wallet = manager
                .create_wallet_from_seed_bytes(
                    Network::Testnet,
                    &[9; 64],
                    WalletAccountCreationOptions::Default,
                    Some(0),
                )
                .await
                .unwrap();
            track_registration_lock(&wallet, lock).await;
            wallet
        }

        /// A registration that landed while its answer, or the record of its
        /// identity, was lost: startup records the identity and marks the
        /// lock consumed.
        #[tokio::test]
        async fn should_record_the_identity_a_registration_lock_created() {
            let lock = OutPoint::new(Txid::from_byte_array([4; 32]), 0);
            let mut landed = claimed(our_master_key());
            dpp::identity::accessors::IdentitySettersV0::set_id(
                &mut landed,
                identity_id_for_out_point(lock),
            );
            let wallet = wallet_with_registration_lock(lock, Some(landed.clone())).await;

            let resolution = wallet
                .identity()
                .resolve_registration_locks(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution.landed, 1, "{resolution:?}");
            assert_eq!(
                slot_state(&wallet, 0).await,
                SlotState::Occupied(landed.id())
            );
            let wm = wallet.identity().wallet_manager.read().await;
            assert!(wm
                .get_wallet_info(&wallet.wallet_id())
                .unwrap()
                .tracked_asset_locks[&lock]
                .status
                .is_consumed());
        }

        #[tokio::test]
        async fn should_leave_a_registration_lock_that_created_nothing() {
            let lock = OutPoint::new(Txid::from_byte_array([4; 32]), 0);
            let wallet = wallet_with_registration_lock(lock, None).await;

            let resolution = wallet
                .identity()
                .resolve_registration_locks(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution, ClaimLeaseResolution::default());
            assert_eq!(slot_state(&wallet, 0).await, SlotState::Free);
            let wm = wallet.identity().wallet_manager.read().await;
            assert!(!wm
                .get_wallet_info(&wallet.wallet_id())
                .unwrap()
                .tracked_asset_locks[&lock]
                .status
                .is_consumed());
        }

        /// A lease restored from the store for a slot an identity filled
        /// since it was stored ends, whatever its phase.
        #[tokio::test]
        async fn should_end_a_restored_lease_whose_slot_is_filled() {
            let backend = Backend::new(full());
            let (wallet, slot) =
                wallet_with_claim(backend.clone(), None, SlotLeasePhase::Submitted).await;
            {
                let mut wm = wallet.identity().wallet_manager.write().await;
                let manager = &mut wm
                    .get_wallet_info_mut(&wallet.wallet_id())
                    .unwrap()
                    .identity_manager;
                manager
                    .place_in_slot_unstored(registered_with_our_master(), 0, wallet.wallet_id())
                    .unwrap();
                // As a restore replays the stored lease onto the filled slot.
                manager.restore_slot_lease(
                    wallet.wallet_id(),
                    0,
                    SlotLease {
                        holder: slot.holder,
                        phase: SlotLeasePhase::Submitted,
                    },
                );
            }

            let resolution = wallet
                .identity()
                .resolve_submitted_claim_leases(InvitationClaimantKey::Master(&master()))
                .await
                .unwrap();

            assert_eq!(resolution.ended, 1, "{resolution:?}");
            assert!(ended_last(&backend));
            assert_eq!(
                wallet
                    .identity()
                    .release_identity_slot(slot.holder)
                    .await
                    .unwrap(),
                None
            );
        }
    }
}
