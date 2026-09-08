//! Construction & lifecycle for [`IdentityManager`].
//!
//! Adds, removes, and looks up the HD index for managed identities.
//! Every mutation here persists a corresponding
//! [`IdentityChangeSet`](crate::changeset::IdentityChangeSet) via
//! the supplied [`WalletPersister`].

use super::{IdentityLocation, IdentityManager};
use crate::changeset::{IdentityChangeSet, IdentityEntry, PlatformWalletChangeSet};
use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::ManagedIdentity;
use crate::wallet::persister::WalletPersister;
use crate::wallet::platform_wallet::WalletId;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

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

        if self.identity(&identity_id).is_some() {
            return Err(PlatformWalletError::IdentityAlreadyExists(identity_id));
        }

        let mut managed_identity = ManagedIdentity::new(identity, identity_index);
        // Denormalize the bucket keys onto the value so callers that
        // iterate values know which wallet + slot they came from
        // without re-threading the keys.
        managed_identity.wallet_id = Some(wallet_id);

        let entry = IdentityEntry::from_managed(&managed_identity);
        // No `key_storage` to snapshot here anymore — the field is
        // dropped from `ManagedIdentity`. Public keys still arrive
        // via the separate `IdentityKeysChangeSet` path.
        let keys_cs = managed_identity.keys_snapshot_changeset();

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
    /// Persists the tombstone changeset via `persister` and returns the
    /// removed [`Identity`]. O(log n) via the side-index.
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
        persister: &WalletPersister,
    ) -> Result<Identity, PlatformWalletError> {
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
