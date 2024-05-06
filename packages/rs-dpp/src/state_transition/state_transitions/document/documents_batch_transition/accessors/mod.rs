mod v0;

use crate::state_transition::state_transitions::document::documents_batch_transition::document_transition::DocumentTransition;
use crate::state_transition::state_transitions::document::documents_batch_transition::DocumentsBatchTransition;
pub use v0::*;

impl DocumentsBatchTransitionAccessorsV0 for DocumentsBatchTransition {
    fn transitions(&self) -> &Vec<DocumentTransition> {
        match self {
            DocumentsBatchTransition::V0(v0) => &v0.transitions,
        }
    }

    fn transitions_slice(&self) -> &[DocumentTransition] {
        match self {
            DocumentsBatchTransition::V0(v0) => v0.transitions.as_slice(),
        }
    }
}
