use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::batched_transition::Entropy;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::token_transition::token_order_buy_limit_transition_action::v0::action::{TokenOrderBuyLimitTransitionActionAccessorsV0, TokenOrderBuyLimitTransitionActionV0};

/// Token release transition action
#[derive(Debug, Clone, From)]
pub enum TokenOrderBuyLimitTransitionAction {
    /// v0
    V0(TokenOrderBuyLimitTransitionActionV0),
}

impl TokenOrderBuyLimitTransitionActionAccessorsV0 for TokenOrderBuyLimitTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenOrderBuyLimitTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenOrderBuyLimitTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order_id_entropy(&self) -> Entropy {
        match self {
            TokenOrderBuyLimitTransitionAction::V0(v0) => v0.order_id_entropy,
        }
    }

    fn token_amount(&self) -> TokenAmount {
        match self {
            TokenOrderBuyLimitTransitionAction::V0(v0) => v0.token_amount,
        }
    }

    fn token_max_price(&self) -> Credits {
        match self {
            TokenOrderBuyLimitTransitionAction::V0(v0) => v0.token_max_price,
        }
    }
}
