mod transformer;

use std::sync::Arc;
use dpp::identifier::Identifier;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};

/// Token issuance transition action v0
#[derive(Debug, Clone)]
pub struct TokenIssuanceTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount of tokens to create
    pub issuance_amount: u64,
    /// The identity to credit the token to
    pub identity_balance_holder_id: Identifier,
}

/// Accessors for `TokenIssuanceTransitionActionV0`
pub trait TokenIssuanceTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Consumes self and returns the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to issuance
    fn issuance_amount(&self) -> u64;

    /// Sets the amount of tokens to issuance
    fn set_issuance_amount(&mut self, amount: u64);

    /// Consumes self and returns the identity balance holder ID
    fn identity_balance_holder_id(&self) -> Identifier;

    /// Sets the identity balance holder ID
    fn set_identity_balance_holder_id(&mut self, id: Identifier);

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

impl TokenIssuanceTransitionActionAccessorsV0 for TokenIssuanceTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn issuance_amount(&self) -> u64 {
        self.issuance_amount
    }

    fn set_issuance_amount(&mut self, amount: u64) {
        self.issuance_amount = amount;
    }

    fn identity_balance_holder_id(&self) -> Identifier {
        self.identity_balance_holder_id
    }

    fn set_identity_balance_holder_id(&mut self, id: Identifier) {
        self.identity_balance_holder_id = id;
    }
}
