use platform_value::Identifier;

use crate::prelude::Revision;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::batched_transition::document_transfer_transition::DocumentTransferTransitionV0;

pub trait DocumentTransferTransitionV0Methods: DocumentBaseTransitionAccessors {
    /// Returns a reference to the `revision` field of the `DocumentReplaceTransitionV0`.
    fn revision(&self) -> Revision;

    /// Sets the value of the `revision` field in the `DocumentReplaceTransitionV0`.
    fn set_revision(&mut self, revision: Revision);

    /// Returns the `recipient_owner_id` field of the `DocumentReplaceTransitionV0`.
    fn recipient_owner_id(&self) -> Identifier;

    /// Returns a reference to the `recipient_owner_id` field of the `DocumentReplaceTransitionV0`.
    fn recipient_owner_id_ref(&self) -> &Identifier;

    /// Sets the value of the `recipient_owner_id` field in the `DocumentReplaceTransitionV0`.
    fn set_recipient_owner_id(&mut self, recipient_owner_id: Identifier);
}

impl DocumentBaseTransitionAccessors for DocumentTransferTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base;
    }
}

impl DocumentTransferTransitionV0Methods for DocumentTransferTransitionV0 {
    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn recipient_owner_id(&self) -> Identifier {
        self.recipient_owner_id
    }

    fn recipient_owner_id_ref(&self) -> &Identifier {
        &self.recipient_owner_id
    }

    fn set_recipient_owner_id(&mut self, recipient_owner_id: Identifier) {
        self.recipient_owner_id = recipient_owner_id;
    }
}
