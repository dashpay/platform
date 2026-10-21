use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use std::sync::Arc;

/// transformer module for the token place ask transition action
pub mod transformer;
mod v0;

pub use v0::*;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token place ask transition action: a listing of a tradeable token (protocol version 15)
#[derive(Debug, Clone, From)]
pub enum TokenPlaceAskTransitionAction {
    /// v0
    V0(TokenPlaceAskTransitionActionV0),
}

impl TokenPlaceAskTransitionActionAccessorsV0 for TokenPlaceAskTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order_id(&self) -> Identifier {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => v0.order_id,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => v0.amount,
        }
    }

    fn price(&self) -> Credits {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => v0.price,
        }
    }

    fn min_fill(&self) -> TokenAmount {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => v0.min_fill,
        }
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => v0.expires_at,
        }
    }

    fn order_document(&self) -> &Document {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => &v0.order_document,
        }
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            TokenPlaceAskTransitionAction::V0(v0) => &v0.token_market_contract,
        }
    }
}
