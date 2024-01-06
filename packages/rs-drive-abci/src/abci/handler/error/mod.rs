pub mod consensus;

use crate::error::query::QueryError;
use crate::error::Error;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::platform_value::platform_value;
use dpp::platform_value::string_encoding::{encode, Encoding};
use tenderdash_abci::proto::abci::ResponseException;

/// ABCI handlers errors
#[derive(Debug, thiserror::Error)]
// Allow dead code, as the majority of these errors are reserved for future use
#[allow(dead_code)]
pub enum HandlerError {
    /// ABCI Handler error (Cancelled)
    #[error("{0}")]
    Cancelled(String),
    /// ABCI Handler error (Unknown)
    #[error("{0}")]
    Unknown(String),
    /// ABCI Handler error (InvalidArgument)
    #[error("{0}")]
    InvalidArgument(String),
    /// ABCI Handler error (DeadlineExceeded)
    #[error("{0}")]
    DeadlineExceeded(String),
    /// ABCI Handler error (NotFound)
    #[error("{0}")]
    NotFound(String),
    /// ABCI Handler error (AlreadyExists)
    #[error("{0}")]
    AlreadyExists(String),
    /// ABCI Handler error (PermissionDenied)
    #[error("{0}")]
    PermissionDenied(String),
    /// ABCI Handler error (ResourceExhausted)
    #[error("{0}")]
    ResourceExhausted(String),
    /// ABCI Handler error (FailedPrecondition)
    #[error("{0}")]
    FailedPrecondition(String),
    /// ABCI Handler error (Aborted)
    #[error("{0}")]
    Aborted(String),
    /// ABCI Handler error (OutOfRange)
    #[error("{0}")]
    OutOfRange(String),
    /// ABCI Handler error (Unimplemented)
    #[error("{0}")]
    Unimplemented(String),
    /// ABCI Handler error (Internal)
    #[error("{0}")]
    Internal(String),
    /// ABCI Handler error (Unavailable)
    #[error("{0}")]
    Unavailable(String),
    /// ABCI Handler error (DataLoss)
    #[error("{0}")]
    DataLoss(String),
    /// ABCI Handler error (Unauthenticated)
    #[error("{0}")]
    Unauthenticated(String),
    /// State Transition processing consensus error
    #[error(transparent)]
    StateTransitionConsensusError(ConsensusError),
}

/// Error codes for ABCI handlers
#[repr(u32)]
pub enum HandlerErrorCode {
    /// ABCI Handler error (Cancelled)
    Cancelled = 1,
    /// ABCI Handler error (Unknown)
    Unknown = 2,
    /// ABCI Handler error (InvalidArgument)
    InvalidArgument = 3,
    /// ABCI Handler error (DeadlineExceeded)
    DeadlineExceeded = 4,
    /// ABCI Handler error (NotFound)
    NotFound = 5,
    /// ABCI Handler error (AlreadyExists)
    AlreadyExists = 6,
    /// ABCI Handler error (PermissionDenied)
    PermissionDenied = 7,
    /// ABCI Handler error (ResourceExhausted)
    ResourceExhausted = 8,
    /// ABCI Handler error (FailedPrecondition)
    FailedPrecondition = 9,
    /// ABCI Handler error (Aborted)
    Aborted = 10,
    /// ABCI Handler error (OutOfRange)
    OutOfRange = 11,
    /// ABCI Handler error (Unimplemented)
    Unimplemented = 12,
    /// ABCI Handler error (Internal)
    Internal = 13,
    /// ABCI Handler error (Unavailable)
    Unavailable = 14,
    /// ABCI Handler error (DataLoss)
    DataLoss = 15,
    /// ABCI Handler error (Unauthenticated)
    Unauthenticated = 16,
}

