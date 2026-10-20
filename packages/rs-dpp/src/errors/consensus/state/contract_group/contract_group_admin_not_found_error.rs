use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
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
    "Contract group {} names admin {}, which is not an existing non-masternode identity",
    contract_group_id,
    admin_id
)]
#[platform_serialize(unversioned)]
pub struct ContractGroupAdminNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_group_id: Identifier,
    admin_id: Identifier,
}

impl ContractGroupAdminNotFoundError {
    pub fn new(contract_group_id: Identifier, admin_id: Identifier) -> Self {
        Self {
            contract_group_id,
            admin_id,
        }
    }

    pub fn contract_group_id(&self) -> &Identifier {
        &self.contract_group_id
    }

    pub fn admin_id(&self) -> &Identifier {
        &self.admin_id
    }
}

impl From<ContractGroupAdminNotFoundError> for ConsensusError {
    fn from(err: ContractGroupAdminNotFoundError) -> Self {
        Self::StateError(StateError::ContractGroupAdminNotFoundError(err))
    }
}
