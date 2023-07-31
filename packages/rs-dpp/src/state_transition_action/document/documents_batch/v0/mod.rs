mod transformer;

use crate::identifier::Identifier;
use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;

#[derive(Default, Debug, Clone)]
pub struct DocumentsBatchTransitionActionV0<'a> {
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<DocumentTransitionAction<'a>>,
}
