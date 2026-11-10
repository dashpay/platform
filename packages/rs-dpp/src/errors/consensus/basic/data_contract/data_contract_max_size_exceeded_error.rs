use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, DecodeUntrusted, Encode};

/// A contract whose bytes in the format state transitions carry would be over
/// `max_serialized_size`. A contract update that carries the whole contract cannot reach the
/// cap, as a state transition is capped well below it; one that carries only what it changes
/// (protocol version 15) grows a contract a transition at a time, up to the cap.
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
    "Data contract takes {serialized_size} bytes, over the maximum of {max_serialized_size} bytes a contract may take"
)]
#[platform_serialize(unversioned)]
pub struct DataContractMaxSizeExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    serialized_size: u64,
    max_serialized_size: u32,
}

impl DataContractMaxSizeExceededError {
    pub fn new(serialized_size: u64, max_serialized_size: u32) -> Self {
        Self {
            serialized_size,
            max_serialized_size,
        }
    }

    pub fn serialized_size(&self) -> u64 {
        self.serialized_size
    }

    pub fn max_serialized_size(&self) -> u32 {
        self.max_serialized_size
    }
}

impl From<DataContractMaxSizeExceededError> for ConsensusError {
    fn from(err: DataContractMaxSizeExceededError) -> Self {
        Self::BasicError(BasicError::DataContractMaxSizeExceededError(err))
    }
}
