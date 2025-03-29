use platform_value::Identifier;
use crate::prelude::Revision;
use crate::state_transition::batch_transition::batched_transition::token_order_cancel_transition::transition::TokenOrderCancelLimitTransition;
use crate::state_transition::batch_transition::batched_transition::token_order_cancel_transition::v0::accessors::TokenOrderCancelLimitTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;

impl TokenBaseTransitionAccessors for TokenOrderCancelLimitTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            Self::V0(v0) => v0.base(),
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            Self::V0(v0) => v0.base_mut(),
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            Self::V0(v0) => v0.set_base(base),
        }
    }
}

impl TokenOrderCancelLimitTransitionV0Methods for TokenOrderCancelLimitTransition {
    fn order_id(&self) -> Identifier {
        match self {
            TokenOrderCancelLimitTransition::V0(v0) => v0.order_id(),
        }
    }

    fn set_order_id(&mut self, id: Identifier) {
        match self {
            TokenOrderCancelLimitTransition::V0(v0) => v0.set_order_id(id),
        }
    }

    fn order_revision(&self) -> Revision {
        match self {
            TokenOrderCancelLimitTransition::V0(v0) => v0.order_revision(),
        }
    }

    fn set_order_revision(&mut self, revision: Revision) {
        match self {
            TokenOrderCancelLimitTransition::V0(v0) => v0.set_order_revision(revision),
        }
    }
}
