use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("'{property_name}' property is not defined in the '{document_type}' document")]
#[platform_serialize(unversioned)]
pub struct UndefinedIndexPropertyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
    property_name: String,
}

impl UndefinedIndexPropertyError {
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

impl From<UndefinedIndexPropertyError> for ConsensusError {
    fn from(err: UndefinedIndexPropertyError) -> Self {
        Self::BasicError(BasicError::UndefinedIndexPropertyError(err))
    }
}
