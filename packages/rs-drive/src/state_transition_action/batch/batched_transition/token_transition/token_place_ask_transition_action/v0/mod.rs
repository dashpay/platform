mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use std::sync::Arc;

/// Token place ask transition action v0
#[derive(Debug, Clone)]
pub struct TokenPlaceAskTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The listing's order id, derived from the token, the seller and the nonce
    pub order_id: Identifier,
    /// The amount listed, in base units
    pub amount: TokenAmount,
    /// The credits asked per whole token
    pub price: Credits,
    /// The smallest fill the seller accepts unless it takes everything open
    pub min_fill: TokenAmount,
    /// The block time from which the listing can no longer be filled
    pub expires_at: Option<TimestampMillis>,
    /// The token's decimals the listing was signed at, which its price is at and its order
    /// records; the structure validation refuses them unless they are the token's
    pub decimals: u8,
    /// The listing's `ask` document, stamped with the block that places it
    pub order_document: Document,
    /// The token market contract, whose `ask` document records the listing
    pub token_market_contract: Arc<DataContractFetchInfo>,
}

/// Accessors for `TokenPlaceAskTransitionActionV0`
pub trait TokenPlaceAskTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// The listing's order id
    fn order_id(&self) -> Identifier;

    /// The amount listed
    fn amount(&self) -> TokenAmount;

    /// The credits asked per whole token
    fn price(&self) -> Credits;

    /// The smallest fill the seller accepts
    fn min_fill(&self) -> TokenAmount;

    /// The block time from which the listing can no longer be filled
    fn expires_at(&self) -> Option<TimestampMillis>;

    /// The token's decimals the listing was signed at
    fn decimals(&self) -> u8;

    /// The listing's `ask` document
    fn order_document(&self) -> &Document;

    /// The token market contract
    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo>;

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns a reference to the data contract fetch info of the token's contract
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }
}

impl TokenPlaceAskTransitionActionAccessorsV0 for TokenPlaceAskTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn order_id(&self) -> Identifier {
        self.order_id
    }

    fn amount(&self) -> TokenAmount {
        self.amount
    }

    fn price(&self) -> Credits {
        self.price
    }

    fn min_fill(&self) -> TokenAmount {
        self.min_fill
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        self.expires_at
    }

    fn decimals(&self) -> u8 {
        self.decimals
    }

    fn order_document(&self) -> &Document {
        &self.order_document
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        &self.token_market_contract
    }
}
