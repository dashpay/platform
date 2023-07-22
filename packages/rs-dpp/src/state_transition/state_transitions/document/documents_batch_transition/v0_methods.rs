use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::state_transition::documents_batch_transition::v0::v0_methods::DocumentsBatchTransitionV0Methods;
use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;

impl DocumentsBatchTransitionV0Methods for DocumentsBatchTransition {
    fn get_transitions(&self) -> &Vec<DocumentTransition> {
        match self {
            DocumentsBatchTransition::V0(v0) => &v0.transitions,
        }
    }

    fn get_transitions_slice(&self) -> &[DocumentTransition] {
        match self {
            DocumentsBatchTransition::V0(v0) => v0.transitions.as_slice(),
        }
    }
}
