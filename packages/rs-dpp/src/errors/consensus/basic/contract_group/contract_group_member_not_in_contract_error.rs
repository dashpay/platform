use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::contract_group::ContractGroupMember;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Contract group member {} does not exist in data contract {}",
    member,
    data_contract_id
)]
#[platform_serialize(unversioned)]
pub struct ContractGroupMemberNotInContractError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    member: ContractGroupMember,
}

impl ContractGroupMemberNotInContractError {
    pub fn new(data_contract_id: Identifier, member: ContractGroupMember) -> Self {
        Self {
            data_contract_id,
            member,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn member(&self) -> &ContractGroupMember {
        &self.member
    }
}

impl From<ContractGroupMemberNotInContractError> for ConsensusError {
    fn from(err: ContractGroupMemberNotInContractError) -> Self {
        Self::BasicError(BasicError::ContractGroupMemberNotInContractError(err))
    }
}
