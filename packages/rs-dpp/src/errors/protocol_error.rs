use thiserror::Error;

use crate::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use crate::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
use crate::consensus::ConsensusError;
use crate::data_contract::errors::*;
use crate::data_contract::state_transition::errors::MissingDataContractIdError;
use crate::data_contract::state_transition::errors::PublicKeyIsDisabledError;
use crate::document::errors::*;
use crate::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, InvalidSignaturePublicKeyError, PublicKeyMismatchError,
    PublicKeySecurityLevelNotMetError, StateTransitionError, StateTransitionIsNotSignedError,
    WrongPublicKeyPurposeError,
};
use crate::{
    CompatibleProtocolVersionIsNotDefinedError, DashPlatformProtocolInitError, NonConsensusError,
    SerdeParsingError,
};

use dashcore::consensus::encode::Error as DashCoreError;

use platform_value::{Error as ValueError, Value};

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
    #[error("unknown protocol version error {0}")]
    UnknownProtocolVersionError(String),
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
    StateTransitionError(#[from] StateTransitionError),

    #[error(transparent)]
    StructureError(#[from] StructureError),

    #[error(transparent)]
    ConsensusError(Box<ConsensusError>),

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

    #[error(transparent)]
    NonConsensusError(#[from] NonConsensusError),

    #[error(transparent)]
    CompatibleProtocolVersionIsNotDefinedError(#[from] CompatibleProtocolVersionIsNotDefinedError),

    // Data Contract
    #[error("Data Contract already exists")]
    DataContractAlreadyExistsError,

    #[error(transparent)]
    InvalidDataContractError(InvalidDataContractError),

    #[error(transparent)]
    InvalidDocumentTypeError(InvalidDocumentTypeError),

    #[error(transparent)]
    DataContractNotPresentError(DataContractNotPresentError),

    #[error(transparent)]
    InvalidSignaturePublicKeySecurityLevelError(InvalidSignaturePublicKeySecurityLevelError),

    #[error(transparent)]
    InvalidStateTransitionTypeError(InvalidStateTransitionTypeError),

    #[error(transparent)]
    MissingDataContractIdError(MissingDataContractIdError),

    #[error(transparent)]
    PublicKeyIsDisabledError(PublicKeyIsDisabledError),

    #[error(transparent)]
    IdentityNotPresentError(IdentityNotPresentError),

    /// Error
    #[error("overflow error: {0}")]
    Overflow(&'static str),

    /// Error
    #[error("missing key: {0}")]
    DocumentKeyMissing(String),

    /// Value error
    #[error("value error: {0}")]
    ValueError(#[from] ValueError),

    /// Dash core error
    #[error("dash core error: {0}")]
    DashCoreError(#[from] DashCoreError),

    #[error("Invalid Identity: {errors:?}")]
    InvalidIdentityError {
        errors: Vec<ConsensusError>,
        raw_identity: Value,
    },

    #[error("Public key generation error {0}")]
    PublicKeyGenerationError(String),

    #[error("corrupted code execution: {0}")]
    CorruptedCodeExecution(String),
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
        ProtocolError::ConsensusError(Box::new(e))
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

impl From<DashPlatformProtocolInitError> for ProtocolError {
    fn from(e: DashPlatformProtocolInitError) -> Self {
        ProtocolError::Generic(e.to_string())
    }
}
