use crate::prelude::DataContract;
use crate::state_transition::state_transitions::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionActionV0,
};
use derive_more::From;

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

impl From<DataContractCreateTransition> for DataContractCreateTransitionAction {
    fn from(value: DataContractCreateTransition) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                DataContractCreateTransitionActionV0::from(v0).into()
            }
        }
    }
}

impl From<&DataContractCreateTransition> for DataContractCreateTransitionAction {
    fn from(value: &DataContractCreateTransition) -> Self {
        match value {
            DataContractCreateTransition::V0(v0) => {
                DataContractCreateTransitionActionV0::from(v0).into()
            }
        }
    }
}
