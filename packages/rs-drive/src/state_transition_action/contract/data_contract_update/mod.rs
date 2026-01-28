/// transformer
pub mod transformer;
/// v0
pub mod v0;
/// v1
pub mod v1;

use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::v1::DataContractUpdateTransitionActionV1;
use derive_more::From;
use dpp::data_contract::DataContract;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// data contract update transition action
#[derive(Debug, Clone, From)]
#[allow(clippy::large_enum_variant)]
pub enum DataContractUpdateTransitionAction {
    /// v0
    V0(DataContractUpdateTransitionActionV0),
    /// v1
    V1(DataContractUpdateTransitionActionV1),
}

impl DataContractUpdateTransitionAction {
    /// data contract
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.data_contract,
            DataContractUpdateTransitionAction::V1(transition) => transition.data_contract,
        }
    }
    /// data contract ref
    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &transition.data_contract,
            DataContractUpdateTransitionAction::V1(transition) => &transition.data_contract,
        }
    }

    /// data contract mut
    pub fn data_contract_mut(&mut self) -> &mut DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &mut transition.data_contract,
            DataContractUpdateTransitionAction::V1(transition) => &mut transition.data_contract,
        }
    }

    /// identity contract nonce
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => {
                transition.identity_contract_nonce
            }
            DataContractUpdateTransitionAction::V1(transition) => {
                transition.identity_contract_nonce
            }
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.user_fee_increase,
            DataContractUpdateTransitionAction::V1(transition) => transition.user_fee_increase,
        }
    }

    /// old data contract ref (only available for V1)
    pub fn old_data_contract_ref(&self) -> Option<&DataContract> {
        match self {
            DataContractUpdateTransitionAction::V0(_) => None,
            DataContractUpdateTransitionAction::V1(transition) => {
                Some(&transition.old_data_contract)
            }
        }
    }
}
