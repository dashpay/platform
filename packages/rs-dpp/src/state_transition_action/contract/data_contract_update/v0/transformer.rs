use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;

impl From<DataContractUpdateTransitionV0> for DataContractUpdateTransitionActionV0 {
    fn from(value: DataContractUpdateTransitionV0) -> Self {
        DataContractUpdateTransitionActionV0 {
            data_contract: value.data_contract,
        }
    }
}

impl From<&DataContractUpdateTransitionV0> for DataContractUpdateTransitionActionV0 {
    fn from(value: &DataContractUpdateTransitionV0) -> Self {
        DataContractUpdateTransitionActionV0 {
            data_contract: value.data_contract.clone(),
        }
    }
}
