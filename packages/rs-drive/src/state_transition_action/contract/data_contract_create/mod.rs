/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use derive_more::From;
use dpp::data_contract::DataContract;
use dpp::prelude::{FeeMultiplier, IdentityNonce};

/// data contract create transition action
#[derive(Debug, Clone, From)]
pub enum DataContractCreateTransitionAction {
    /// v0
    V0(DataContractCreateTransitionActionV0),
}

impl DataContractCreateTransitionAction {
    /// data contract
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.data_contract,
        }
    }
    /// data contract ref
    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }

    /// identity nonce
    pub fn identity_nonce(&self) -> IdentityNonce {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.identity_nonce,
        }
    }

    /// fee multiplier
    pub fn fee_multiplier(&self) -> FeeMultiplier {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.fee_multiplier,
        }
    }
}
