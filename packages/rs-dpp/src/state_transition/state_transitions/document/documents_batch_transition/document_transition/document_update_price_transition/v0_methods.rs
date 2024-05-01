use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use crate::state_transition::documents_batch_transition::document_transition::DocumentUpdatePriceTransition;

impl DocumentUpdatePriceTransitionV0Methods for DocumentUpdatePriceTransition {
    fn base(&self) -> &DocumentBaseTransition {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => v0.base = base,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => v0.revision,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => v0.revision = revision,
        }
    }

    fn price(&self) -> Credits {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => v0.price,
        }
    }

    fn set_price(&mut self, price: Credits) {
        match self {
            DocumentUpdatePriceTransition::V0(v0) => v0.price = price,
        }
    }
}
