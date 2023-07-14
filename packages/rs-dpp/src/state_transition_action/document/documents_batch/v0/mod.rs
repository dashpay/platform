mod transformer;

use crate::identifier::Identifier;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransitionAction;

#[derive(Default, Debug, Clone)]
pub struct DocumentsBatchTransitionActionV0 {
    /// The owner making the transitions
    pub owner_id: Identifier,
    /// The inner transitions
    pub transitions: Vec<DocumentTransitionAction>,
}
