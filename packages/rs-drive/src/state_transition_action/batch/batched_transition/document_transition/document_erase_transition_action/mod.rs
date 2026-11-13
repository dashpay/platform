use derive_more::From;

use crate::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::{DocumentEraseTransitionActionAccessorsV0, DocumentEraseTransitionActionV0};

/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

/// document erase transition action
#[derive(Debug, Clone, From)]
pub enum DocumentEraseTransitionAction {
    /// v0
    V0(DocumentEraseTransitionActionV0),
}

impl DocumentEraseTransitionActionAccessorsV0 for DocumentEraseTransitionAction {
    fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentEraseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentEraseTransitionAction::V0(v0) => v0.base,
        }
    }
}
