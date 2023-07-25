use crate::state_transition::documents_batch_transition::document_transition::document_delete_transition::DocumentDeleteTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionV0;

impl From<DocumentDeleteTransitionV0> for DocumentDeleteTransitionActionV0 {
    fn from(value: DocumentDeleteTransitionV0) -> Self {
        let DocumentDeleteTransitionV0 { base } = value;
        DocumentDeleteTransitionActionV0 { base: base.into() }
    }
}

impl From<&DocumentDeleteTransitionV0> for DocumentDeleteTransitionActionV0 {
    fn from(value: &DocumentDeleteTransitionV0) -> Self {
        let DocumentDeleteTransitionV0 { base } = value;
        DocumentDeleteTransitionActionV0 { base: base.into() }
    }
}
