use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Choosing token mint recipient not allowed for token {}", token_id)]
#[platform_serialize(unversioned)]
pub struct ChoosingTokenMintRecipientNotAllowedError {
    token_id: Identifier,
}

impl ChoosingTokenMintRecipientNotAllowedError {
    pub fn new(token_id: Identifier) -> Self {
        Self { token_id }
    }
    pub fn token_id(&self) -> Identifier {
        self.token_id
    }
}

impl From<ChoosingTokenMintRecipientNotAllowedError> for ConsensusError {
    fn from(err: ChoosingTokenMintRecipientNotAllowedError) -> Self {
        Self::BasicError(BasicError::ChoosingTokenMintRecipientNotAllowedError(err))
    }
}
