use crate::document::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use crate::document::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::document::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;
use crate::document::document_transition::{Action, DocumentBaseTransitionAction};
use derive_more::From;
use serde::{Deserialize, Serialize};

pub const DOCUMENT_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum DocumentTransitionAction {
    CreateAction(DocumentCreateTransitionAction),
    ReplaceAction(DocumentReplaceTransitionAction),
    DeleteAction(DocumentDeleteTransitionAction),
}

impl DocumentTransitionAction {
    pub fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentTransitionAction::CreateAction(d) => &d.base,
            DocumentTransitionAction::DeleteAction(d) => &d.base,
            DocumentTransitionAction::ReplaceAction(d) => &d.base,
        }
    }

    pub fn action(&self) -> Action {
        match self {
            DocumentTransitionAction::CreateAction(_) => Action::Create,
            DocumentTransitionAction::DeleteAction(_) => Action::Delete,
            DocumentTransitionAction::ReplaceAction(_) => Action::Replace,
        }
    }
}
