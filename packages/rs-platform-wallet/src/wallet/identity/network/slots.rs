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
    RegistrationIndex, SlotHolder, SlotLease, SlotLeasePhase, SlotState,
};
use crate::wallet::platform_wallet::PlatformWalletInfo;

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
    /// the one its earlier attempt may have landed at.
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
                        manager.release_identity_slot(&self.wallet_id, &holder);
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
    /// A claim whose lease is `Submitted` is refused: its claim may have
    /// reached Platform, and only its outcome frees the slot. A claim's
    /// release is stored durably; on a failed store the lease stays.
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
        let SlotHolder::InvitationClaim { invitation } = holder else {
            return Ok(manager.release_identity_slot(&self.wallet_id, &holder));
        };
        if leased.phase == SlotLeasePhase::Submitted {
            return Err(PlatformWalletError::IdentitySlotClaimSubmitted {
                identity_index: leased.identity_index,
            });
        }
        manager.release_identity_slot(&self.wallet_id, &holder);
        let changeset = PlatformWalletChangeSet {
            identity_slot_leases: Some(manager.claim_lease_changeset(&self.wallet_id, invitation)),
            ..Default::default()
        };
        if let Err(not_durable) = self.persister.store_durably(changeset) {
            manager.restore_slot_lease(
                self.wallet_id,
                leased.identity_index,
                SlotLease {
                    holder,
                    phase: leased.phase,
                },
            );
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

    /// Move `invitation`'s claim lease on slot `identity_index` to
    /// `Submitted`, stored durably, before its claim is sent: from then on
    /// only the claim's outcome frees the slot. Refused with
    /// [`PlatformWalletError::IdentitySlotNotHeld`] unless the claim leases
    /// that slot; on a failed store the lease keeps its phase.
    pub(crate) async fn submit_claim_lease(
        &self,
        identity_index: RegistrationIndex,
        invitation: OutPoint,
    ) -> Result<(), PlatformWalletError> {
        self.set_claim_lease_phase(identity_index, invitation, SlotLeasePhase::Submitted)
            .await
    }

    /// Move `invitation`'s claim lease on slot `identity_index` back to
    /// `Reserved`, stored durably: its claim was refused in a way that can
    /// never execute, so the host may retry or release it. Refused like
    /// [`Self::submit_claim_lease`]; on a failed store the lease stays
    /// `Submitted`.
    pub(crate) async fn unsubmit_claim_lease(
        &self,
        identity_index: RegistrationIndex,
        invitation: OutPoint,
    ) -> Result<(), PlatformWalletError> {
        self.set_claim_lease_phase(identity_index, invitation, SlotLeasePhase::Reserved)
            .await
    }

    async fn set_claim_lease_phase(
        &self,
        identity_index: RegistrationIndex,
        invitation: OutPoint,
        phase: SlotLeasePhase,
    ) -> Result<(), PlatformWalletError> {
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
            return Ok(());
        }
        manager.set_slot_lease_phase(&self.wallet_id, &holder, phase);
        let changeset = PlatformWalletChangeSet {
            identity_slot_leases: Some(manager.claim_lease_changeset(&self.wallet_id, invitation)),
            ..Default::default()
        };
        if let Err(not_durable) = self.persister.store_durably(changeset) {
            manager.set_slot_lease_phase(&self.wallet_id, &holder, lease.phase);
            return Err(self.persister.classify_not_durable(not_durable));
        }
        Ok(())
    }

    /// Before anything is funded at `identity_index`: the identity Platform
    /// holds under `keys_map[0]` (the master key derived there), if any.
    /// Found, and `expected` (when given) is its id, it is recorded in slot
    /// `identity_index`, releasing `holder`'s lease in the same write, and
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
        holder: &SlotHolder,
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
            Some(holder),
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
    /// there, in that slot, releasing `holder`'s lease in the same durable
    /// write ([`IdentityManager::place_in_slot`](crate::wallet::identity::IdentityManager::place_in_slot)).
    pub(crate) async fn record_registered_identity(
        &self,
        identity: Identity,
        identity_index: RegistrationIndex,
        holder: &SlotHolder,
    ) -> Result<(), PlatformWalletError> {
        let keys = derived_keys(&identity, self.wallet_id, identity_index);
        let mut wm = self.wallet_manager.write().await;
        let info = self.current_wallet_info_mut(&mut wm)?;
        info.identity_manager.place_in_slot(
            identity,
            identity_index,
            self.wallet_id,
            Some(holder),
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
                && lock.status != AssetLockStatus::Consumed
        })
        .map(|lock| lock.identity_index)
        .collect()
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
        stores: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl Backend {
        fn new(capabilities: PersistenceCapabilities) -> Arc<Self> {
            Arc::new(Self {
                capabilities,
                fail: Mutex::new(false),
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
            true
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
                &slot.holder,
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
            .slot_taken_on_chain(slot.identity_index, &master_keys(), &slot.holder, None)
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
}
