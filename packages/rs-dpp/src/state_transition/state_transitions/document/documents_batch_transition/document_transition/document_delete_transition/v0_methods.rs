use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::v0::v0_methods::DocumentDeleteTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentDeleteTransition;

impl DocumentDeleteTransitionV0Methods for DocumentDeleteTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentDeleteTransition::V0(v0) => v0.base = base,
        }
    }
}
