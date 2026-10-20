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
    "Identity {} is neither the owner nor an admin of contract group {}",
    identity_id,
    contract_group_id
)]
#[platform_serialize(unversioned)]
pub struct IdentityNotContractGroupOwnerOrAdminError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
    contract_group_id: Identifier,
}

impl IdentityNotContractGroupOwnerOrAdminError {
    pub fn new(identity_id: Identifier, contract_group_id: Identifier) -> Self {
        Self {
            identity_id,
            contract_group_id,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn contract_group_id(&self) -> &Identifier {
        &self.contract_group_id
    }
}

impl From<IdentityNotContractGroupOwnerOrAdminError> for ConsensusError {
    fn from(err: IdentityNotContractGroupOwnerOrAdminError) -> Self {
        Self::StateError(StateError::IdentityNotContractGroupOwnerOrAdminError(err))
    }
}
