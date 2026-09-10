/// transformer
pub mod transformer;

use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[derive(Debug, Clone)]
/// document erase transition action v0
pub struct DocumentEraseTransitionActionV0 {
    /// base
    pub base: DocumentBaseTransitionAction,
}

/// document erase transition action accessors v0
pub trait DocumentEraseTransitionActionAccessorsV0 {
    /// base
    fn base(&self) -> &DocumentBaseTransitionAction;
    /// base owned
    fn base_owned(self) -> DocumentBaseTransitionAction;
}
