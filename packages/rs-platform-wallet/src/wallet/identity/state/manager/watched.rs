//! Watched (read-only) identities.
//!
//! Observable identities whose keys the wallet does not own.
//! Useful for tracking contacts' balances, names, and status
//! without being able to sign on their behalf.

use super::IdentityManager;
use crate::error::PlatformWalletError;
use crate::wallet::identity::state::managed_identity::{IdentityStatus, WatchedIdentity};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

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
