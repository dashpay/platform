use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Error as PlatformValueError;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("{value_error}")]
#[platform_serialize(unversioned)]
pub struct ValueError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    value_error: String,
}

impl ValueError {
    pub fn new(value_error: PlatformValueError) -> Self {
        Self {
            value_error: value_error.to_string(),
        }
    }

    pub fn value_error(&self) -> &str {
        &self.value_error
    }
}
impl From<ValueError> for ConsensusError {
    fn from(err: ValueError) -> Self {
        Self::BasicError(BasicError::ValueError(err))
    }
}

impl From<PlatformValueError> for ValueError {
    fn from(err: PlatformValueError) -> Self {
        ValueError::new(err)
    }
}
