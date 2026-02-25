//! Established contact management for ManagedIdentity

use super::ManagedIdentity;
use crate::EstablishedContact;
use dpp::prelude::Identifier;

#[allow(dead_code)]
impl ManagedIdentity {
    /// Add an established contact
    pub(crate) fn add_established_contact(&mut self, contact: EstablishedContact) {
        self.established_contacts
            .insert(contact.contact_identity_id, contact);
    }

    /// Remove an established contact by identity ID
    pub(crate) fn remove_established_contact(
        &mut self,
        contact_id: &Identifier,
    ) -> Option<EstablishedContact> {
        self.established_contacts.remove(contact_id)
    }

    /// Get an established contact by identity ID
    pub(crate) fn established_contact(
        &self,
        contact_id: &Identifier,
    ) -> Option<&EstablishedContact> {
        self.established_contacts.get(contact_id)
    }

    /// Get a mutable established contact by identity ID
    pub(crate) fn established_contact_mut(
        &mut self,
        contact_id: &Identifier,
    ) -> Option<&mut EstablishedContact> {
        self.established_contacts.get_mut(contact_id)
    }
}
