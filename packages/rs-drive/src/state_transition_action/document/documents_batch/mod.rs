use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use crate::state_transition_action::document::documents_batch::v0::DocumentsBatchTransitionActionV0;
use derive_more::From;
use dpp::platform_value::Identifier;

/// document transition
pub mod document_transition;
/// v0
pub mod v0;

/// documents batch transition action
#[derive(Debug, Clone, From)]
pub enum DocumentsBatchTransitionAction {
    /// v0
    V0(DocumentsBatchTransitionActionV0),
}

impl DocumentsBatchTransitionAction {
    /// owner id
    pub fn owner_id(&self) -> Identifier {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.owner_id,
        }
    }

    /// transitions
    pub fn transitions(&self) -> &Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => &v0.transitions,
        }
    }

    /// transitions owned
    pub fn transitions_owned(self) -> Vec<DocumentTransitionAction> {
        match self {
            DocumentsBatchTransitionAction::V0(v0) => v0.transitions,
        }
    }
}
