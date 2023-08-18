use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;

pub trait DocumentsBatchTransitionAccessorsV0 {
    fn transitions(&self) -> &Vec<DocumentTransition>;
    fn transitions_slice(&self) -> &[DocumentTransition];
}
