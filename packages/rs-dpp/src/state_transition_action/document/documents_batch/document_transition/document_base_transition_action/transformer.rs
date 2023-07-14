use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionV0};

impl From<DocumentBaseTransition> for DocumentBaseTransitionAction {
    fn from(value: DocumentBaseTransition) -> Self {
        match value {
            DocumentBaseTransition::V0(v0) => DocumentBaseTransitionActionV0::from(v0).into(),
        }
    }
}

impl From<&DocumentBaseTransition> for DocumentBaseTransitionAction {
    fn from(value: &DocumentBaseTransition) -> Self {
        match value {
            DocumentBaseTransition::V0(v0) => DocumentBaseTransitionActionV0::from(v0).into(),
        }
    }
}
