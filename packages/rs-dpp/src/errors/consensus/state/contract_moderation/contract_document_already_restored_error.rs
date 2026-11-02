use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::TimestampMillis;
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
    "Document {} on contract {} was already restored by {} at {}: it is live",
    document_id,
    contract_id,
    restored_by,
    restored_at
)]
#[platform_serialize(unversioned)]
pub struct ContractDocumentAlreadyRestoredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_id: Identifier,
    restored_by: Identifier,
    restored_at: TimestampMillis,
}

impl ContractDocumentAlreadyRestoredError {
    pub fn new(
        contract_id: Identifier,
        document_id: Identifier,
        restored_by: Identifier,
        restored_at: TimestampMillis,
    ) -> Self {
        Self {
            contract_id,
            document_id,
            restored_by,
            restored_at,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }

    /// The contract owner or moderator that restored the document
    pub fn restored_by(&self) -> Identifier {
        self.restored_by
    }

    /// The time of the block that restored it, in milliseconds
    pub fn restored_at(&self) -> TimestampMillis {
        self.restored_at
    }
}

impl From<ContractDocumentAlreadyRestoredError> for ConsensusError {
    fn from(err: ContractDocumentAlreadyRestoredError) -> Self {
        Self::StateError(StateError::ContractDocumentAlreadyRestoredError(err))
    }
}
