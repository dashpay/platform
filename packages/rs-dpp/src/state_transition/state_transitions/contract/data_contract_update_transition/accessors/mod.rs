mod v0;

use crate::data_contract::serialized_version::DataContractInSerializationFormat;

use crate::prelude::IdentityNonce;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::ProtocolError;
use platform_value::Identifier;
pub use v0::*;

impl DataContractUpdateTransitionAccessorsV0 for DataContractUpdateTransition {
    fn data_contract(&self) -> Option<&DataContractInSerializationFormat> {
        match self {
            DataContractUpdateTransition::V0(transition) => Some(&transition.data_contract),
            DataContractUpdateTransition::V1(_) => None,
        }
    }

    fn set_data_contract(
        &mut self,
        data_contract: DataContractInSerializationFormat,
    ) -> Result<(), ProtocolError> {
        match self {
            DataContractUpdateTransition::V0(transition) => {
                transition.data_contract = data_contract;
                Ok(())
            }
            DataContractUpdateTransition::V1(_) => Err(ProtocolError::Generic(
                "a delta-based data contract update transition embeds no contract to replace"
                    .to_string(),
            )),
        }
    }

    fn identity_contract_nonce(&self) -> IdentityNonce {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.identity_contract_nonce,
            DataContractUpdateTransition::V1(transition) => transition.identity_contract_nonce,
        }
    }

    fn data_contract_id(&self) -> Identifier {
        match self {
            DataContractUpdateTransition::V0(transition) => transition.data_contract.id(),
            DataContractUpdateTransition::V1(transition) => transition.data_contract_id,
        }
    }
}
