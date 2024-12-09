use derive_more::From;
use std::sync::Arc;

use crate::drive::contract::DataContractFetchInfo;
use dpp::identifier::Identifier;
use dpp::prelude::IdentityNonce;

/// transformer module for token burn transition action
pub mod transformer;
mod v0;

pub use v0::*; // re-export the v0 module items (including TokenBurnTransitionActionV0)

use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

/// Token burn transition action
#[derive(Debug, Clone, From)]
pub enum TokenBurnTransitionAction {
    /// v0
    V0(TokenBurnTransitionActionV0),
}

/// Accessors trait for TokenBurnTransitionAction for version 0 fields
pub trait TokenBurnTransitionActionAccessorsV0 {
    /// Returns a reference to the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Returns the burn amount
    fn burn_amount(&self) -> u64;

    /// Returns the token ID
    fn token_id(&self) -> u16 {
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

    /// Returns the ID of the token burn transition
    fn id(&self) -> Identifier {
        self.base().id()
    }
}

impl TokenBurnTransitionActionAccessorsV0 for TokenBurnTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenBurnTransitionAction::V0(v0) => &v0.base,
        }
    }

    fn burn_amount(&self) -> u64 {
        match self {
            TokenBurnTransitionAction::V0(v0) => v0.burn_amount,
        }
    }
}