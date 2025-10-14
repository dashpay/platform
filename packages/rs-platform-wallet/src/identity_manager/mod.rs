//! Identity management for platform wallets
//!
//! This module handles the storage and management of Dash Platform identities
//! associated with a wallet.

use crate::managed_identity::ManagedIdentity;
use dpp::prelude::Identifier;
use indexmap::IndexMap;

// Import implementation modules
mod initializers;
mod accessors;

/// Manages identities for a platform wallet
#[derive(Debug, Clone, Default)]
pub struct IdentityManager {
    /// All managed identities owned by this wallet, indexed by identity ID
    pub identities: IndexMap<Identifier, ManagedIdentity>,

    /// The primary identity ID (if set)
    pub primary_identity_id: Option<Identifier>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_identity(id: Identifier) -> dpp::identity::Identity {
        use dpp::identity::v0::IdentityV0;
        use dpp::identity::Identity;
        use std::collections::BTreeMap;

        // Create a minimal test identity
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

        manager.add_identity(identity.clone()).unwrap();

        assert_eq!(manager.identities.len(), 1);
        assert!(manager.identity(&identity_id).is_some());
        assert_eq!(manager.primary_identity_id, Some(identity_id));
    }

    #[test]
    fn test_remove_identity() {
        use dpp::identity::accessors::IdentityGettersV0;

        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);
        let identity = create_test_identity(identity_id);

        manager.add_identity(identity).unwrap();
        let removed = manager.remove_identity(&identity_id).unwrap();

        assert_eq!(removed.id(), identity_id);
        assert_eq!(manager.identities.len(), 0);
        assert_eq!(manager.primary_identity_id, None);
    }

    #[test]
    fn test_primary_identity_switching() {
        let mut manager = IdentityManager::new();

        let id1 = Identifier::from([1u8; 32]);
        let id2 = Identifier::from([2u8; 32]);

        manager.add_identity(create_test_identity(id1)).unwrap();
        manager.add_identity(create_test_identity(id2)).unwrap();

        // First identity should be primary
        assert_eq!(manager.primary_identity_id, Some(id1));

        // Switch primary
        manager.set_primary_identity(id2).unwrap();
        assert_eq!(manager.primary_identity_id, Some(id2));
    }

    #[test]
    fn test_managed_identity() {
        let mut manager = IdentityManager::new();
        let identity_id = Identifier::from([1u8; 32]);

        manager
            .add_identity(create_test_identity(identity_id))
            .unwrap();

        // Update metadata
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
