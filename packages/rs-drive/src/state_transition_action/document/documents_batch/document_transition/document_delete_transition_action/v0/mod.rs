pub mod transformer;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;

#[derive(Debug, Clone)]
pub struct DocumentDeleteTransitionActionV0 {
    pub base: DocumentBaseTransitionAction,
}

pub trait DocumentDeleteTransitionActionAccessorsV0 {
    fn base(&self) -> &DocumentBaseTransitionAction;
    fn base_owned(self) -> DocumentBaseTransitionAction;
}
