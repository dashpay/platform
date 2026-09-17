use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

/// A contested index declares a parameter the native contest machinery
/// cannot honour: an index property that is nested, a system property,
/// not required or transient, or a field match that names a property
/// outside the index, a property that is not a string, or a pattern that
/// tells apart the two strings the index stores under one key.
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Document type '{document_type}' contested index '{index_name}' declares unsupported parameters: {reason}"
)]
#[platform_serialize(unversioned)]
pub struct ContestedIndexInvalidParametersError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
    reason: String,
}

impl ContestedIndexInvalidParametersError {
    pub fn new(document_type: String, index_name: String, reason: String) -> Self {
        Self {
            document_type,
            index_name,
            reason,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }

    pub fn index_name(&self) -> &str {
        &self.index_name
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<ContestedIndexInvalidParametersError> for ConsensusError {
    fn from(err: ContestedIndexInvalidParametersError) -> Self {
        Self::BasicError(BasicError::ContestedIndexInvalidParametersError(err))
    }
}
