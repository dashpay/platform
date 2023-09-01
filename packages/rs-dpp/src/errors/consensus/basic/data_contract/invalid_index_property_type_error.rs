use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("'{property_name}' property of '{document_type}' document has an invalid type '{property_type}' and cannot be use as an index")]
#[platform_serialize(unversioned)]
pub struct InvalidIndexPropertyTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
    property_name: String,
    property_type: String,
}

impl InvalidIndexPropertyTypeError {
    pub fn new(
        document_type: String,
        index_name: String,
        property_name: String,
        property_type: String,
    ) -> Self {
        Self {
            document_type,
            index_name,
            property_name,
            property_type,
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
    pub fn property_type(&self) -> &str {
        &self.property_type
    }
}

impl From<InvalidIndexPropertyTypeError> for ConsensusError {
    fn from(err: InvalidIndexPropertyTypeError) -> Self {
        Self::BasicError(BasicError::InvalidIndexPropertyTypeError(err))
    }
}
