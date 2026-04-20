//! Identity management for platform wallets.
//!
//! Storage and management of Dash Platform identities associated
//! with a single wallet. Implementation is split across several
//! sibling files by responsibility:
//!
//! - [`lifecycle`] — construction, add / remove, HD index lookup.
//! - [`accessors`] — read/write access to managed identities
//!   (getters, labels, primary, total balance, scan watermark).
//! - [`apply`] — replay path for persisted
//!   [`IdentityEntry`](crate::changeset::IdentityEntry)s.
//! - [`watched`] — read-only "observed" identities the wallet does
//!   not own keys for.

mod accessors;
mod apply;
mod lifecycle;
mod watched;

use super::managed_identity::ManagedIdentity;
use super::managed_identity::WatchedIdentity;
use crate::changeset::IdentityManagerStartState;
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

impl From<IdentityManagerStartState> for IdentityManager {
    fn from(state: IdentityManagerStartState) -> Self {
        let IdentityManagerStartState {
            identities,
            watched_identities,
            primary_identity_id,
            last_scanned_index,
        } = state;
        Self {
            identities,
            watched_identities,
            primary_identity_id,
            last_scanned_index,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::persister::{NoPlatformPersistence, WalletPersister};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use std::sync::Arc;

    fn noop_persister() -> WalletPersister {
        WalletPersister::new([0u8; 32], Arc::new(NoPlatformPersistence))
    }

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
        let p = noop_persister();

        manager.add_identity(identity.clone(), 0, &p).unwrap();

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
        let p = noop_persister();

        manager.add_identity(identity, 0, &p).unwrap();
        let removed = manager.remove_identity(&identity_id, &p).unwrap();

        assert_eq!(removed.id(), identity_id);
        assert_eq!(manager.identities.len(), 0);
        assert_eq!(manager.primary_identity_id, None);
    }

    #[test]
    fn test_primary_identity_switching() {
        let mut manager = IdentityManager::new();
        let p = noop_persister();

        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);

        manager
            .add_identity(create_test_identity(id1), 0, &p)
            .unwrap();
        manager
            .add_identity(create_test_identity(id2), 1, &p)
            .unwrap();

        assert_eq!(manager.primary_identity_id, Some(id1));

        manager.set_primary_identity(id2, &p).unwrap();
        assert_eq!(manager.primary_identity_id, Some(id2));
    }

    #[test]
    fn test_managed_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);
        let p = noop_persister();

        manager
            .add_identity(create_test_identity(identity_id), 0, &p)
            .unwrap();

        manager
            .set_label(&identity_id, "My Identity".to_string(), &p)
            .unwrap();

        let managed = manager.managed_identity(&identity_id).unwrap();
        assert_eq!(managed.label, Some("My Identity".to_string()));
        assert_eq!(managed.last_updated_balance_block_time, None);
        assert_eq!(managed.last_synced_keys_block_time, None);
        assert_eq!(managed.id(), identity_id);
    }
}
