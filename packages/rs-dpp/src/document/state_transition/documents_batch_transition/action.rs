use crate::document::document_transition::DocumentTransitionAction;
use crate::identifier::Identifier;

pub const DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone)]
pub struct DocumentsBatchTransitionAction {
    /// The version of the transition
    pub version: u32,
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<DocumentTransitionAction>,
}
