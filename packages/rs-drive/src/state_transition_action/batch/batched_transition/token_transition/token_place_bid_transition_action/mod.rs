use derive_more::From;
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use std::sync::Arc;

/// transformer module for the token place bid transition action
pub mod transformer;
mod v0;

pub use v0::*;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;

/// Token place bid transition action: an offer for a tradeable token (protocol version 15)
#[derive(Debug, Clone, From)]
pub enum TokenPlaceBidTransitionAction {
    /// v0
    V0(TokenPlaceBidTransitionActionV0),
}

impl TokenPlaceBidTransitionActionAccessorsV0 for TokenPlaceBidTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.base,
        }
    }

    fn order_id(&self) -> Identifier {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.order_id,
        }
    }

    fn amount(&self) -> TokenAmount {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.amount,
        }
    }

    fn price(&self) -> Credits {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.price,
        }
    }

    fn min_fill(&self) -> TokenAmount {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.min_fill,
        }
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.expires_at,
        }
    }

    fn escrow(&self) -> Credits {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => v0.escrow,
        }
    }

    fn order_document(&self) -> &Document {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => &v0.order_document,
        }
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            TokenPlaceBidTransitionAction::V0(v0) => &v0.token_market_contract,
        }
    }
}
