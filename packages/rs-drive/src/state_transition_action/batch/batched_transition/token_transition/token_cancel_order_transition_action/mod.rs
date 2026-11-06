use derive_more::From;
use dpp::tokens::token_order::TokenOrder;
use std::sync::Arc;

/// transformer module for the token cancel order transition action
pub mod transformer;
mod v0;

pub use v0::*;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token cancel order transition action: a maker cancelling an open order (protocol version 15)
#[derive(Debug, Clone, From)]
pub enum TokenCancelOrderTransitionAction {
    /// v0
    V0(TokenCancelOrderTransitionActionV0),
}

impl TokenCancelOrderTransitionActionAccessorsV0 for TokenCancelOrderTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenCancelOrderTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenCancelOrderTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order(&self) -> &TokenOrder {
        match self {
            TokenCancelOrderTransitionAction::V0(v0) => &v0.order,
        }
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            TokenCancelOrderTransitionAction::V0(v0) => &v0.token_market_contract,
        }
    }
}
