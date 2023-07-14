use crate::state_transition::documents_batch_transition::document_transition::DocumentDeleteTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::{DocumentDeleteTransitionAction, DocumentDeleteTransitionActionV0};

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionAction {
    fn from(value: DocumentDeleteTransition) -> Self {
        match value {
            DocumentDeleteTransition::V0(v0) => DocumentDeleteTransitionActionV0::from(v0).into(),
        }
    }
}

impl From<&DocumentDeleteTransition> for DocumentDeleteTransitionAction {
    fn from(value: &DocumentDeleteTransition) -> Self {
        match value {
            DocumentDeleteTransition::V0(v0) => DocumentDeleteTransitionActionV0::from(v0).into(),
        }
    }
}
