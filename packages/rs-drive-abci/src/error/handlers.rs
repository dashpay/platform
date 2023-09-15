use crate::error::query::QueryError;

/// ABCI handlers errors
#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    /// ABCI Handler error (Cancelled)
    #[error("ABCI Handler error (Cancelled): {0}")]
    Cancelled(String),
    /// ABCI Handler error (Unknown)
    #[error("ABCI Handler error (Unknown): {0}")]
    Unknown(String),
    /// ABCI Handler error (InvalidArgument)
    #[error("ABCI Handler error (InvalidArgument): {0}")]
    InvalidArgument(String),
    /// ABCI Handler error (DeadlineExceeded)
    #[error("ABCI Handler error (DeadlineExceeded): {0}")]
    DeadlineExceeded(String),
    /// ABCI Handler error (NotFound)
    #[error("ABCI Handler error (NotFound): {0}")]
    NotFound(String),
    /// ABCI Handler error (AlreadyExists)
    #[error("ABCI Handler error (AlreadyExists): {0}")]
    AlreadyExists(String),
    /// ABCI Handler error (PermissionDenied)
    #[error("ABCI Handler error (PermissionDenied): {0}")]
    PermissionDenied(String),
    /// ABCI Handler error (ResourceExhausted)
    #[error("ABCI Handler error (ResourceExhausted): {0}")]
    ResourceExhausted(String),
    /// ABCI Handler error (FailedPrecondition)
    #[error("ABCI Handler error (FailedPrecondition): {0}")]
    FailedPrecondition(String),
    /// ABCI Handler error (Aborted)
    #[error("ABCI Handler error (Aborted): {0}")]
    Aborted(String),
    /// ABCI Handler error (OutOfRange)
    #[error("ABCI Handler error (OutOfRange): {0}")]
    OutOfRange(String),
    /// ABCI Handler error (Unimplemented)
    #[error("ABCI Handler error (Unimplemented): {0}")]
    Unimplemented(String),
    /// ABCI Handler error (Internal)
    #[error("ABCI Handler error (Internal): {0}")]
    Internal(String),
    /// ABCI Handler error (Unavailable)
    #[error("ABCI Handler error (Unavailable): {0}")]
    Unavailable(String),
    /// ABCI Handler error (DataLoss)
    #[error("ABCI Handler error (DataLoss): {0}")]
    DataLoss(String),
    /// ABCI Handler error (Unauthenticated)
    #[error("ABCI Handler error (Unauthenticated): {0}")]
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
