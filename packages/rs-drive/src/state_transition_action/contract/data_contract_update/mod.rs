/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use derive_more::From;
use dpp::data_contract::DataContract;

/// data contract update transition action
#[derive(Debug, Clone, From)]
pub enum DataContractUpdateTransitionAction {
    /// v0
    V0(DataContractUpdateTransitionActionV0),
}

impl DataContractUpdateTransitionAction {
    /// data contract
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.data_contract,
        }
    }
    /// data contract ref
    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }
}
