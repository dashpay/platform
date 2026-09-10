use crate::state_transition::batch_transition::batched_transition::document_erase_transition::DocumentEraseTransitionV0;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;

impl DocumentBaseTransitionAccessors for DocumentEraseTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base
    }
}
