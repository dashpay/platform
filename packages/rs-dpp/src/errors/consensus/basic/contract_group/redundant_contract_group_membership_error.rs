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
#[error("Contract group membership of {} in contract group {} is redundant: the whole contract is already a member", member, contract_group_id)]
#[platform_serialize(unversioned)]
pub struct RedundantContractGroupMembershipError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_group_id: Identifier,
    member: ContractGroupMember,
}

impl RedundantContractGroupMembershipError {
    pub fn new(contract_group_id: Identifier, member: ContractGroupMember) -> Self {
        Self {
            contract_group_id,
            member,
        }
    }

    pub fn contract_group_id(&self) -> &Identifier {
        &self.contract_group_id
    }

    pub fn member(&self) -> &ContractGroupMember {
        &self.member
    }
}

impl From<RedundantContractGroupMembershipError> for ConsensusError {
    fn from(err: RedundantContractGroupMembershipError) -> Self {
        Self::BasicError(BasicError::RedundantContractGroupMembershipError(err))
    }
}
