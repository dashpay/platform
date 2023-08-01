use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use crate::state_transition_action::document::documents_batch::v0::DocumentsBatchTransitionActionV0;
use derive_more::From;
use platform_value::Identifier;

pub mod document_transition;
pub mod v0;

#[derive(Debug, Clone, From)]
pub enum DocumentsBatchTransitionAction<'a> {
    V0(DocumentsBatchTransitionActionV0<'a>),
}

impl<'a> DocumentsBatchTransitionAction<'a> {
    pub fn owner_id(&self) -> Identifier {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.owner_id,
        }
    }

    pub fn transitions(&self) -> &Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => &v0.transitions,
        }
    }

    pub fn transitions_owned(self) -> Vec<DocumentTransitionAction<'a>> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.transitions,
        }
    }
}
