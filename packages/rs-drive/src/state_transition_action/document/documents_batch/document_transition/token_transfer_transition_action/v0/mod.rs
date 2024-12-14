mod transformer;

use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

/// Token transfer transition action v0
#[derive(Debug, Clone)]
pub struct TokenTransferTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount to transfer
    pub amount: u64,
    /// The recipient owner ID
    pub recipient_id: Identifier,
}

/// Accessors for `TokenTransferTransitionActionV0`
pub trait TokenTransferTransitionActionAccessorsV0 {
    /// Returns the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Returns the base owned token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to transfer
    fn amount(&self) -> u64;

    /// Returns the recipient owner ID
    fn recipient_id(&self) -> Identifier;

    /// Returns the token ID from the base action
    fn token_id(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the data contract ID from the base action
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info from the base action
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns the identity contract nonce from the base action
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }

    /// Returns the transition ID from the base action
    fn id(&self) -> Identifier {
        self.base().id()
    }
}

impl TokenTransferTransitionActionAccessorsV0 for TokenTransferTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn amount(&self) -> u64 {
        self.amount
    }

    fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }
}
