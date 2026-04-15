//! Identity management for platform wallets
//!
//! This module handles the storage and management of Dash Platform identities
//! associated with a wallet.

use super::managed_identity::key_storage::IdentityStatus;
use super::managed_identity::ManagedIdentity;
use super::managed_identity::WatchedIdentity;
use crate::changeset::{IdentityChangeSet, IdentityEntry};
use crate::error::PlatformWalletError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use indexmap::IndexMap;

/// Manages identities for a platform wallet
#[derive(Debug, Clone)]
pub struct IdentityManager {
    /// All managed identities owned by this wallet, indexed by identity ID
    pub(crate) identities: IndexMap<Identifier, ManagedIdentity>,

    /// Watched (observed, read-only) identities — we can see them but cannot sign
    pub(crate) watched_identities: IndexMap<Identifier, WatchedIdentity>,

    /// The primary identity ID (if set)
    pub(crate) primary_identity_id: Option<Identifier>,

    /// The last scanned identity index for gap-limit scanning
    pub(crate) last_scanned_index: u32,
}

impl Default for IdentityManager {
    fn default() -> Self {
        Self {
            identities: IndexMap::new(),
            watched_identities: IndexMap::new(),
            primary_identity_id: None,
            last_scanned_index: 0,
        }
    }
}

// --- Construction & lifecycle ---

impl IdentityManager {
    /// Create a new identity manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an identity to the manager with its BIP-9 HD identity index.
    ///
    /// Every identity in this wallet must have its HD index so that signing
    /// and ECDH derivation can locate the correct keys.
    ///
    /// Returns an [`IdentityChangeSet`] carrying a full snapshot of the
    /// new identity and, if this is the first identity, the `primary_identity`
    /// selection.
    pub fn add_identity(
        &mut self,
        identity: Identity,
        identity_index: u32,
    ) -> Result<IdentityChangeSet, PlatformWalletError> {
        let identity_id = identity.id();

        if self.identities.contains_key(&identity_id) {
            return Err(PlatformWalletError::IdentityAlreadyExists(identity_id));
        }

        let managed_identity = ManagedIdentity::new(identity, identity_index);
        let entry = IdentityEntry::from_managed(&managed_identity);
        self.identities.insert(identity_id, managed_identity);

        let mut cs = IdentityChangeSet::default();
        cs.identities.insert(identity_id, entry);

        // If this is the first identity, make it primary
        if self.identities.len() == 1 {
            self.primary_identity_id = Some(identity_id);
            cs.primary_identity = Some(identity_id);
        }

        Ok(cs)
    }

    /// Get the BIP-9 HD identity index for a given identity ID.
    ///
    /// Returns `None` if the identity is not managed or its index was not recorded.
    pub fn identity_index(&self, identity_id: &Identifier) -> Option<u32> {
        self.identities.get(identity_id).map(|m| m.identity_index)
    }

    /// Remove an identity from the manager.
    ///
    /// Returns the removed [`Identity`] and an [`IdentityChangeSet`] with a
    /// tombstone and — if the removed identity was primary and another
    /// identity took its place — the new primary selection.
    ///
    /// Note: if the removed identity was the only one, `primary_identity`
    /// in the changeset remains `None`; the apply path must re-derive the
    /// cleared state from the `removed` set alone.
    pub fn remove_identity(
        &mut self,
        identity_id: &Identifier,
    ) -> Result<(Identity, IdentityChangeSet), PlatformWalletError> {
        let managed_identity = self
            .identities
            .shift_remove(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;

        let mut cs = IdentityChangeSet::default();
        cs.removed.insert(*identity_id);

        if self.primary_identity_id == Some(*identity_id) {
            self.primary_identity_id = self.identities.keys().next().copied();
            if let Some(new_primary) = self.primary_identity_id {
                cs.primary_identity = Some(new_primary);
            }
        }

        Ok((managed_identity.identity, cs))
    }
}

// --- Accessors ---

impl IdentityManager {
    /// Get an identity by ID
    pub fn identity(&self, identity_id: &Identifier) -> Option<&Identity> {
        self.identities.get(identity_id).map(|m| &m.identity)
    }

    /// Get a mutable reference to an identity
    pub fn identity_mut(&mut self, identity_id: &Identifier) -> Option<&mut Identity> {
        self.identities
            .get_mut(identity_id)
            .map(|m| &mut m.identity)
    }

    /// Get all identities
    pub fn identities(&self) -> IndexMap<Identifier, Identity> {
        self.identities
            .iter()
            .map(|(id, managed)| (*id, managed.identity.clone()))
            .collect()
    }

