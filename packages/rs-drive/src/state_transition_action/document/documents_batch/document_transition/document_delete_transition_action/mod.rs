use derive_more::From;

use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::{DocumentDeleteTransitionActionAccessorsV0, DocumentDeleteTransitionActionV0};

pub mod transformer;
pub mod v0;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[derive(Debug, Clone, From)]
pub enum DocumentDeleteTransitionAction {
    V0(DocumentDeleteTransitionActionV0),
}

impl DocumentDeleteTransitionActionAccessorsV0 for DocumentDeleteTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentDeleteTransitionAction::V0(v0) => v0.base,
        }
    }
}
