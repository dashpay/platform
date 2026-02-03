mod v0;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;

use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
pub use v0::*;

impl DataContractCreateTransitionAccessorsV0 for DataContractCreateTransition {
    fn data_contract(&self) -> &DataContractInSerializationFormat {
        match self {
            DataContractCreateTransition::V0(transition) => &transition.data_contract,
            DataContractCreateTransition::V1(_) => {
                panic!("data_contract() accessor is not available for DataContractCreateTransitionV1 - use individual field accessors instead")
            }
        }
    }

    fn identity_nonce(&self) -> IdentityNonce {
        match self {
            DataContractCreateTransition::V0(transition) => transition.identity_nonce,
            DataContractCreateTransition::V1(transition) => transition.identity_nonce,
        }
    }

    fn set_data_contract(&mut self, data_contract: DataContractInSerializationFormat) {
        match self {
            DataContractCreateTransition::V0(transition) => {
                transition.data_contract = data_contract;
            }
            DataContractCreateTransition::V1(_) => {
                panic!("set_data_contract() is not available for DataContractCreateTransitionV1 - use individual field setters instead")
            }
        }
    }
}
