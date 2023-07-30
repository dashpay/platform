mod v0;

use crate::data_contract::DataContract;
use crate::state_transition::data_contract_create_transition::methods::DataContractCreateTransitionMethodsV0;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use platform_value::Bytes32;
pub use v0::*;

impl DataContractCreateTransitionAccessorsV0 for DataContractCreateTransition {
    fn data_contract(&self) -> &DataContract {
        match self {
            DataContractCreateTransition::V0(transition) => &transition.data_contract,
        }
    }

    fn entropy(&self) -> &Bytes32 {
        match self {
            DataContractCreateTransition::V0(transition) => &transition.entropy,
        }
    }

    fn set_data_contract(&mut self, data_contract: DataContract) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.data_contract = data_contract;
            }
        }
    }
}
