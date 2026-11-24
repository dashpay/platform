//! Construction & lifecycle for [`IdentityManager`].
//!
//! Adds, removes, and looks up the HD index for managed identities.
//! Every mutation here persists a corresponding
//! [`IdentityChangeSet`](crate::changeset::IdentityChangeSet) via
//! the supplied [`WalletPersister`], except
//! [`IdentityManager::add_identity_unstored`], whose caller stores.

use super::{IdentityLocation, IdentityManager};
use crate::changeset::{
    IdentityChangeSet, IdentityEntry, KeyWithBreadcrumb, PlatformWalletChangeSet,
};
use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

/// Where the keys of a newly registered identity came from, which decides
/// the derivation breadcrumb [`IdentityManager::record_new_identity`] stores
/// with each of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NewIdentityKeyOrigin {
    /// Each key was derived from the wallet seed at the identity's own index
    /// with the key's id as the key index, so it is stored with the
    /// breadcrumb `(wallet_id, identity_index, key_id)`.
    DerivedAtKeyId,
    /// The flow does not record where the keys came from: they are stored
    /// without a breadcrumb, as [`IdentityManager::add_identity`] stores
    /// them.
    Unrecorded,
}

impl IdentityManager {
    /// Create a new identity manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a wallet-owned identity at the given BIP-9 HD identity index.
    ///
    /// The caller must already know which wallet owns this identity —
    /// pass `wallet_id` explicitly so we don't have to look it up out of
    /// some ambient state. The identity's `wallet_id` and
    /// `identity_index` fields are denormalized from the bucket keys
    /// onto the value at insert time.
    ///
    /// Errors if an identity with the same id already exists in either
    /// bucket.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn add_identity(
        &mut self,
        identity: Identity,
        identity_index: u32,
        wallet_id: WalletId,
        persister: &WalletPersister,
    ) -> Result<(), PlatformWalletError> {
        let identity_id = identity.id();
        self.add_identity_unstored(identity, identity_index, wallet_id)?;
        let managed_identity = self
            .managed_identity_mut(&identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(identity_id))?;

        let entry = IdentityEntry::from_managed(managed_identity);
        // No `key_storage` to snapshot here anymore — the field is
        // dropped from `ManagedIdentity`. Public keys still arrive
        // via the separate `IdentityKeysChangeSet` path.
        let keys_cs = managed_identity.keys_snapshot_changeset();

        let mut id_cs = IdentityChangeSet::default();
        id_cs.identities.insert(identity_id, entry);

        let cs = PlatformWalletChangeSet {
            identities: Some(id_cs),
            identity_keys: if keys_cs.upserts.is_empty() {
                None
            } else {
                Some(keys_cs)
            },
            ..Default::default()
        };

        if let Err(e) = persister.store(cs) {
            tracing::error!("Failed to persist changeset: {}", e);
        }

        Ok(())
    }

    /// Record an identity Platform has just accepted: add it at its HD slot
    /// and store it with its keys as one changeset, each key carrying the
    /// derivation breadcrumb `key_origin` says it has.
    ///
    /// Best-effort, and the only owner of that rule: the identity exists on
    /// chain whatever happens here, so every failure (the id already held,
    /// the store) is logged under `flow` and none is returned. A returned
    /// error would make the calling flow report a failed registration and
    /// skip what it still owes after acceptance (consuming the asset lock,
    /// reconciling the funding addresses).
    ///
    /// A store the backend refused as transient, attesting it kept nothing
    /// (`store_transient_is_reissuable`), is issued once more. An identity
    /// still not stored stays in memory for this session only: an identity
    /// discovery scan finds it again, and a launch that already has another
    /// identity on file does not run one by itself.
    ///
    /// A failed add records no keys either: `IdentityAlreadyExists` can mean
    /// an out-of-wallet entry, and stamping a wallet breadcrumb on its keys
    /// would contradict the manager's location index.
    pub(crate) fn record_new_identity(
        &mut self,
        identity: Identity,
        identity_index: u32,
        wallet_id: WalletId,
        key_origin: NewIdentityKeyOrigin,
        flow: &str,
        persister: &WalletPersister,
    ) {
        let identity_id = identity.id();
        let key_rows = |identity: &Identity| -> Vec<KeyWithBreadcrumb> {
            identity
                .public_keys()
                .values()
                .map(|key| KeyWithBreadcrumb {
                    key: key.clone(),
                    breadcrumb: match key_origin {
                        NewIdentityKeyOrigin::DerivedAtKeyId => {
                            Some((wallet_id, identity_index, key.id()))
                        }
                        NewIdentityKeyOrigin::Unrecorded => None,
                    },
                })
                .collect()
        };

        if let Err(e) = self.add_identity_unstored(identity, identity_index, wallet_id) {
            tracing::warn!(
                flow,
                %identity_id,
                identity_index,
                error = %e,
                "identity accepted by Platform but not added to the local manager; \
                 an identity discovery scan adds it"
            );
            return;
        }
        let Some(managed) = self.managed_identity_mut(&identity_id) else {
            return;
        };
        let keys = key_rows(&managed.identity);
        let stored = match managed.store_keys_with_snapshot(keys, persister) {
            Ok(()) => Ok(()),
            Err(e) => match persister.classify_store_failure(e) {
                // The one changeset is all that puts the identity on disk,
                // so a refusal the backend attests kept nothing of it is
                // issued again, once.
                PlatformWalletError::PersisterStore(refusal) if refusal.is_transient() => {
                    let keys = key_rows(&managed.identity);
                    managed
                        .store_keys_with_snapshot(keys, persister)
                        .map_err(|e| persister.classify_store_failure(e))
                }
                error => Err(error),
            },
        };
        if let Err(e) = stored {
            tracing::warn!(
                flow,
                %identity_id,
                identity_index,
                error = %e,
                "identity accepted by Platform and held in memory, but the identity \
                 and its keys were not persisted"
            );
        }
    }

    /// [`Self::add_identity`] in memory only: the caller stores the identity
    /// and its keys itself, as one changeset. Load and discovery use it, so
    /// the keys they verified against the seed reach the store with the
    /// identity, never after a breadcrumb-less snapshot of them that could
    /// commit on its own (a disk that fills between the two writes would
    /// otherwise leave every key row without its breadcrumb).
    pub(crate) fn add_identity_unstored(
        &mut self,
        identity: Identity,
        identity_index: u32,
        wallet_id: WalletId,
    ) -> Result<(), PlatformWalletError> {
        let identity_id = identity.id();

        if self.identity(&identity_id).is_some() {
            return Err(PlatformWalletError::IdentityAlreadyExists(identity_id));
        }

        let mut managed_identity = ManagedIdentity::new(identity, identity_index);
        // Denormalize the bucket keys onto the value so callers that
        // iterate values know which wallet + slot they came from
        // without re-threading the keys.
        managed_identity.wallet_id = Some(wallet_id);
        // Held before anything can send or accept for it, when a persister
        // kept its record marked.
        if self.retained_private_payment_markers.remove(&identity_id) {
            managed_identity.hold_for_retained_private_payment_marker();
        }

        self.wallet_identities
            .entry(wallet_id)
            .or_default()
            .insert(identity_index, managed_identity);
        // Keep the side-index invariant in lockstep with the bucket
        // insert above.
        self.location_index_insert(
            identity_id,
            IdentityLocation::InWallet {
                wallet_id,
                registration_index: identity_index,
            },
        );

        Ok(())
    }

    /// Hold every identity in `owners`, whose private contact request scan
    /// state record a persister kept marked
    /// (`PrivateContactRequestsPosition::private_payments_unresolved`), as a
    /// load holds one loaded with its record marked
    /// (`DashPayState::private_payments_unrestored`): one this manager holds
    /// now at once, every other when it is added. Its private payment may
    /// still execute, while the identity, added afresh, holds no reservation
    /// or record of it. Only a restoration from the shielded store's journal
    /// releases the hold.
    pub(crate) fn retain_private_payment_markers(
        &mut self,
        owners: impl IntoIterator<Item = Identifier>,
    ) {
        for owner in owners {
            match self.managed_identity_mut(&owner) {
                Some(managed) => managed.hold_for_retained_private_payment_marker(),
                None => {
                    self.retained_private_payment_markers.insert(owner);
                }
            }
        }
    }

    /// Add an identity to the out-of-wallet (observed read-only) bucket.
    ///
    /// There is no separate `WatchedIdentity` type; observed identities
    /// are just `ManagedIdentity` rows with `wallet_id == None` and
    /// `identity_index == None`, which is what forces signing and
    /// derivation callers to handle them explicitly. The `Identity` is
    /// stored exactly as supplied — an observed identity fetched from
    /// Platform keeps the public keys it came with. If an identity with
    /// the same id is already in either bucket this is a no-op.
    pub fn add_out_of_wallet_identity(
        &mut self,
        identity: Identity,
        persister: &WalletPersister,
    ) -> Result<(), PlatformWalletError> {
        let identity_id = identity.id();

        if self.identity(&identity_id).is_some() {
            return Ok(());
        }

        // `identity_index` is meaningless for out-of-wallet identities
        // (there's no wallet to derive against). The dedicated constructor
        // sets it to `None` — the type then forces every signing /
        // derivation caller downstream to handle the absence explicitly.
        let managed = ManagedIdentity::new_out_of_wallet(identity);
        let entry = IdentityEntry::from_managed(&managed);
        self.out_of_wallet_identities.insert(identity_id, managed);
        // Side-index records the bucket discriminant — no placeholder
        // index, no ambiguity with the `0`-registered case.
        self.location_index_insert(identity_id, IdentityLocation::OutOfWallet);

        let mut id_cs = IdentityChangeSet::default();
        id_cs.identities.insert(identity_id, entry);

        if let Err(e) = persister.store(id_cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }

        Ok(())
    }

    /// Get the BIP-9 HD identity index for a given identity ID.
    ///
    /// Returns `None` if the identity is not managed or the lookup
    /// landed in the out-of-wallet bucket (where `identity_index` is
    /// meaningless). O(log n) via the side-index.
    pub fn identity_index(&self, identity_id: &Identifier) -> Option<u32> {
        // Only wallet-owned identities have a meaningful index — read
        // the bucket discriminant straight off the side-index and bail
        // out for the out-of-wallet case. Unknown ids miss cleanly.
        match self.location_index().get(identity_id).copied()? {
            IdentityLocation::InWallet {
                registration_index, ..
            } => Some(registration_index),
            IdentityLocation::OutOfWallet => None,
        }
    }

    /// Remove an identity from whichever bucket holds it.
    ///
    /// Persists the removal via `persister` and returns the removed
    /// [`Identity`]. O(log n) via the side-index.
    ///
    /// Removal is terminal in storage as well as in memory: the persister
    /// deletes the identity and every row it owns, so re-adding the same
    /// id later starts from a blank identity.
    ///
    /// Refused, with nothing removed, while the identity's private contact
    /// request channel is held: a contact request in flight, private or
    /// public, or a private payment that may still execute, whether its own
    /// state or the wallet's journal holds it
    /// ([`PlatformWalletError::PrivateContactRequestPending`]; a public one
    /// whose outcome is unconfirmed is
    /// [`PlatformWalletError::PublicContactRequestUnconfirmed`]), or one from an
    /// earlier process not restored yet
    /// ([`PlatformWalletError::PrivateContactPaymentsNotRestored`]). Removed
    /// and loaded again, the identity would start with none of those holds,
    /// and its persisted marker with it, so a public request could publish a
    /// pair a private payment still keeps off chain.
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
        persister: &WalletPersister,
    ) -> Result<Identity, PlatformWalletError> {
        if let Some(managed) = self.managed_identity(identity_id) {
            let dashpay = managed.dashpay();
            if let Some(recipient) = dashpay
                .unconfirmed_public_request_recipients()
                .first()
                .copied()
            {
                return Err(PlatformWalletError::PublicContactRequestUnconfirmed(
                    recipient,
                ));
            }
            if let Some(recipient) = dashpay
                .contact_request_in_flight()
                .or_else(|| persister.private_sender_journaled(identity_id))
            {
                return Err(PlatformWalletError::PrivateContactRequestPending(recipient));
            }
            if dashpay.private_payments_unrestored() {
                return Err(PlatformWalletError::PrivateContactPaymentsNotRestored);
            }
        }
        // Drop the side-index entry first; its value tells us which
        // bucket the identity lives in so we can avoid a second scan.
        let location = self
            .location_index_remove(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let removed_identity = match location {
            IdentityLocation::OutOfWallet => {
                let managed = self
                    .out_of_wallet_identities
                    .remove(identity_id)
                    .expect("location_index agrees with out_of_wallet_identities");
                managed.identity
            }
            IdentityLocation::InWallet {
                wallet_id,
                registration_index,
            } => {
                let inner = self
                    .wallet_identities
                    .get_mut(&wallet_id)
                    .expect("location_index agrees with wallet_identities outer map");
                let managed = inner
                    .remove(&registration_index)
                    .expect("location_index agrees with wallet_identities inner map");
                // Drop the empty inner map entirely so `highest_registration_index`
                // returns `None` for an empty wallet bucket.
                if inner.is_empty() {
                    self.wallet_identities.remove(&wallet_id);
                }
                managed.identity
            }
        };

        self.persist_removal(*identity_id, persister);
        Ok(removed_identity)
    }

    fn persist_removal(&self, identity_id: Identifier, persister: &WalletPersister) {
        let mut cs = IdentityChangeSet::default();
        cs.removed.insert(identity_id);
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }
}

