use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::batched_transition::Entropy;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_sell_limit_transition_action::v0::action::{TokenOrderSellLimitTransitionActionAccessorsV0, TokenOrderSellLimitTransitionActionV0};

/// Token release transition action
#[derive(Debug, Clone, From)]
pub enum TokenOrderSellLimitTransitionAction {
    /// v0
    V0(TokenOrderSellLimitTransitionActionV0),
}

impl TokenOrderSellLimitTransitionActionAccessorsV0 for TokenOrderSellLimitTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenOrderSellLimitTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenOrderSellLimitTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order_id_entropy(&self) -> Entropy {
        match self {
            TokenOrderSellLimitTransitionAction::V0(v0) => v0.order_id_entropy,
        }
    }

    fn token_amount(&self) -> TokenAmount {
        match self {
            TokenOrderSellLimitTransitionAction::V0(v0) => v0.token_amount,
        }
    }

    fn token_min_price(&self) -> Credits {
        match self {
            TokenOrderSellLimitTransitionAction::V0(v0) => v0.token_min_price,
        }
    }
}
