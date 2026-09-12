use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid authentication scope: {reason}")]
#[platform_serialize(unversioned)]
pub struct InvalidAuthenticationScopeError {
    reason: String,
}
impl InvalidAuthenticationScopeError {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
    pub fn reason(&self) -> &String {
        &self.reason
    }
}
impl From<InvalidAuthenticationScopeError> for ConsensusError {
    fn from(error: InvalidAuthenticationScopeError) -> Self {
        Self::BasicError(BasicError::InvalidAuthenticationScopeError(error))
    }
}
