use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};
use platform_value::Identifier;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error(
    "position field is not present for document type \"{}\"",
    document_type_name
)]
#[platform_serialize(unversioned)]
pub struct MissingPositionsInDocumentTypePropertiesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    missing_position: u32,
    contract_id: Identifier,
    document_type_name: String,
}

impl MissingPositionsInDocumentTypePropertiesError {
    pub fn new(missing_position: u32, contract_id: Identifier, document_type_name: String) -> Self {
        Self {
            missing_position,
            contract_id,
            document_type_name,
        }
    }
}

impl From<MissingPositionsInDocumentTypePropertiesError> for ConsensusError {
    fn from(err: MissingPositionsInDocumentTypePropertiesError) -> Self {
        Self::BasicError(BasicError::MissingPositionsInDocumentTypePropertiesError(
            err,
        ))
    }
}
