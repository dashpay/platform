mod transformer;

use std::sync::Arc;
use dpp::identifier::Identifier;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenSetPriceForDirectPurchaseTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// What should be the price for a single token
    /// Setting this to None makes it no longer purchasable
    pub price: Option<TokenPricingSchedule>,
    /// The public note
    pub public_note: Option<String>,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the price
    fn price(&self) -> Option<&TokenPricingSchedule>;

    /// Sets the amount of tokens to issuance
    fn set_price(&mut self, price: Option<TokenPricingSchedule>);

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

    /// Returns the public note (optional)
    fn public_note(&self) -> Option<&String>;

    /// Returns the public note (owned)
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);
}

impl TokenSetPriceForDirectPurchaseTransitionActionAccessorsV0
    for TokenSetPriceForDirectPurchaseTransitionActionV0
{
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn price(&self) -> Option<&TokenPricingSchedule> {
        self.price.as_ref()
    }

    fn set_price(&mut self, price: Option<TokenPricingSchedule>) {
        self.price = price;
    }

    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }
}
