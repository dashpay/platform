mod action_type;
pub mod document_base_transition_action;
pub mod document_create_transition_action;
pub mod document_delete_transition_action;
pub mod document_replace_transition_action;

pub use dpp::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;

use derive_more::From;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
pub const DOCUMENT_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone, From)]
pub enum DocumentTransitionAction {
    CreateAction(DocumentCreateTransitionAction),
    ReplaceAction(DocumentReplaceTransitionAction),
    DeleteAction(DocumentDeleteTransitionAction),
}

impl DocumentTransitionAction {
    pub fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentTransitionAction::CreateAction(d) => &d.base(),
            DocumentTransitionAction::DeleteAction(d) => &d.base(),
            DocumentTransitionAction::ReplaceAction(d) => &d.base(),
        }
    }
}
