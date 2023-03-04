use serde::{Deserialize, Serialize};
use crate::document::document_transition::action::DocumentTransitionAction::{CreateAction, DeleteAction, ReplaceAction};
use crate::document::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use crate::document::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::document::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;
use crate::prelude::DocumentTransition;


pub const DOCUMENT_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocumentTransitionAction {
    CreateAction(DocumentCreateTransitionAction),
    ReplaceAction(DocumentReplaceTransitionAction),
    DeleteAction(DocumentDeleteTransitionAction),
}

impl From<DocumentTransition> for DocumentTransitionAction {
    fn from(value: DocumentTransition) -> Self {
        match value {
            DocumentTransition::Create(document_create_transition) => CreateAction(document_create_transition.into()),
            DocumentTransition::Replace(document_replace_transition) => ReplaceAction(document_replace_transition.into()),
            DocumentTransition::Delete(document_delete_transition) => DeleteAction(document_delete_transition.into()),
        }
    }
}