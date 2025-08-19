//! Managed identity that combines a Platform Identity with wallet-specific metadata
//!
//! This module provides the `ManagedIdentity` struct which wraps a Platform Identity
//! with additional metadata for wallet management.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

/// A managed identity that combines an Identity with wallet-specific metadata
#[derive(Debug, Clone)]
pub struct ManagedIdentity {
    /// The Platform identity
    pub identity: Identity,

    /// Last sync timestamp for this identity
    pub last_sync_timestamp: Option<u64>,

    /// Last sync block height
    pub last_sync_height: Option<u64>,

    /// User-defined label for this identity
    pub label: Option<String>,

    /// Whether this identity is active
    pub is_active: bool,
}

impl ManagedIdentity {
    /// Create a new managed identity
    pub fn new(identity: Identity) -> Self {
        Self {
            identity,
            last_sync_timestamp: None,
            last_sync_height: None,
            label: None,
            is_active: true,
        }
    }

    /// Get the identity ID
    pub fn id(&self) -> Identifier {
        self.identity.id()
    }

    /// Get the identity's balance
    pub fn balance(&self) -> u64 {
        self.identity.balance()
    }

    /// Get the identity's revision
    pub fn revision(&self) -> u64 {
        self.identity.revision()
    }

    /// Set the label for this identity
    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    /// Clear the label for this identity
    pub fn clear_label(&mut self) {
        self.label = None;
    }

    /// Mark this identity as active
    pub fn activate(&mut self) {
        self.is_active = true;
    }

    /// Mark this identity as inactive
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Update the last sync information
    pub fn update_sync_info(&mut self, timestamp: u64, height: u64) {
        self.last_sync_timestamp = Some(timestamp);
        self.last_sync_height = Some(height);
    }

    /// Check if this identity needs syncing based on time elapsed
    pub fn needs_sync(&self, current_timestamp: u64, max_age_seconds: u64) -> bool {
        match self.last_sync_timestamp {
            Some(last_sync) => (current_timestamp - last_sync) > max_age_seconds,
            None => true, // Never synced
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::v0::IdentityV0;
    use std::collections::BTreeMap;

    fn create_test_identity() -> Identity {
        let identity_v0 = IdentityV0 {
            id: Identifier::from([1u8; 32]),
            public_keys: BTreeMap::new(),
            balance: 1000,
            revision: 1,
        };
        Identity::V0(identity_v0)
    }

    #[test]
    fn test_managed_identity_creation() {
        let identity = create_test_identity();
        let managed = ManagedIdentity::new(identity);

        assert_eq!(managed.id(), Identifier::from([1u8; 32]));
        assert_eq!(managed.balance(), 1000);
        assert_eq!(managed.revision(), 1);
        assert_eq!(managed.label, None);
        assert_eq!(managed.is_active, true);
        assert_eq!(managed.last_sync_timestamp, None);
        assert_eq!(managed.last_sync_height, None);
    }

    #[test]
    fn test_label_management() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        managed.set_label("Test Identity".to_string());
        assert_eq!(managed.label, Some("Test Identity".to_string()));

        managed.clear_label();
        assert_eq!(managed.label, None);
    }

    #[test]
    fn test_active_state() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        assert_eq!(managed.is_active, true);

        managed.deactivate();
        assert_eq!(managed.is_active, false);

        managed.activate();
        assert_eq!(managed.is_active, true);
    }

    #[test]
    fn test_sync_info() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        managed.update_sync_info(1234567890, 100000);
        assert_eq!(managed.last_sync_timestamp, Some(1234567890));
        assert_eq!(managed.last_sync_height, Some(100000));
    }

    #[test]
    fn test_needs_sync() {
        let identity = create_test_identity();
        let mut managed = ManagedIdentity::new(identity);

        // Never synced - needs sync
        assert_eq!(managed.needs_sync(1000, 100), true);

        // Just synced
        managed.update_sync_info(1000, 100);
        assert_eq!(managed.needs_sync(1050, 100), false);

        // Old sync - needs sync
        assert_eq!(managed.needs_sync(1200, 100), true);
    }
}
