use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use crate::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::batch_transition::batched_transition::document_update_price_transition::DocumentUpdatePriceTransitionV0;

pub trait DocumentUpdatePriceTransitionV0Methods: DocumentBaseTransitionAccessors {
    /// Returns a reference to the `revision` field of the `DocumentUpdatePriceTransitionV0`.
    fn revision(&self) -> Revision;

    /// Sets the value of the `revision` field in the `DocumentUpdatePriceTransitionV0`.
    fn set_revision(&mut self, revision: Revision);

    /// Returns a reference to the `price` field of the `DocumentUpdatePriceTransitionV0`.
    fn price(&self) -> Credits;

    /// Sets the value of the `price` field in the `DocumentUpdatePriceTransitionV0`.
    fn set_price(&mut self, price: Credits);
}

impl DocumentBaseTransitionAccessors for DocumentUpdatePriceTransitionV0 {
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

impl DocumentUpdatePriceTransitionV0Methods for DocumentUpdatePriceTransitionV0 {
    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn price(&self) -> Credits {
        self.price
    }

    fn set_price(&mut self, price: Credits) {
        self.price = price;
    }
}
