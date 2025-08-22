use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenDirectPurchaseTransitionAction {
    /// v0
    V0(TokenDirectPurchaseTransitionActionV0),
}

impl TokenDirectPurchaseTransitionActionAccessorsV0 for TokenDirectPurchaseTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.base,
        }
    }
    fn token_count(&self) -> TokenAmount {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.token_count,
        }
    }

    fn set_token_count(&mut self, amount: TokenAmount) {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.token_count = amount,
        }
    }

    fn total_agreed_price(&self) -> Credits {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.total_agreed_price,
        }
    }

    fn set_total_agreed_price(&mut self, agreed_price: Credits) {
        match self {
            TokenDirectPurchaseTransitionAction::V0(v0) => v0.total_agreed_price = agreed_price,
        }
    }
}
