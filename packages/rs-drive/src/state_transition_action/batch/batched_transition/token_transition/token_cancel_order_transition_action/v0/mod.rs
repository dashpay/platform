mod transformer;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::identifier::Identifier;
use dpp::tokens::token_order::TokenOrder;
use std::sync::Arc;

/// Token cancel order transition action v0
#[derive(Debug, Clone)]
pub struct TokenCancelOrderTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The order as it stands before the cancel
    pub order: TokenOrder,
    /// The token market contract, whose document records the order
    pub token_market_contract: Arc<DataContractFetchInfo>,
}

/// Accessors for `TokenCancelOrderTransitionActionV0`
pub trait TokenCancelOrderTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// The order as it stands before the cancel
    fn order(&self) -> &TokenOrder;

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

impl TokenCancelOrderTransitionActionAccessorsV0 for TokenCancelOrderTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn order(&self) -> &TokenOrder {
        &self.order
    }

    fn token_market_contract(&self) -> &Arc<DataContractFetchInfo> {
        &self.token_market_contract
    }
}
