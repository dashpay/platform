use derive_more::From;
use dpp::platform_value::Identifier;

use dpp::prelude::{FeeMultiplier, IdentityNonce};

/// transformer module
pub mod transformer;
mod v0;

pub use v0::*;

/// bump identity data contract nonce action
#[derive(Debug, Clone, From)]
pub enum BumpIdentityDataContractNonceAction {
    /// v0
    V0(BumpIdentityDataContractNonceActionV0),
}

impl BumpIdentityDataContractNonceActionAccessorsV0 for BumpIdentityDataContractNonceAction {
    fn identity_id(&self) -> Identifier {
        match self {
            BumpIdentityDataContractNonceAction::V0(v0) => v0.identity_id,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            BumpIdentityDataContractNonceAction::V0(v0) => v0.data_contract_id,
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            BumpIdentityDataContractNonceAction::V0(v0) => v0.identity_contract_nonce,
        }
    }

    fn fee_multiplier(&self) -> FeeMultiplier {
        match self {
            BumpIdentityDataContractNonceAction::V0(transition) => transition.fee_multiplier,
        }
    }
}
