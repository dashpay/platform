use derive_more::From;

use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::v0::{
    TokenTransferTransitionActionV0, TokenTransferTransitionActionAccessorsV0,
};
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use std::sync::Arc;
use crate::drive::contract::DataContractFetchInfo;

/// transformer module
pub mod transformer;
pub mod v0;

#[derive(Debug, Clone, From)]
pub enum TokenTransferTransitionAction {
    /// v0
    V0(TokenTransferTransitionActionV0),
}

/// Accessors trait for TokenTransferTransitionAction
pub trait TokenTransferTransitionActionAccessors {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Returns a reference to the base token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to transfer
    fn amount(&self) -> u64;

    /// Returns the recipient ID
    fn recipient_id(&self) -> Identifier;

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

    /// Returns the identity contract nonce
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }
}

impl TokenTransferTransitionActionAccessors for TokenTransferTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.base(),
        }
    }
    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.base_owned(),
        }
    }

    fn amount(&self) -> u64 {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.amount(),
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.recipient_id(),
        }
    }
}
