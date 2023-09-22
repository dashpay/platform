use crate::error::query::QueryError;

/// ABCI handlers errors
#[derive(Debug, thiserror::Error)]
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
}

/// Error codes for ABCI handlers
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
    pub fn code(&self) -> HandlerErrorCode {
        match self {
            HandlerError::Cancelled(_) => HandlerErrorCode::Cancelled,
            HandlerError::Unknown(_) => HandlerErrorCode::Unknown,
            HandlerError::InvalidArgument(_) => HandlerErrorCode::InvalidArgument,
            HandlerError::DeadlineExceeded(_) => HandlerErrorCode::DeadlineExceeded,
            HandlerError::NotFound(_) => HandlerErrorCode::NotFound,
            HandlerError::AlreadyExists(_) => HandlerErrorCode::AlreadyExists,
            HandlerError::PermissionDenied(_) => HandlerErrorCode::PermissionDenied,
            HandlerError::ResourceExhausted(_) => HandlerErrorCode::ResourceExhausted,
            HandlerError::FailedPrecondition(_) => HandlerErrorCode::FailedPrecondition,
            HandlerError::Aborted(_) => HandlerErrorCode::Aborted,
            HandlerError::OutOfRange(_) => HandlerErrorCode::OutOfRange,
            HandlerError::Unimplemented(_) => HandlerErrorCode::Unimplemented,
            HandlerError::Internal(_) => HandlerErrorCode::Internal,
            HandlerError::Unavailable(_) => HandlerErrorCode::Unavailable,
            HandlerError::DataLoss(_) => HandlerErrorCode::DataLoss,
            HandlerError::Unauthenticated(_) => HandlerErrorCode::Unauthenticated,
        }
    }
}

impl From<&QueryError> for HandlerError {
    fn from(value: &QueryError) -> Self {
        match value {
            QueryError::NotFound(message) => HandlerError::NotFound(message.to_owned()),
            QueryError::InvalidArgument(message) => {
                HandlerError::InvalidArgument(message.to_owned())
            }
            QueryError::DocumentQuery(error) => HandlerError::InvalidArgument(error.to_string()),
            _ => HandlerError::Unknown(value.to_string()),
        }
    }
}
