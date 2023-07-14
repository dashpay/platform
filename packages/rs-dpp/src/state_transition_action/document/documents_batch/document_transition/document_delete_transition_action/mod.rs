mod v0;

use crate::document::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::document::document_transition::DocumentDeleteTransition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
