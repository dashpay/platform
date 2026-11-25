use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, DecodeUntrusted, Encode};

/// A contract whose compiled form, the bytes a node stores it as from protocol version 15,
/// would be over `max_compiled_contract_size`: compiling inlines every `$ref` into each
/// property that uses it, so a contract within the size transitions carry can compile much
/// larger.
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
    "Data contract compiles to {compiled_size} bytes, over the maximum of {max_compiled_size} bytes a contract stored compiled may take"
)]
#[platform_serialize(unversioned)]
pub struct DataContractCompiledMaxSizeExceededError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    compiled_size: u64,
    max_compiled_size: u32,
}

impl DataContractCompiledMaxSizeExceededError {
    pub fn new(compiled_size: u64, max_compiled_size: u32) -> Self {
        Self {
            compiled_size,
            max_compiled_size,
        }
    }

    pub fn compiled_size(&self) -> u64 {
        self.compiled_size
    }

    pub fn max_compiled_size(&self) -> u32 {
        self.max_compiled_size
    }
}

impl From<DataContractCompiledMaxSizeExceededError> for ConsensusError {
    fn from(err: DataContractCompiledMaxSizeExceededError) -> Self {
        Self::BasicError(BasicError::DataContractCompiledMaxSizeExceededError(err))
    }
}