// `dpp::identity::accessors::IdentityGettersV0` is referenced via the
// `id()` calls above through the imports at the top.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changeset::{
        ClientStartState, IdentityKeyDerivationIndices, PersistenceError, PersistenceErrorKind,
        PlatformWalletPersistence,
    };
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    const WALLET_ID: WalletId = [9u8; 32];
    const IDENTITY_INDEX: u32 = 3;

    /// Records every changeset stored, and fails every store when armed.
    #[derive(Default)]
    struct RecordingPersister {
        fail: bool,
        /// Stores still to refuse as transient, nothing retained.
        transient_failures: AtomicUsize,
        reissuable: bool,
        stores: Mutex<Vec<PlatformWalletChangeSet>>,
    }

    impl PlatformWalletPersistence for RecordingPersister {
        fn store_transient_is_reissuable(&self) -> bool {
            self.reissuable
        }

        fn store(
            &self,
            _wallet_id: WalletId,
            changeset: PlatformWalletChangeSet,
        ) -> Result<(), PersistenceError> {
            if self.fail {
                return Err(PersistenceError::backend("store armed to fail"));
            }
            let refuse = self
                .transient_failures
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                    left.checked_sub(1)
                })
                .is_ok();
            if refuse {
                return Err(PersistenceError::backend_with_kind(
                    PersistenceErrorKind::Transient,
                    "busy",
                ));
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

    fn persister(fail: bool) -> (Arc<RecordingPersister>, WalletPersister) {
        let backend = Arc::new(RecordingPersister {
            fail,
            ..Default::default()
        });
        let persister = WalletPersister::new(WALLET_ID, backend.clone());
        (backend, persister)
    }

    fn identity_with_keys(id: [u8; 32], key_ids: &[KeyID]) -> Identity {
        let public_keys: BTreeMap<KeyID, IdentityPublicKey> = key_ids
            .iter()
            .map(|key_id| {
                let key = IdentityPublicKey::V0(IdentityPublicKeyV0 {
                    id: *key_id,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    contract_bounds: None,
                    key_type: KeyType::ECDSA_SECP256K1,
                    read_only: false,
                    data: BinaryData::new(vec![0x02; 33]),
                    disabled_at: None,
                });
                (*key_id, key)
            })
            .collect();
        Identity::V0(IdentityV0 {
            id: Identifier::from(id),
            public_keys,
            balance: 0,
            revision: 0,
        })
    }

    /// The registration flows call this after Platform accepted the identity
    /// and still owe work after it (the asset-lock flow consumes its lock).
    /// The method returns nothing, so a store that fails cannot turn the
    /// accepted registration into an error or skip that work; the identity
    /// stays usable in memory for the rest of the session.
    #[test]
    fn should_keep_an_accepted_identity_in_memory_when_its_keys_cannot_be_stored() {
        let mut manager = IdentityManager::new();
        let (_, failing) = persister(true);
        let identity = identity_with_keys([1u8; 32], &[0, 1]);
        let identity_id = identity.id();

        manager.record_new_identity(
            identity,
            IDENTITY_INDEX,
            WALLET_ID,
            NewIdentityKeyOrigin::DerivedAtKeyId,
            "test_flow",
            &failing,
        );

        let managed = manager.identity(&identity_id).expect("held in memory");
        assert_eq!(managed.wallet_id, Some(WALLET_ID));
        assert_eq!(managed.identity_index, Some(IDENTITY_INDEX));
        assert_eq!(managed.identity.public_keys().len(), 2);
        assert_eq!(manager.identity_index(&identity_id), Some(IDENTITY_INDEX));
    }

    #[test]
    fn should_store_a_new_identity_and_its_key_breadcrumbs_as_one_changeset() {
        let mut manager = IdentityManager::new();
        let (backend, recording) = persister(false);
        let identity = identity_with_keys([2u8; 32], &[0, 1]);
        let identity_id = identity.id();

        manager.record_new_identity(
            identity,
            IDENTITY_INDEX,
            WALLET_ID,
            NewIdentityKeyOrigin::DerivedAtKeyId,
            "test_flow",
            &recording,
        );

        let stores = backend.stores.lock().unwrap();
        assert_eq!(
            stores.len(),
            1,
            "identity and keys reach the store together"
        );
        let identities = stores[0].identities.as_ref().expect("identity entry");
        assert!(identities.identities.contains_key(&identity_id));
        let keys = stores[0].identity_keys.as_ref().expect("key entries");
        assert_eq!(keys.upserts.len(), 2);
        for key_id in [0, 1] {
            let entry = &keys.upserts[&(identity_id, key_id)];
            assert_eq!(entry.wallet_id, Some(WALLET_ID));
            assert_eq!(
                entry.derivation_indices,
                Some(IdentityKeyDerivationIndices {
                    identity_index: IDENTITY_INDEX,
                    key_index: key_id,
                })
            );
        }
    }

    fn transient_persister(
        failures: usize,
        reissuable: bool,
    ) -> (Arc<RecordingPersister>, WalletPersister) {
        let backend = Arc::new(RecordingPersister {
            transient_failures: AtomicUsize::new(failures),
            reissuable,
            ..Default::default()
        });
        let persister = WalletPersister::new(WALLET_ID, backend.clone());
        (backend, persister)
    }

    /// The identity and its keys go out as one changeset, so one refused
    /// store would leave nothing of the identity on disk, and a launch that
    /// finds an older identity on file runs no scan to find it again.
    #[test]
    fn should_store_a_new_identity_again_after_one_transient_refusal() {
        let mut manager = IdentityManager::new();
        let (backend, busy_once) = transient_persister(1, true);
        let identity = identity_with_keys([5u8; 32], &[0, 1]);
        let identity_id = identity.id();

        manager.record_new_identity(
            identity,
            IDENTITY_INDEX,
            WALLET_ID,
            NewIdentityKeyOrigin::DerivedAtKeyId,
            "test_flow",
            &busy_once,
        );

        let stores = backend.stores.lock().unwrap();
        assert_eq!(stores.len(), 1, "the refused changeset is stored again");
        let identities = stores[0].identities.as_ref().expect("identity entry");
        assert!(identities.identities.contains_key(&identity_id));
        let keys = stores[0].identity_keys.as_ref().expect("key entries");
        for key_id in [0, 1] {
            assert_eq!(
                keys.upserts[&(identity_id, key_id)].derivation_indices,
                Some(IdentityKeyDerivationIndices {
                    identity_index: IDENTITY_INDEX,
                    key_index: key_id,
                })
            );
        }
    }

    /// A backend that does not attest it retained nothing of a refused store
    /// may still hold the changeset; storing it again could apply it twice.
    #[test]
    fn should_not_store_again_when_the_backend_may_have_kept_the_refused_changeset() {
        let mut manager = IdentityManager::new();
        let (backend, busy_once) = transient_persister(1, false);
        let identity = identity_with_keys([6u8; 32], &[0]);

        manager.record_new_identity(
            identity,
            IDENTITY_INDEX,
            WALLET_ID,
            NewIdentityKeyOrigin::DerivedAtKeyId,
            "test_flow",
            &busy_once,
        );

        assert!(backend.stores.lock().unwrap().is_empty());
        assert_eq!(backend.transient_failures.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn should_store_no_breadcrumb_for_keys_of_unrecorded_origin() {
        let mut manager = IdentityManager::new();
        let (backend, recording) = persister(false);
        let identity = identity_with_keys([3u8; 32], &[0]);
        let identity_id = identity.id();

        manager.record_new_identity(
            identity,
            IDENTITY_INDEX,
            WALLET_ID,
            NewIdentityKeyOrigin::Unrecorded,
            "test_flow",
            &recording,
        );

        let stores = backend.stores.lock().unwrap();
        assert_eq!(stores.len(), 1);
        let keys = stores[0].identity_keys.as_ref().expect("key entries");
        let entry = &keys.upserts[&(identity_id, 0)];
        assert_eq!(entry.wallet_id, None);
        assert_eq!(entry.derivation_indices, None);
    }

    /// An id the manager already holds may be an out-of-wallet entry: it is
    /// left as it was, and no wallet breadcrumb is stored for its keys.
    #[test]
    fn should_leave_an_identity_already_held_untouched() {
        let mut manager = IdentityManager::new();
        let (backend, recording) = persister(false);
        let identity = identity_with_keys([4u8; 32], &[0]);
        let identity_id = identity.id();
        manager
            .add_out_of_wallet_identity(identity.clone(), &recording)
            .unwrap();
        let stores_before = backend.stores.lock().unwrap().len();

        manager.record_new_identity(
            identity,
            IDENTITY_INDEX,
            WALLET_ID,
            NewIdentityKeyOrigin::DerivedAtKeyId,
            "test_flow",
            &recording,
        );

        let managed = manager.identity(&identity_id).expect("still held");
        assert_eq!(managed.wallet_id, None);
        assert_eq!(manager.identity_index(&identity_id), None);
        assert_eq!(backend.stores.lock().unwrap().len(), stores_before);
    }
}
