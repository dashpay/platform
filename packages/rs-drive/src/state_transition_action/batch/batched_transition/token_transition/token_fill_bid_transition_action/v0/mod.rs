mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::balances::credits::TokenAmount;
use dpp::document::Document;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::tokens::token_order::TokenOrder;
use std::sync::Arc;

/// Token fill bid transition action v0
#[derive(Debug, Clone)]
pub struct TokenFillBidTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The offer as it stands before the fill
    pub order: TokenOrder,
    /// The amount the fill takes, in base units
    pub fill_amount: TokenAmount,
    /// The credits the seller receives out of the offer's escrow for it
    pub credits: Credits,
    /// What the offer holds in escrow after the fill. When the fill closes the offer, this is
    /// what its rounding left over, returned to the buyer.
    pub escrow_after: Credits,
    /// The offer's document after a partial fill, stamped with the block; `None` when the fill
    /// takes everything still wanted and the offer closes
    pub updated_order_document: Option<Document>,
    /// The token market contract, whose `bid` document records the offer
    pub token_market_contract: Arc<DataContractFetchInfo>,
}

/// Accessors for `TokenFillBidTransitionActionV0`
pub trait TokenFillBidTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// The offer as it stands before the fill
    fn order(&self) -> &TokenOrder;

    /// The amount the fill takes
    fn fill_amount(&self) -> TokenAmount;

    /// The credits the seller receives
    fn credits(&self) -> Credits;

    /// What the offer holds in escrow after the fill
    fn escrow_after(&self) -> Credits;

    /// The offer's document after a partial fill, `None` when the offer closes
    fn updated_order_document(&self) -> Option<&Document>;

    /// The token market contract
    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo>;

    /// What the offer still wants after the fill
    fn remaining_after(&self) -> TokenAmount {
        self.order().remaining.saturating_sub(self.fill_amount())
    }

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

impl TokenFillBidTransitionActionAccessorsV0 for TokenFillBidTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn order(&self) -> &TokenOrder {
        &self.order
    }

    fn fill_amount(&self) -> TokenAmount {
        self.fill_amount
    }

    fn credits(&self) -> Credits {
        self.credits
    }

    fn escrow_after(&self) -> Credits {
        self.escrow_after
    }

    fn updated_order_document(&self) -> Option<&Document> {
        self.updated_order_document.as_ref()
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        &self.token_market_contract
    }
}
