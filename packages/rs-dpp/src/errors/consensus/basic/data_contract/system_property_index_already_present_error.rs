use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("System property '{property_name}' is already indexed and can't be used in '{index_name}' index of '{document_type}' document")]
#[platform_serialize(unversioned)]
pub struct SystemPropertyIndexAlreadyPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
    property_name: String,
}

impl SystemPropertyIndexAlreadyPresentError {
    pub fn new(document_type: String, index_name: String, property_name: String) -> Self {
        Self {
            document_type,
            index_name,
            property_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn index_name(&self) -> &str {
        &self.index_name
    }
    pub fn property_name(&self) -> &str {
        &self.property_name
    }
}

impl From<SystemPropertyIndexAlreadyPresentError> for ConsensusError {
    fn from(err: SystemPropertyIndexAlreadyPresentError) -> Self {
        Self::BasicError(BasicError::SystemPropertyIndexAlreadyPresentError(err))
    }
}
