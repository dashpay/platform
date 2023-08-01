pub mod transformer;
pub mod v0;

use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use derive_more::From;
use dpp::data_contract::DataContract;

#[derive(Debug, Clone, From)]
pub enum DataContractCreateTransitionAction {
    V0(DataContractCreateTransitionActionV0),
}

impl DataContractCreateTransitionAction {
    pub fn data_contract(self) -> DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => transition.data_contract,
        }
    }

    pub fn data_contract_ref(&self) -> &DataContract {
        match self {
            DataContractCreateTransitionAction::V0(transition) => &transition.data_contract,
        }
    }
}
