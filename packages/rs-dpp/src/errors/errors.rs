use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
use crate::consensus::ConsensusError;
use crate::data_contract::errors::*;
use crate::data_contract::state_transition::errors::MissingDataContractIdError;
use crate::data_contract::state_transition::errors::PublicKeyIsDisabledError;
use crate::document::errors::*;
use crate::identity::{Purpose, SecurityLevel};
use crate::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, InvalidSignaturePublicKeyError, PublicKeyMismatchError,
    PublicKeySecurityLevelNotMetError, StateTransitionIsNotSignedError, WrongPublicKeyPurposeError,
};
use crate::{CompatibleProtocolVersionIsNotDefinedError, NonConsensusError, SerdeParsingError};

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Identifier Error: {0}")]
    IdentifierError(String),
    #[error("String Decode Error {0}")]
    StringDecodeError(String),
    #[error("Public key data is not set")]
    EmptyPublicKeyDataError,
    #[error("Payload reached a {max_size_kbytes}KB limit")]
    MaxEncodedBytesReachedError {
        payload: Vec<u8>,
        max_size_kbytes: usize,
    },
    #[error("Encoding Error - {0}")]
    EncodingError(String),
    #[error("Decoding Error - {0}")]
    DecodingError(String),
    #[error("File not found Error - {0}")]
    FileNotFound(String),
    #[error("Not included or invalid protocol version")]
    NoProtocolVersionError,
    #[error("Parsing error: {0}")]
    ParsingError(String),

    #[error(transparent)]
    ParsingJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    Error(#[from] anyhow::Error),

    #[error(transparent)]
    DataContractError(#[from] DataContractError),

    #[error(transparent)]
    StructureError(#[from] StructureError),

    #[error(transparent)]
    AbstractConsensusError(Box<ConsensusError>),

    #[error(transparent)]
    Document(Box<DocumentError>),

    #[error("Generic Error: {0}")]
    Generic(String),

    // State Transition Errors
    #[error(transparent)]
    InvalidIdentityPublicKeyTypeError(InvalidIdentityPublicKeyTypeError),
    #[error(transparent)]
    StateTransitionIsNotSignedError(StateTransitionIsNotSignedError),
    #[error(transparent)]
    PublicKeySecurityLevelNotMetError(PublicKeySecurityLevelNotMetError),
    #[error(transparent)]
    WrongPublicKeyPurposeError(WrongPublicKeyPurposeError),
    #[error(transparent)]
    PublicKeyMismatchError(PublicKeyMismatchError),
    #[error(transparent)]
    InvalidSignaturePublicKeyError(InvalidSignaturePublicKeyError),

    // TODO decide if it should be a string
    #[error("Non-Consensus error: {0}")]
    NonConsensusError(String),

    #[error(transparent)]
    CompatibleProtocolVersionIsNotDefinedError(#[from] CompatibleProtocolVersionIsNotDefinedError),

    // Data Contract
    #[error("Data Contract already exists")]
    DataContractAlreadyExistsError,

    #[error(transparent)]
    InvalidDataContractError(InvalidDataContractError),

    #[error(transparent)]
    DataContractNotPresentError(DataContractNotPresentError),

    #[error(transparent)]
    InvalidSignaturePublicKeySecurityLevelError(InvalidSignaturePublicKeySecurityLevelError),

    #[error("State Transition type is not present")]
    InvalidStateTransitionTypeError,

    #[error(transparent)]
    MissingDataContractIdError(MissingDataContractIdError),

    #[error(transparent)]
    PublicKeyIsDisabledError(PublicKeyIsDisabledError),

    #[error(transparent)]
    IdentityNotPresentError(IdentityNotPresentError),
    #[error("Identity is not present")]
    IdentityNotPresentError { id: Identifier },

    /// Error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// Error
    #[error("missing key: {0}")]
    DocumentKeyMissing(String),

    #[error("Invalid Identity: {errors:?}")]
    InvalidIdentityError {
        errors: Vec<ConsensusError>,
        raw_identity: JsonValue,
    },

    #[error("Public key generation error {0}")]
    PublicKeyGenerationError(String),
}

impl From<NonConsensusError> for ProtocolError {
    fn from(e: NonConsensusError) -> Self {
        Self::NonConsensusError(e.to_string())
    }
}

impl From<&str> for ProtocolError {
    fn from(v: &str) -> ProtocolError {
        ProtocolError::Generic(String::from(v))
    }
}

impl From<String> for ProtocolError {
    fn from(v: String) -> ProtocolError {
        Self::from(v.as_str())
    }
}

impl From<ConsensusError> for ProtocolError {
    fn from(e: ConsensusError) -> Self {
        ProtocolError::AbstractConsensusError(Box::new(e))
    }
}

impl From<DocumentError> for ProtocolError {
    fn from(e: DocumentError) -> Self {
        ProtocolError::Document(Box::new(e))
    }
}

impl From<SerdeParsingError> for ProtocolError {
    fn from(e: SerdeParsingError) -> Self {
        ProtocolError::ParsingError(e.to_string())
    }
}
