//! Established contact management for ManagedIdentity

use super::ManagedIdentity;
use crate::EstablishedContact;
use dpp::prelude::Identifier;

impl ManagedIdentity {
    /// Get a mutable established contact by identity ID.
    ///
    /// Escape hatch for per-contact sub-field mutation (channel-broken
    /// flag, account labels, external-account registration). Callers that
    /// mutate through it own the persistence step (a hand-built
    /// [`ContactChangeSet`](crate::changeset::ContactChangeSet)) — map
    /// *membership* still only changes through the invariant-holding
    /// methods and their `apply_*` replay counterparts.
    pub fn established_contact_mut(
        &mut self,
        contact_id: &Identifier,
    ) -> Option<&mut EstablishedContact> {
        self.dashpay.established_contacts.get_mut(contact_id)
    }
}
