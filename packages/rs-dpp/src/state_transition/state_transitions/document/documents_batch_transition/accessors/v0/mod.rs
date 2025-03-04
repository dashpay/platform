use crate::state_transition::state_transitions::document::documents_batch_transition::batched_transition::DocumentTransition;

pub trait DocumentsBatchTransitionAccessorsV0 {
    fn transitions(&self) -> &Vec<DocumentTransition>;
    fn transitions_slice(&self) -> &[DocumentTransition];
}