    /// Get all identities as a vector
    pub fn all_identities(&self) -> Vec<&Identity> {
        self.identities
            .values()
            .map(|managed| &managed.identity)
            .collect()
    }

    /// Get the primary identity ID.
    pub fn primary_identity_id(&self) -> Option<&Identifier> {
        self.primary_identity_id.as_ref()
    }

    /// Get the primary identity
    pub fn primary_identity(&self) -> Option<&Identity> {
        self.primary_identity_id
            .as_ref()
            .and_then(|id| self.identities.get(id))
            .map(|m| &m.identity)
    }

    /// Set the primary identity.
    ///
    /// Returns an [`IdentityChangeSet`] carrying the new selection.
    pub fn set_primary_identity(
        &mut self,
        identity_id: Identifier,
    ) -> Result<IdentityChangeSet, PlatformWalletError> {
        if !self.identities.contains_key(&identity_id) {
            return Err(PlatformWalletError::IdentityNotFound(identity_id));
        }
        self.primary_identity_id = Some(identity_id);
        Ok(IdentityChangeSet {
            primary_identity: Some(identity_id),
            ..Default::default()
        })
    }

    /// Get a managed identity by ID
    pub fn managed_identity(&self, identity_id: &Identifier) -> Option<&ManagedIdentity> {
        self.identities.get(identity_id)
    }

    /// Get a mutable managed identity by ID
    pub fn managed_identity_mut(
        &mut self,
        identity_id: &Identifier,
    ) -> Option<&mut ManagedIdentity> {
        self.identities.get_mut(identity_id)
    }

    /// Set a label for an identity.
    ///
    /// Returns an [`IdentityChangeSet`] carrying a full snapshot of the
    /// updated identity.
    pub fn set_label(
        &mut self,
        identity_id: &Identifier,
        label: String,
    ) -> Result<IdentityChangeSet, PlatformWalletError> {
        let managed = self
            .identities
            .get_mut(identity_id)
            .ok_or(PlatformWalletError::IdentityNotFound(*identity_id))?;
        Ok(managed.set_label(label))
    }

    /// Get total credit balance across all identities
    pub fn total_credit_balance(&self) -> u64 {
        self.identities
            .values()
            .map(|managed| managed.identity.balance())
            .sum()
    }

    /// Get the number of managed identities.
    pub fn identity_count(&self) -> usize {
        self.identities.len()
    }

    /// Check if there are no managed identities.
    pub fn is_empty(&self) -> bool {
        self.identities.is_empty()
    }

    /// Get the last scanned identity index.
    pub fn last_scanned_index(&self) -> u32 {
        self.last_scanned_index
    }

