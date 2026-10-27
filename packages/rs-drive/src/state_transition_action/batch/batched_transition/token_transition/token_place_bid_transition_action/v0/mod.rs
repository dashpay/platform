mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use std::sync::Arc;

/// Token place bid transition action v0
#[derive(Debug, Clone)]
pub struct TokenPlaceBidTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The offer's order id, derived from the token, the buyer and the nonce
    pub order_id: Identifier,
    /// The amount wanted, in base units
    pub amount: TokenAmount,
    /// The credits offered per whole token
    pub price: Credits,
    /// The smallest fill the buyer accepts unless it takes everything still wanted
    pub min_fill: TokenAmount,
    /// The token's decimals the offer was signed at, which its price is at and its order
    /// records; the batch's advanced-structure validation refuses them unless they are the
    /// token's (`TokenOrderDecimalsMismatchError`)
    pub decimals: u8,
    /// What the offer is worth, rounded up: the credits it moves from the buyer's balance into
    /// escrow
    pub escrow: Credits,
    /// The offer's `bid` document, stamped with the block that places it
    pub order_document: Document,
    /// The token market contract, whose `bid` document records the offer
    pub token_market_contract: Arc<DataContractFetchInfo>,
}

/// Accessors for `TokenPlaceBidTransitionActionV0`
pub trait TokenPlaceBidTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// The offer's order id
    fn order_id(&self) -> Identifier;

    /// The amount wanted
    fn amount(&self) -> TokenAmount;

    /// The credits offered per whole token
    fn price(&self) -> Credits;

    /// The smallest fill the buyer accepts
    fn min_fill(&self) -> TokenAmount;

    /// The token's decimals the offer was signed at
    fn decimals(&self) -> u8;

    /// The credits the offer holds in escrow
    fn escrow(&self) -> Credits;

    /// The offer's `bid` document
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

impl TokenPlaceBidTransitionActionAccessorsV0 for TokenPlaceBidTransitionActionV0 {
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

    fn decimals(&self) -> u8 {
        self.decimals
    }

    fn escrow(&self) -> Credits {
        self.escrow
    }

    fn order_document(&self) -> &Document {
        &self.order_document
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        &self.token_market_contract
    }
}
