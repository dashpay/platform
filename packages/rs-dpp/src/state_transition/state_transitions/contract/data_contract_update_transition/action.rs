use crate::prelude::DataContract;
use derive_more::From;
use crate::state_transition::data_contract_update_transition::{DataContractUpdateTransition, DataContractUpdateTransitionActionV0};

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

impl From<DataContractUpdateTransition> for DataContractUpdateTransitionAction {
    fn from(value: DataContractUpdateTransition) -> Self {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                DataContractUpdateTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&DataContractUpdateTransition> for DataContractUpdateTransitionAction {
    fn from(value: &DataContractUpdateTransition) -> Self {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                DataContractUpdateTransitionActionV0::from(v0).into()
            }
        }
    }
}
