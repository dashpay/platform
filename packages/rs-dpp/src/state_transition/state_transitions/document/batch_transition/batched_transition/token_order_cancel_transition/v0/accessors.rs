use platform_value::Identifier;
use crate::balances::credits::TokenAmount;
use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::batched_transition::token_order_cancel_transition::v0::transition::TokenOrderCancelTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderCancelTransitionV0 {
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

pub trait TokenOrderCancelTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Order ID to cancel
    fn order_id(&self) -> Identifier;

    /// Set order ID to cancel
    fn set_order_id(&mut self, id: Identifier);

    /// Order revision to cancel
    fn order_revision(&self) -> Revision;

    /// Set order revision
    fn set_order_revision(&mut self, revision: Revision);
}

impl TokenOrderCancelTransitionV0Methods for TokenOrderCancelTransitionV0 {
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
}
