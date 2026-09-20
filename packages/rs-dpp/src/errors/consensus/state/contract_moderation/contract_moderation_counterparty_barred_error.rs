use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use std::fmt;
use thiserror::Error;

/// The part a barred identity was asked to play in a document transition it did not sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum ContractModerationCounterpartyRole {
    /// The identity a document was to be transferred to.
    Recipient,
    /// The identity a document was to be bought from.
    Seller,
}

impl fmt::Display for ContractModerationCounterpartyRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractModerationCounterpartyRole::Recipient => write!(f, "recipient"),
            ContractModerationCounterpartyRole::Seller => write!(f, "seller"),
        }
    }
}

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
    "Identity {} is banned or suspended on contract {} and can not be the {} of a document",
    identity_id,
    contract_id,
    role
)]
#[platform_serialize(unversioned)]
pub struct ContractModerationCounterpartyBarredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    identity_id: Identifier,
    role: ContractModerationCounterpartyRole,
}

impl ContractModerationCounterpartyBarredError {
    pub fn new(
        contract_id: Identifier,
        identity_id: Identifier,
        role: ContractModerationCounterpartyRole,
    ) -> Self {
        Self {
            contract_id,
            identity_id,
            role,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    pub fn role(&self) -> ContractModerationCounterpartyRole {
        self.role
    }
}

impl From<ContractModerationCounterpartyBarredError> for ConsensusError {
    fn from(err: ContractModerationCounterpartyBarredError) -> Self {
        Self::StateError(StateError::ContractModerationCounterpartyBarredError(err))
    }
}
