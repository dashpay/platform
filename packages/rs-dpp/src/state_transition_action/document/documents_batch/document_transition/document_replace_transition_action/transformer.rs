use crate::identity::TimestampMillis;
use crate::state_transition::documents_batch_transition::document_transition::DocumentReplaceTransition;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionV0};

impl DocumentReplaceTransitionAction {
    pub fn from_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransition,
        originally_created_at: Option<TimestampMillis>,
    ) -> Self {
        match document_replace_transition {
            DocumentReplaceTransition::V0(v0) => {
                DocumentReplaceTransitionActionV0::from_document_replace_transition(
                    v0,
                    originally_created_at,
                )
                .into()
            }
        }
    }
}
