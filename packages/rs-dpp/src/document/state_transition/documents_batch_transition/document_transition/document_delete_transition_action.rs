use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentDeleteTransition;

#[derive(Debug, Clone, Default)]
pub struct DocumentDeleteTransitionAction {
    pub base: DocumentBaseTransitionAction,
}

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionAction {
    fn from(value: DocumentDeleteTransition) -> Self {
        let DocumentDeleteTransition { base } = value;
        DocumentDeleteTransitionAction { base: base.into() }
    }
}

impl From<&DocumentDeleteTransition> for DocumentDeleteTransitionAction {
    fn from(value: &DocumentDeleteTransition) -> Self {
        let DocumentDeleteTransition { base } = value;
        DocumentDeleteTransitionAction { base: base.into() }
    }
}
