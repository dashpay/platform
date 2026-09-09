use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document type {document_type} required fields update is not allowed: {details}")]
#[platform_serialize(unversioned)]
pub struct DataContractInvalidRequiredFieldsUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    details: String,
}

impl DataContractInvalidRequiredFieldsUpdateError {
    pub fn new(document_type: String, details: String) -> Self {
        Self {
            document_type,
            details,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn details(&self) -> &str {
        &self.details
    }
}

impl From<DataContractInvalidRequiredFieldsUpdateError> for ConsensusError {
    fn from(err: DataContractInvalidRequiredFieldsUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractInvalidRequiredFieldsUpdateError(
            err,
        ))
    }
}
