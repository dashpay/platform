//! Label management for ManagedIdentity

use super::ManagedIdentity;
use crate::changeset::IdentityChangeSet;

impl ManagedIdentity {
    /// Set the label for this identity.
    ///
    /// Returns an [`IdentityChangeSet`] carrying a full snapshot of the
    /// updated identity.
    pub fn set_label(&mut self, label: String) -> IdentityChangeSet {
        self.label = Some(label);
        self.snapshot_changeset()
    }

    /// Clear the label for this identity.
    ///
    /// Returns an [`IdentityChangeSet`] carrying a full snapshot of the
    /// updated identity.
    pub fn clear_label(&mut self) -> IdentityChangeSet {
        self.label = None;
        self.snapshot_changeset()
    }
}
