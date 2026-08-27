use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::batched_transition::DocumentDeleteTransition;

impl DocumentBaseTransitionAccessors for DocumentDeleteTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &v0.base,
            DocumentDeleteTransition::V1(v1) => &v1.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentDeleteTransition::V0(v0) => &mut v0.base,
            DocumentDeleteTransition::V1(v1) => &mut v1.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentDeleteTransition::V0(v0) => v0.base = base,
            DocumentDeleteTransition::V1(v1) => v1.base = base,
        }
    }
}
