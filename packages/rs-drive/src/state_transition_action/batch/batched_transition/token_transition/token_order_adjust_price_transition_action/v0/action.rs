use std::sync::Arc;
use dpp::balances::credits::TokenAmount;
use dpp::fee::Credits;
use dpp::identifier::Identifier;
use dpp::prelude::Revision;
use dpp::state_transition::batch_transition::batched_transition::Entropy;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token order cancel transition action v0
#[derive(Debug, Clone)]
pub struct TokenOrderAdjustPriceTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// Entropy generated to create order ID
    pub order_id: Identifier,
    /// Token amount to sell
    pub order_revision: Revision,
    ///
    pub token_price: Credits,
}

/// Accessors for `TokenOrderAdjustPriceTransitionActionV0`
pub trait TokenOrderAdjustPriceTransitionActionAccessorsV0 {
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

    /// Order ID to cancel
    fn order_id(&self) -> Identifier;

    /// Order Revision to cancel
    fn order_revision(&self) -> Revision;
}

impl TokenOrderAdjustPriceTransitionActionAccessorsV0 for TokenOrderAdjustPriceTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn order_id(&self) -> Identifier {
        self.order_id
    }

    fn order_revision(&self) -> Revision {
        self.order_revision
    }
}
