use crate::data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::data_contract::DataContract;
use crate::serialization_traits::PlatformSerializable;
use crate::state_transition::StateTransitionType;

use crate::ProtocolError;
use bincode::config;
use bincode::Encode;
use platform_serialization::PlatformSerialize;

#[derive(Debug, Clone, PartialEq, Encode, PlatformSerialize)]
#[platform_error_type(ProtocolError)]
pub struct TempDataContractUpdateTransitionWithoutWitness<'a> {
    pub protocol_version: u32,
    pub transition_type: StateTransitionType,
    pub data_contract: &'a DataContract,
}

impl<'a> From<&'a DataContractUpdateTransition>
    for TempDataContractUpdateTransitionWithoutWitness<'a>
{
    fn from(value: &'a DataContractUpdateTransition) -> Self {
        let DataContractUpdateTransition {
            protocol_version,
            transition_type,
            data_contract,
            ..
        } = value;
        TempDataContractUpdateTransitionWithoutWitness {
            protocol_version: *protocol_version,
            transition_type: *transition_type,
            data_contract,
        }
    }
}
