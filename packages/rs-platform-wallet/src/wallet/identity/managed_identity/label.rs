//! Label management for ManagedIdentity

use super::ManagedIdentity;
use crate::wallet::persister::WalletPersister;

impl ManagedIdentity {
    /// Set the label for this identity.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn set_label(&mut self, label: String, persister: &WalletPersister) {
        self.label = Some(label);
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }

    /// Clear the label for this identity.
    ///
    /// Persists the resulting changeset via `persister` and returns `()`.
    pub fn clear_label(&mut self, persister: &WalletPersister) {
        self.label = None;
        let cs = self.snapshot_changeset();
        if let Err(e) = persister.store(cs.into()) {
            tracing::error!("Failed to persist changeset: {}", e);
        }
    }
}
