mod transformer;

use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenDirectPurchaseTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// How many tokens should we buy.
    pub token_count: TokenAmount,
    /// Agreed price
    /// The user will pay this amount
    pub total_agreed_price: Credits,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenDirectPurchaseTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to purchase
    fn token_count(&self) -> TokenAmount;

    /// Sets the amount of tokens to purchase
    fn set_token_count(&mut self, amount: TokenAmount);

    /// The agreed price
    fn total_agreed_price(&self) -> Credits;

    /// Sets the agreed price
    fn set_total_agreed_price(&mut self, agreed_price: Credits);

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
}

impl TokenDirectPurchaseTransitionActionAccessorsV0 for TokenDirectPurchaseTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn token_count(&self) -> TokenAmount {
        self.token_count
    }

    fn set_token_count(&mut self, amount: TokenAmount) {
        self.token_count = amount;
    }

    fn total_agreed_price(&self) -> Credits {
        self.total_agreed_price
    }

    fn set_total_agreed_price(&mut self, agreed_price: Credits) {
        self.total_agreed_price = agreed_price;
    }
}
