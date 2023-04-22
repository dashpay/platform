use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::data_contract::{DataContract, DataContractInner};
use crate::state_transition::StateTransitionType;
use crate::util::hash::{hash, hash_to_vec};
use crate::ProtocolError;
use bincode::config;
use platform_value::Bytes32;
// use platform_serialization::PlatformSerialize;

#[derive(Debug, Clone, PartialEq)]
pub struct TempDataContractCreateTransitionWithoutWitness<'a> {
    pub protocol_version: u32,
    pub transition_type: StateTransitionType,
    pub data_contract: &'a DataContract,
    pub entropy: &'a Bytes32,
}

impl<'a> From<&'a DataContractCreateTransition>
    for TempDataContractCreateTransitionWithoutWitness<'a>
{
    fn from(value: &'a DataContractCreateTransition) -> Self {
        let DataContractCreateTransition {
            protocol_version,
            transition_type,
            data_contract,
            entropy,
            ..
        } = value;
        TempDataContractCreateTransitionWithoutWitness {
            protocol_version: *protocol_version,
            transition_type: *transition_type,
            data_contract,
            entropy,
        }
    }
}
