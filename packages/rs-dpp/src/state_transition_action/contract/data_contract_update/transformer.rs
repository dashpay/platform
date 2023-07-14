use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;

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