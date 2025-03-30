use derive_more::From;
use dpp::identifier::Identifier;
use dpp::prelude::Revision;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_cancel_transition_action::v0::action::{TokenOrderCancelTransitionActionAccessorsV0, TokenOrderCancelTransitionActionV0};

/// Token release transition action
#[derive(Debug, Clone, From)]
pub enum TokenOrderCancelTransitionAction {
    /// v0
    V0(TokenOrderCancelTransitionActionV0),
}

impl TokenOrderCancelTransitionActionAccessorsV0 for TokenOrderCancelTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenOrderCancelTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenOrderCancelTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order_id(&self) -> Identifier {
        match self {
            TokenOrderCancelTransitionAction::V0(v0) => v0.order_id,
        }
    }

    fn order_revision(&self) -> Revision {
        match self {
            TokenOrderCancelTransitionAction::V0(v0) => v0.order_revision,
        }
    }
}
