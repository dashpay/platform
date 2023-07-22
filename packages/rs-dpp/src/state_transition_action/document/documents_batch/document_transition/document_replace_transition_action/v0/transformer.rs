use crate::identity::TimestampMillis;
use crate::state_transition::documents_batch_transition::document_transition::document_replace_transition::DocumentReplaceTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::v0::DocumentReplaceTransitionActionV0;

impl DocumentReplaceTransitionActionV0 {
    pub fn from_document_replace_transition(
        document_replace_transition: &DocumentReplaceTransitionV0,
        originally_created_at: Option<TimestampMillis>,
    ) -> Self {
        let DocumentReplaceTransitionV0 {
            base,
            revision,
            updated_at,
            data,
            ..
        } = document_replace_transition;
        DocumentReplaceTransitionActionV0 {
            base: base.into(),
            revision: *revision,
            created_at: originally_created_at,
            updated_at: *updated_at,
            data: data.clone().unwrap_or_default(),
        }
    }
}
