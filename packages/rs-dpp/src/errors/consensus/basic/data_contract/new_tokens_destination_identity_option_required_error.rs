use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("token configuration requires a new tokens destination identity if choosing destination is not allowed for contract {contract_id} at position {token_position}")]
#[platform_serialize(unversioned)]
pub struct NewTokensDestinationIdentityOptionRequiredError {
    contract_id: Identifier,
    token_position: TokenContractPosition,
}

impl NewTokensDestinationIdentityOptionRequiredError {
    pub fn new(contract_id: Identifier, token_position: TokenContractPosition) -> Self {
        Self {
            contract_id,
            token_position,
        }
    }

    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn token_position(&self) -> TokenContractPosition {
        self.token_position
    }
}

impl From<NewTokensDestinationIdentityOptionRequiredError> for ConsensusError {
    fn from(err: NewTokensDestinationIdentityOptionRequiredError) -> Self {
        Self::BasicError(BasicError::NewTokensDestinationIdentityOptionRequiredError(
            err,
        ))
    }
}
