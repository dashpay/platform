use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Indexed property '{property_name}' for '{document_type}' document has an invalid constraint '{constraint_name}', reason: '{reason}'")]
#[platform_serialize(unversioned)]
pub struct InvalidIndexedPropertyConstraintError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_type: String,
    index_name: String,
    property_name: String,
    constraint_name: String,
    reason: String,
}

impl InvalidIndexedPropertyConstraintError {
    pub fn new(
        document_type: String,
        index_name: String,
        property_name: String,
        constraint_name: String,
        reason: String,
    ) -> Self {
        Self {
            document_type,
            index_name,
            property_name,
            constraint_name,
            reason,
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
    pub fn constraint_name(&self) -> &str {
        &self.constraint_name
    }
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<InvalidIndexedPropertyConstraintError> for ConsensusError {
    fn from(err: InvalidIndexedPropertyConstraintError) -> Self {
        Self::BasicError(BasicError::InvalidIndexedPropertyConstraintError(err))
    }
}
