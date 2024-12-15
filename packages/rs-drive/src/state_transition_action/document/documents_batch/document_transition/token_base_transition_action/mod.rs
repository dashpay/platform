use derive_more::From;
use dpp::platform_value::Identifier;
use dpp::prelude::IdentityNonce;
use std::sync::Arc;

/// transformer module
pub mod transformer;
mod v0;

use crate::drive::contract::DataContractFetchInfo;

pub use v0::*;

/// document base transition action
#[derive(Debug, Clone, From)]
pub enum TokenBaseTransitionAction {
    /// v0
    V0(TokenBaseTransitionActionV0),
}

impl TokenBaseTransitionActionAccessorsV0 for TokenBaseTransitionAction {
    fn token_position(&self) -> u16 {
        match self {
            TokenBaseTransitionAction::V0(v0) => v0.token_position,
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
}
