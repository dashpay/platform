use platform_value::Identifier;
use crate::prelude::Revision;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransferTransition;

impl DocumentTransferTransitionV0Methods for DocumentTransferTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentTransferTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentTransferTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentTransferTransition::V0(v0) => v0.base = base,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            DocumentTransferTransition::V0(v0) => v0.revision,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            DocumentTransferTransition::V0(v0) => v0.revision = revision,
        }
    }

    fn recipient_owner_id(&self) -> Identifier {
        match self {
            DocumentTransferTransition::V0(v0) => v0.recipient_owner_id,
        }
    }

    fn recipient_owner_id_ref(&self) -> &Identifier {
        match self {
            DocumentTransferTransition::V0(v0) => &v0.recipient_owner_id,
        }
    }

    fn set_recipient_owner_id(&mut self, recipient_owner_id: Identifier) {
        match self {
            DocumentTransferTransition::V0(v0) => v0.recipient_owner_id = recipient_owner_id,
        }
    }
}
