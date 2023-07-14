use crate::state_transition_action::document::documents_batch::v0::DocumentsBatchTransitionActionV0;

mod document_transition;
pub mod v0;

#[derive(Debug, Clone)]
pub enum DocumentsBatchTransitionAction {
    V0(DocumentsBatchTransitionActionV0),
}
