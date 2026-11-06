use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::tokens::token_order::TokenOrder;
use std::sync::Arc;

/// transformer module for the token fill ask transition action
pub mod transformer;
mod v0;

pub use v0::*;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token fill ask transition action: a purchase from an open listing (protocol version 15)
#[derive(Debug, Clone, From)]
pub enum TokenFillAskTransitionAction {
    /// v0
    V0(TokenFillAskTransitionActionV0),
}

impl TokenFillAskTransitionActionAccessorsV0 for TokenFillAskTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenFillAskTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenFillAskTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order(&self) -> &TokenOrder {
        match self {
            TokenFillAskTransitionAction::V0(v0) => &v0.order,
        }
    }

    fn fill_amount(&self) -> TokenAmount {
        match self {
            TokenFillAskTransitionAction::V0(v0) => v0.fill_amount,
        }
    }

    fn credits(&self) -> Credits {
        match self {
            TokenFillAskTransitionAction::V0(v0) => v0.credits,
        }
    }

    fn updated_order_document(&self) -> Option<&Document> {
        match self {
            TokenFillAskTransitionAction::V0(v0) => v0.updated_order_document.as_ref(),
        }
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            TokenFillAskTransitionAction::V0(v0) => &v0.token_market_contract,
        }
    }
}
