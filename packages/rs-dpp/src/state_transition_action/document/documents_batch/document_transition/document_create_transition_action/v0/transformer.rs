use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionActionV0;

impl From<DocumentCreateTransitionV0> for DocumentCreateTransitionActionV0 {
    fn from(value: DocumentCreateTransitionV0) -> Self {
        let DocumentCreateTransitionV0 {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        DocumentCreateTransitionActionV0 {
            base: base.into(),
            created_at,
            updated_at,
            data: data.unwrap_or_default(),
        }
    }
}

impl From<&DocumentCreateTransitionV0> for DocumentCreateTransitionActionV0 {
    fn from(value: &DocumentCreateTransitionV0) -> Self {
        let DocumentCreateTransitionV0 {
            base,
            created_at,
            updated_at,
            data,
            ..
        } = value;
        DocumentCreateTransitionActionV0 {
            base: base.into(),
            created_at: *created_at,
            updated_at: *updated_at,
            data: data.clone().unwrap_or_default(),
        }
    }
}
