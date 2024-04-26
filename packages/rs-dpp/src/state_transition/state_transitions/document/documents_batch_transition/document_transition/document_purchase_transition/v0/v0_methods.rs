use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_purchase_transition::DocumentPurchaseTransitionV0;

pub trait DocumentPurchaseTransitionV0Methods {
    /// Returns a reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base(&self) -> &DocumentBaseTransition;
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;

    /// Sets the value of the `base` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `base` - A value of type `DocumentBaseTransition` to set.
    fn set_base(&mut self, base: DocumentBaseTransition);

    /// Returns a reference to the `revision` field of the `DocumentReplaceTransitionV0`.
    fn revision(&self) -> Revision;

    /// Sets the value of the `revision` field in the `DocumentReplaceTransitionV0`.
    fn set_revision(&mut self, revision: Revision);
    fn price(&self) -> Credits;
}

impl DocumentPurchaseTransitionV0Methods for DocumentPurchaseTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base
    }

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
