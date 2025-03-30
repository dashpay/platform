use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::state_transition::batch_transition::batched_transition::Entropy;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token order buy limit transition action v0
#[derive(Debug, Clone)]
pub struct TokenOrderBuyLimitTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Entropy generated to create order ID
    pub order_id_entropy: Entropy,
    /// Token amount to buy
    pub token_amount: TokenAmount,
    /// Max price to pay for the tokens
    pub token_max_price: Credits,
}

/// Accessors for `TokenOrderBuyLimitTransitionActionV0`
pub trait TokenOrderBuyLimitTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns the data contract ID
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns entropy generated to create order ID
    fn order_id_entropy(&self) -> Entropy;

    /// Returns the token amount to buy
    fn token_amount(&self) -> TokenAmount;

    /// Returns the max price to pay for the tokens
    fn token_max_price(&self) -> Credits;
}

impl TokenOrderBuyLimitTransitionActionAccessorsV0 for TokenOrderBuyLimitTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn order_id_entropy(&self) -> Entropy {
        self.order_id_entropy
    }

    fn token_amount(&self) -> TokenAmount {
        self.token_amount
    }

    fn token_max_price(&self) -> Credits {
        self.token_max_price
    }
}
