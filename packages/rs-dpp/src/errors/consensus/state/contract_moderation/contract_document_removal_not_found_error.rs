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
    "Contract {} keeps no record of a moderator's deletion of {} document {}: there is nothing to restore",
    contract_id,
    document_type_name,
    document_id
)]
#[platform_serialize(unversioned)]
pub struct ContractDocumentRemovalNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    document_id: Identifier,
}

impl ContractDocumentRemovalNotFoundError {
    pub fn new(
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
    ) -> Self {
        Self {
            contract_id,
            document_type_name,
            document_id,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn document_id(&self) -> Identifier {
        self.document_id
    }
}

impl From<ContractDocumentRemovalNotFoundError> for ConsensusError {
    fn from(err: ContractDocumentRemovalNotFoundError) -> Self {
        Self::StateError(StateError::ContractDocumentRemovalNotFoundError(err))
    }
}
