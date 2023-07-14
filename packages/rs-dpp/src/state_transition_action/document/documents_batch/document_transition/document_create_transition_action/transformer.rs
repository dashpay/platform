use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionV0};

impl From<DocumentCreateTransition> for DocumentCreateTransitionAction {
    fn from(value: DocumentCreateTransition) -> Self {
        match value {
            DocumentCreateTransition::V0(v0) => DocumentCreateTransitionActionV0::from(v0).into(),
        }
    }
}

impl From<&DocumentCreateTransition> for DocumentCreateTransitionAction {
    fn from(value: &DocumentCreateTransition) -> Self {
        match value {
            DocumentCreateTransition::V0(v0) => DocumentCreateTransitionActionV0::from(v0).into(),
        }
    }
}
