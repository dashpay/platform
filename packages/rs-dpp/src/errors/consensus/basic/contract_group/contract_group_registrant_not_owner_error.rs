use crate::consensus::basic::BasicError;
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
    "Identity {} registers a contract group it is not the owner of",
    registrant_id
)]
#[platform_serialize(unversioned)]
pub struct ContractGroupRegistrantNotOwnerError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    registrant_id: Identifier,
}

impl ContractGroupRegistrantNotOwnerError {
    pub fn new(registrant_id: Identifier) -> Self {
        Self { registrant_id }
    }

    pub fn registrant_id(&self) -> &Identifier {
        &self.registrant_id
    }
}

impl From<ContractGroupRegistrantNotOwnerError> for ConsensusError {
    fn from(err: ContractGroupRegistrantNotOwnerError) -> Self {
        Self::BasicError(BasicError::ContractGroupRegistrantNotOwnerError(err))
    }
}
