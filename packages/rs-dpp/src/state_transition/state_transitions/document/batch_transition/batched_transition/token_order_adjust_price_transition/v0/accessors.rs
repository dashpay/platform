use platform_value::Identifier;
use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::batched_transition::token_order_adjust_price_transition::v0::transition::TokenOrderAdjustPriceTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderAdjustPriceTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenOrderAdjustPriceTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Order ID to adjust price for
    fn order_id(&self) -> Identifier;

    /// Set order ID to adjust price for
    fn set_order_id(&mut self, id: Identifier);

    /// Order revision to adjust price for
    fn order_revision(&self) -> Revision;

    /// Set order revision to adjust price for
    fn set_order_revision(&mut self, revision: Revision);

    /// New price for specified order
    fn token_price(&self) -> Credits;

    /// Set new price for specified order
    fn set_token_price(&mut self, price: Credits);
}

impl TokenOrderAdjustPriceTransitionV0Methods for TokenOrderAdjustPriceTransitionV0 {
    fn order_id(&self) -> Identifier {
        self.order_id
    }

    fn set_order_id(&mut self, id: Identifier) {
        self.order_id = id;
    }

    fn order_revision(&self) -> Revision {
        self.order_revision
    }

    fn set_order_revision(&mut self, revision: Revision) {
        self.order_revision = revision;
    }

    fn token_price(&self) -> Credits {
        self.token_price
    }

    fn set_token_price(&mut self, price: Credits) {
        self.token_price = price;
    }
}
