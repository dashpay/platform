mod v0;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;

use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
pub use v0::*;

impl DataContractUpdateTransitionAccessorsV0 for DataContractUpdateTransition {
    fn data_contract(&self) -> &DataContractInSerializationFormat {
        match self {
            DataContractUpdateTransition::V0(transition) => &transition.data_contract,
        }
    }

    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat) {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                transition.data_contract = data_contract
            }
        }
    }
}
