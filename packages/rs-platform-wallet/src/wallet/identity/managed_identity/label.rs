//! Label management for ManagedIdentity

use super::ManagedIdentity;
use crate::changeset::{IdentityChangeSet, IdentityEntry};

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

    /// Helper: produce an [`IdentityChangeSet`] containing a full
    /// [`IdentityEntry`] snapshot of `self`.
    pub(crate) fn snapshot_changeset(&self) -> IdentityChangeSet {
        let mut cs = IdentityChangeSet::default();
        cs.identities
            .insert(self.id(), IdentityEntry::from_managed(self));
        cs
    }
}
