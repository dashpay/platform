use crate::document::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use crate::document::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::document::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;

pub const DOCUMENT_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone)]
pub enum DocumentTransitionAction {
    CreateAction(DocumentCreateTransitionAction),
    ReplaceAction(DocumentReplaceTransitionAction),
    DeleteAction(DocumentDeleteTransitionAction),
}