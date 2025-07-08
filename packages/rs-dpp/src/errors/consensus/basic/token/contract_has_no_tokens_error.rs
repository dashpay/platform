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
#[error("Contract {} has no tokens", contract_id)]
#[platform_serialize(unversioned)]
pub struct ContractHasNoTokensError {
    contract_id: Identifier,
}

impl ContractHasNoTokensError {
    pub fn new(contract_id: Identifier) -> Self {
        Self { contract_id }
    }
    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }
}

impl From<ContractHasNoTokensError> for ConsensusError {
    fn from(err: ContractHasNoTokensError) -> Self {
        Self::BasicError(BasicError::ContractHasNoTokensError(err))
    }
}