    /// Set the last scanned identity index.
    ///
    /// Returns an [`IdentityChangeSet`] carrying the new watermark.
    pub fn set_last_scanned_index(&mut self, index: u32) -> IdentityChangeSet {
        self.last_scanned_index = index;
        IdentityChangeSet {
            last_scanned_index: Some(index),
            ..Default::default()
        }
    }
}

// --- Apply (restore from changeset) ---

impl IdentityManager {
    /// Restore a single [`IdentityEntry`] into the manager.
    ///
    /// This is the apply-side counterpart to the snapshot-emitting
    /// mutation methods on [`ManagedIdentity`]. It does NOT enforce
    /// invariants the mutation methods do at runtime (DPNS dedup,
    /// auto-establishment, fresh block-time stamps) — replay must be
    /// faithful to whatever the persisted entry carried.
    ///
    /// Contact state (`sent_contact_requests`, `incoming_contact_requests`,
    /// `established_contacts`) is NOT touched here — those live in
    /// [`ContactChangeSet`](crate::changeset::ContactChangeSet) and are
    /// applied separately by the caller.
    ///
    /// # Behaviour
    ///
    /// - If the identity is not yet present, a fresh `ManagedIdentity` is
    ///   constructed and every persistable field is copied from the entry.
    /// - If the identity is already present, persistable fields are
    ///   updated in place. The on-chain `Identity` blob is only replaced
    ///   when `entry.identity.revision() >= existing.identity.revision()`,
    ///   matching the merge policy on `IdentityChangeSet`.
    ///
    /// Idempotent: applying the same entry twice is the same as applying
    /// it once.
    pub(crate) fn apply_identity_entry(&mut self, entry: IdentityEntry) {
        use dpp::identity::accessors::IdentityGettersV0;
        let id = entry.identity.id();
        match self.identities.get_mut(&id) {
            Some(existing) => {
                if entry.identity.revision() >= existing.identity.revision() {
                    existing.identity = entry.identity;
                }
                // identity_index is immutable per identity (it's the
                // BIP-9 HD derivation index used during registration),
                // so we don't overwrite it here. This matches
                // `IdentityChangeSet::merge`'s policy.
                existing.label = entry.label;
                existing.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                existing.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                existing.status = entry.status;
                existing.wallet_id = entry.wallet_id;
                existing.dashpay_profile = entry.dashpay_profile;
                // Append new DPNS names by label, preserving any
                // pre-existing entries the changeset didn't carry.
                for name in entry.dpns_names {
                    if !existing.dpns_names.iter().any(|n| n.label == name.label) {
                        existing.dpns_names.push(name);
                    }
                }
                existing.key_storage.extend(entry.key_storage);
                existing.dashpay_payments.extend(entry.dashpay_payments);
            }
            None => {
                let mut managed = ManagedIdentity::new(entry.identity, entry.identity_index);
                managed.label = entry.label;
                managed.last_updated_balance_block_time = entry.last_updated_balance_block_time;
                managed.last_synced_keys_block_time = entry.last_synced_keys_block_time;
                managed.status = entry.status;
                managed.wallet_id = entry.wallet_id;
                managed.dpns_names = entry.dpns_names;
                managed.key_storage = entry.key_storage;
                managed.dashpay_profile = entry.dashpay_profile;
                managed.dashpay_payments = entry.dashpay_payments;
                self.identities.insert(id, managed);
                if self.primary_identity_id.is_none() {
                    self.primary_identity_id = Some(id);
                }
            }
        }
    }
}

// --- Watched identities ---

impl IdentityManager {
    /// Add a watched (read-only) identity.
    ///
    /// Watched identities are observed but not owned — we cannot sign on their
    /// behalf. If an identity with the same ID already exists in either the
    /// managed or watched collection, this is a no-op.
    pub fn add_watched_identity(&mut self, identity: Identity) -> Result<(), PlatformWalletError> {
        let identity_id = identity.id();

        // Already managed or watched — nothing to do.
        if self.identities.contains_key(&identity_id)
            || self.watched_identities.contains_key(&identity_id)
        {
            return Ok(());
        }

        self.watched_identities.insert(
            identity_id,
            WatchedIdentity {
                identity,
                dpns_names: Vec::new(),
                status: IdentityStatus::Active,
            },
        );

        Ok(())
    }

    /// Look up a watched identity by ID.
    pub fn watched_identity(&self, identity_id: &Identifier) -> Option<&WatchedIdentity> {
        self.watched_identities.get(identity_id)
    }

    /// Get all watched identities.
    pub fn all_watched_identities(&self) -> Vec<&WatchedIdentity> {
        self.watched_identities.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_identity(id: Identifier) -> Identity {
        use dpp::identity::v0::IdentityV0;
        use std::collections::BTreeMap;

        let identity_v0 = IdentityV0 {
            id,
            public_keys: BTreeMap::new(),
            balance: 0,
            revision: 0,
        };

        Identity::V0(identity_v0)
    }

    #[test]
    fn test_add_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);

        manager.add_identity(identity.clone(), 0).unwrap();

        assert_eq!(manager.identities.len(), 1);
        assert!(manager.identity(&identity_id).is_some());
        assert_eq!(manager.primary_identity_id, Some(identity_id));
        assert_eq!(manager.identity_index(&identity_id), Some(0));
    }

    #[test]
    fn test_remove_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);

        manager.add_identity(identity, 0).unwrap();
        let (removed, cs) = manager.remove_identity(&identity_id).unwrap();

        assert_eq!(removed.id(), identity_id);
        assert!(cs.removed.contains(&identity_id));
        assert_eq!(manager.identities.len(), 0);
        assert_eq!(manager.primary_identity_id, None);
    }

    #[test]
    fn test_primary_identity_switching() {
        let mut manager = IdentityManager::new();

        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);

        manager.add_identity(create_test_identity(id1), 0).unwrap();
        manager.add_identity(create_test_identity(id2), 1).unwrap();

        assert_eq!(manager.primary_identity_id, Some(id1));

        manager.set_primary_identity(id2).unwrap();
        assert_eq!(manager.primary_identity_id, Some(id2));
    }

    #[test]
    fn test_managed_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);

        manager
            .add_identity(create_test_identity(identity_id), 0)
            .unwrap();

        manager
            .set_label(&identity_id, "My Identity".to_string())
            .unwrap();

        let managed = manager.managed_identity(&identity_id).unwrap();
        assert_eq!(managed.label, Some("My Identity".to_string()));
        assert_eq!(managed.last_updated_balance_block_time, None);
        assert_eq!(managed.last_synced_keys_block_time, None);
        assert_eq!(managed.id(), identity_id);
    }
}
