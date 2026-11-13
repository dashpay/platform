use crate::state_transition::batch_transition::batched_transition::DocumentEraseTransition;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

impl DocumentBaseTransitionAccessors for DocumentEraseTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentEraseTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentEraseTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentEraseTransition::V0(v0) => v0.base = base,
        }
    }
}
