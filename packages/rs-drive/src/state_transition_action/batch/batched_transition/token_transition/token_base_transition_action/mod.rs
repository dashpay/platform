use derive_more::From;
use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::GroupAction;
use dpp::group::GroupStateTransitionResolvedInfo;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use std::sync::Arc;

/// transformer module
pub mod transformer;
mod v0;

use crate::drive::contract::DataContractFetchInfo;

use crate::error::Error;
pub use v0::*;

/// document base transition action
#[derive(Debug, Clone, From)]
pub enum TokenBaseTransitionAction {
    /// v0
    V0(TokenBaseTransitionActionV0),
}

impl TokenBaseTransitionActionAccessorsV0 for TokenBaseTransitionAction {
    fn token_position(&self) -> TokenContractPosition {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_contract_position,
        }
    }

    fn token_id(&self) -> Identifier {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_id,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.data_contract_id(),
        }
    }

    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.data_contract_fetch_info_ref(),
        }
    }

    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.data_contract_fetch_info(),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.identity_contract_nonce(),
        }
    }

    fn token_configuration(&self) -> Result<&TokenConfiguration, Error> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_configuration(),
        }
    }

    fn store_in_group(&self) -> Option<&GroupStateTransitionResolvedInfo> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.store_in_group(),
        }
    }

    fn original_group_action(&self) -> Option<&GroupAction> {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.original_group_action(),
        }
    }

    fn perform_action(&self) -> bool {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.perform_action(),
        }
    }
}
