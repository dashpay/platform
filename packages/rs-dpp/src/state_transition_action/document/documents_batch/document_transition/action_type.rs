use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;

// @append-only
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum DocumentTransitionActionType {
    Create,
    Replace,
    Delete,
}

impl<'a> DocumentTransitionAction<'a> {
    pub fn action_type(&self) -> DocumentTransitionActionType {
        match self {
            DocumentTransitionAction::CreateAction(_) => DocumentTransitionActionType::Create,
            DocumentTransitionAction::DeleteAction(_) => DocumentTransitionActionType::Delete,
            DocumentTransitionAction::ReplaceAction(_) => DocumentTransitionActionType::Replace,
        }
    }
}
