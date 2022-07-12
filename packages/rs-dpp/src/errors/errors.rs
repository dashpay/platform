use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::data_contract::{errors::*, DataContract};
use crate::document::{errors::*, Document};
use crate::identity::{IdentityPublicKey, Purpose, SecurityLevel};
use crate::state_transition::StateTransition;
use crate::{CompatibleProtocolVersionIsNotDefinedError, NonConsensusError};

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Identifier Error: {0}")]
    IdentifierError(String),
    #[error("String Decode Error {0}")]
    StringDecodeError(String),
    #[error("Public key data is not set")]
    EmptyPublicKeyDataError,
    #[error("Payload reached a {0}Kb limit")]
    MaxEncodedBytesReachedError(usize),
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
    DataContractError(DataContractError),

    #[error(transparent)]
    AbstractConsensusError(Box<ConsensusError>),

    #[error(transparent)]
    Document(Box<DocumentError>),

    #[error("Generic Error: {0}")]
    Generic(String),

    // State Transition Errors
    #[error("Invalid signature type")]
    InvalidIdentityPublicKeyTypeError { public_key_type: u32 },
    #[error("State Transition is not signed")]
    StateTransitionIsNotIsSignedError { state_transition: StateTransition },
    #[error(
        "State transition is signed with a key with security level '{public_key_security_level}', but expected at leas '{required_security_level}'"
    )]
    PublicKeySecurityLevelNotMetError {
        public_key_security_level: SecurityLevel,
        required_security_level: SecurityLevel,
    },
    #[error("State transition must be signed with a key that has purpose '{key_purpose_requirement}' but got '{public_key_purpose}'")]
    WrongPublicKeyPurposeError {
        public_key_purpose: Purpose,
        key_purpose_requirement: Purpose,
    },
    #[error("Public key mismatched")]
    PublicKeyMismatchError { public_key: IdentityPublicKey },

    #[error("Invalid signature public key")]
    InvalidSignaturePublicKeyError { public_key: Vec<u8> },

    // Documents
    //? This error is duplicated by `[crate::errors::consensus::abstract_basic_error::BasicError]`
    #[error("Data Contract doesn't define document wit type '{document_type}'")]
    InvalidDocumentTypeError {
        document_type: String,
        data_contract: DataContract,
    },

    #[error("Invalid Document: {errors:?}")]
    InvalidDocumentError {
        errors: Vec<ConsensusError>,
        raw_document: JsonValue,
    },

    #[error("No documents were supplied to state transition")]
    NoDocumentsSuppliedError,

    #[error("Documents have mixed owner ids")]
    MismatchOwnerIdsError { documents: Vec<Document> },

    #[error("Invalid Document initial revision {}", document.revision)]
    InvalidInitialRevisionError { document: Document },

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

impl From<DataContractError> for ProtocolError {
    fn from(e: DataContractError) -> Self {
        ProtocolError::DataContractError(e)
    }
}