impl HandlerError {
    /// Returns ABCI handler error code
    pub fn code(&self) -> u32 {
        match self {
            HandlerError::Cancelled(_) => HandlerErrorCode::Cancelled as u32,
            HandlerError::Unknown(_) => HandlerErrorCode::Unknown as u32,
            HandlerError::InvalidArgument(_) => HandlerErrorCode::InvalidArgument as u32,
            HandlerError::DeadlineExceeded(_) => HandlerErrorCode::DeadlineExceeded as u32,
            HandlerError::NotFound(_) => HandlerErrorCode::NotFound as u32,
            HandlerError::AlreadyExists(_) => HandlerErrorCode::AlreadyExists as u32,
            HandlerError::PermissionDenied(_) => HandlerErrorCode::PermissionDenied as u32,
            HandlerError::ResourceExhausted(_) => HandlerErrorCode::ResourceExhausted as u32,
            HandlerError::FailedPrecondition(_) => HandlerErrorCode::FailedPrecondition as u32,
            HandlerError::Aborted(_) => HandlerErrorCode::Aborted as u32,
            HandlerError::OutOfRange(_) => HandlerErrorCode::OutOfRange as u32,
            HandlerError::Unimplemented(_) => HandlerErrorCode::Unimplemented as u32,
            HandlerError::Internal(_) => HandlerErrorCode::Internal as u32,
            HandlerError::Unavailable(_) => HandlerErrorCode::Unavailable as u32,
            HandlerError::DataLoss(_) => HandlerErrorCode::DataLoss as u32,
            HandlerError::Unauthenticated(_) => HandlerErrorCode::Unauthenticated as u32,
            HandlerError::StateTransitionConsensusError(error) => error.code(),
        }
    }

    /// Returns error message
    pub fn message(&self) -> String {
        match self {
            HandlerError::Cancelled(message) => message.to_owned(),
            HandlerError::Unknown(message) => message.to_owned(),
            HandlerError::InvalidArgument(message) => message.to_owned(),
            HandlerError::DeadlineExceeded(message) => message.to_owned(),
            HandlerError::NotFound(message) => message.to_owned(),
            HandlerError::AlreadyExists(message) => message.to_owned(),
            HandlerError::PermissionDenied(message) => message.to_owned(),
            HandlerError::ResourceExhausted(message) => message.to_owned(),
            HandlerError::FailedPrecondition(message) => message.to_owned(),
            HandlerError::Aborted(message) => message.to_owned(),
            HandlerError::OutOfRange(message) => message.to_owned(),
            HandlerError::Unimplemented(message) => message.to_owned(),
            HandlerError::Internal(message) => message.to_owned(),
            HandlerError::Unavailable(message) => message.to_owned(),
            HandlerError::DataLoss(message) => message.to_owned(),
            HandlerError::Unauthenticated(message) => message.to_owned(),
            HandlerError::StateTransitionConsensusError(error) => error.to_string(),
        }
    }

    /// Returns base64-encoded message for info field of ABCI handler responses
    pub fn response_info(&self) -> Result<String, ResponseException> {
        let error_data_buffer = platform_value!({
            "message": self.message(),
            // TODO: consider capturing stack with one of the libs
            //   and send it to the client
            //"stack": "..."
        })
        .to_cbor_buffer()
        .map_err(|e| ResponseException::from(Error::Protocol(e.into())))?;

        let error_data_base64 = encode(&error_data_buffer, Encoding::Base64);

        Ok(error_data_base64)
    }
}

impl From<&QueryError> for HandlerError {
    fn from(value: &QueryError) -> Self {
        match value {
            QueryError::NotFound(message) => HandlerError::NotFound(message.to_owned()),
            QueryError::InvalidArgument(message) => {
                HandlerError::InvalidArgument(message.to_owned())
            }
            QueryError::Query(error) => HandlerError::InvalidArgument(error.to_string()),
            _ => HandlerError::Unknown(value.to_string()),
        }
    }
}

impl From<&ConsensusError> for HandlerError {
    fn from(value: &ConsensusError) -> Self {
        Self::StateTransitionConsensusError(value.to_owned())
    }
}

impl From<&Error> for HandlerError {
    fn from(value: &Error) -> Self {
        Self::Internal(value.to_string())
    }
}
