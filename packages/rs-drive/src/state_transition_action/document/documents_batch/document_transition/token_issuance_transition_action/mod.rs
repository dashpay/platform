use derive_more::From;
use std::sync::Arc;

use crate::drive::contract::DataContractFetchInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;
use dpp::util::hash::hash_double;

/// transformer module for token issuance transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenIssuanceTransitionActionV0)

use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

/// Token issuance transition action
#[derive(Debug, Clone, From)]
pub enum TokenIssuanceTransitionAction {
    /// v0
    V0(TokenIssuanceTransitionActionV0),
}

/// Accessors trait for TokenIssuanceTransitionAction for version 0 fields
pub trait TokenIssuanceTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Returns the issuance amount
    fn issuance_amount(&self) -> u64;

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

    /// Returns the ID of the token issuance transition
    fn id(&self) -> Identifier {
        self.base().id()
    }
}

impl TokenIssuanceTransitionActionAccessorsV0 for TokenIssuanceTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenIssuanceTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn issuance_amount(&self) -> u64 {
        match self {
            TokenIssuanceTransitionAction::V0(v0) => v0.issuance_amount,
        }
    }
}
