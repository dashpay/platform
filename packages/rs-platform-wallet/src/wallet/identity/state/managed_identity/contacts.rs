//! Established contact management for ManagedIdentity

use super::ManagedIdentity;
use crate::{ContactRequest, EstablishedContact};
use dpp::prelude::Identifier;

#[allow(dead_code)]
impl ManagedIdentity {
    /// Add an established contact
    pub(crate) fn add_established_contact(&mut self, contact: EstablishedContact) {
        self.dashpay
            .established_contacts
            .insert(contact.contact_identity_id, contact);
    }

    /// Remove an established contact by identity ID
    pub(crate) fn remove_established_contact(
        &mut self,
        contact_id: &Identifier,
    ) -> Option<EstablishedContact> {
        self.dashpay.established_contacts.remove(contact_id)
    }

    /// Get an established contact by identity ID
    pub(crate) fn established_contact(
        &self,
        contact_id: &Identifier,
    ) -> Option<&EstablishedContact> {
        self.dashpay.established_contacts.get(contact_id)
    }

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

    /// Record a rotation when the established contact `contact_id` is about
    /// to carry `incoming` with another encrypted xpub than it has now (see
    /// `DashPayState::outbound_rekeyed`).
    pub(crate) fn note_outbound_rekey(
        &mut self,
        contact_id: &Identifier,
        incoming: &ContactRequest,
    ) {
        if self
            .dashpay
            .established_contacts
            .get(contact_id)
            .is_some_and(|contact| {
                contact.incoming_request.encrypted_public_key != incoming.encrypted_public_key
            })
        {
            self.dashpay.outbound_rekeyed.insert(*contact_id);
        }
    }

    /// Whether `contact_id` rotated since its outbound account was built.
    pub(crate) fn outbound_rekeyed(&self, contact_id: &Identifier) -> bool {
        self.dashpay.outbound_rekeyed.contains(contact_id)
    }

    /// Clear the rotation once the outbound account is built from the live
    /// request.
    pub(crate) fn clear_outbound_rekey(&mut self, contact_id: &Identifier) {
        self.dashpay.outbound_rekeyed.remove(contact_id);
    }
}
