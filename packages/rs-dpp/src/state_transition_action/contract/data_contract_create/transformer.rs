use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;

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
