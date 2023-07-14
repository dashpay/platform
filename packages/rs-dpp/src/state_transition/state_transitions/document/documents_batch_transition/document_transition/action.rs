use derive_more::From;
use serde::{Deserialize, Serialize};
use crate::state_transition::documents_batch_transition::document_transition::{DocumentBaseTransitionAction, DocumentCreateTransitionAction, DocumentDeleteTransitionAction, DocumentReplaceTransitionAction};

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
