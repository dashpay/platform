pub mod transformer;
pub mod v0;

use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use derive_more::From;
use dpp::data_contract::DataContract;

#[derive(Debug, Clone, From)]
pub enum DataContractUpdateTransitionAction {
    V0(DataContractUpdateTransitionActionV0),
}

impl DataContractUpdateTransitionAction {
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => transition.data_contract,
        }
    }

    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractUpdateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }
}
