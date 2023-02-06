use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::data_contract::errors::*;
use crate::document::errors::*;
use crate::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use crate::prelude::Identifier;
use crate::state_transition::StateTransition;
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
    #[error("Invalid signature type")]
    InvalidIdentityPublicKeyTypeError { public_key_type: KeyType },
    #[error("State Transition is not signed")]
    StateTransitionIsNotIsSignedError { state_transition: StateTransition },
    #[error(
        "Invalid key security level: {public_key_security_level}. The state transition requires at least: {required_security_level}"
    )]
    PublicKeySecurityLevelNotMetError {
        public_key_security_level: SecurityLevel,
        required_security_level: SecurityLevel,
    },
    #[error("Invalid identity key purpose {public_key_purpose}. This state transition requires {key_purpose_requirement}")]
    WrongPublicKeyPurposeError {
        public_key_purpose: Purpose,
        key_purpose_requirement: Purpose,
    },
    #[error("Public key generation error {0}")]
    PublicKeyGenerationError(String),

    #[error("Public key mismatched")]
    PublicKeyMismatchError { public_key: IdentityPublicKey },

    #[error("Invalid signature public key")]
    InvalidSignaturePublicKeyError { public_key: Vec<u8> },

    // TODO decide if it should be a string
    #[error("Non-Consensus error: {0}")]
    NonConsensusError(String),

    #[error(transparent)]
    CompatibleProtocolVersionIsNotDefinedError(#[from] CompatibleProtocolVersionIsNotDefinedError),

    // Data Contract
    #[error("Data Contract already exists")]
    DataContractAlreadyExistsError,

    #[error("Invalid Data Contract: {errors:?}")]
    InvalidDataContractError {
        errors: Vec<ConsensusError>,
        raw_data_contract: JsonValue,
    },

    #[error("Data Contract is not present")]
    DataContractNotPresentError { data_contract_id: Identifier },

    #[error("Invalid public key security level {public_key_security_level}. This state transition requires {required_security_level}")]
    InvalidSignaturePublicKeySecurityLevelError {
        public_key_security_level: SecurityLevel,
        required_security_level: SecurityLevel,
    },

    #[error("State Transition type is not present")]
    InvalidStateTransitionTypeError,

    #[error("$dataContractId is not present")]
    MissingDataContractIdError { raw_document_transition: JsonValue },

    #[error("Public key is disabled")]
    PublicKeyIsDisabledError { public_key: IdentityPublicKey },

    #[error("Identity is not present")]
    IdentityNotPresentError { id: Identifier },
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
