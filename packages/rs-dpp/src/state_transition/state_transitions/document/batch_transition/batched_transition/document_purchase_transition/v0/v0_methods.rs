use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::batched_transition::document_purchase_transition::DocumentPurchaseTransitionV0;

pub trait DocumentPurchaseTransitionV0Methods: DocumentBaseTransitionAccessors {
    /// Returns a reference to the `revision` field of the `DocumentReplaceTransitionV0`.
    fn revision(&self) -> Revision;

    /// Sets the value of the `revision` field in the `DocumentReplaceTransitionV0`.
    fn set_revision(&mut self, revision: Revision);
    fn price(&self) -> Credits;
}

impl DocumentBaseTransitionAccessors for DocumentPurchaseTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base;
    }
}

impl DocumentPurchaseTransitionV0Methods for DocumentPurchaseTransitionV0 {
    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn price(&self) -> Credits {
        self.price
    }
}
