pub mod consensus;

use crate::error::query::QueryError;
use crate::error::Error;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;
use dpp::platform_value::platform_value;
use dpp::platform_value::string_encoding::{encode, Encoding};
use tenderdash_abci::proto::abci as proto;

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
    pub fn response_info(&self) -> Result<String, Error> {
        let error_data_buffer = platform_value!({
            "message": self.message(),
            // TODO: consider capturing stack with one of the libs
            //   and send it to the client
            //"stack": "..."
        })
        .to_cbor_buffer()?;

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

pub fn error_into_exception(error: Error) -> proto::ResponseException {
    proto::ResponseException {
        error: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::execution::ExecutionError;

    #[test]
    fn handler_error_code_cancelled() {
        let err = HandlerError::Cancelled("cancelled".into());
        assert_eq!(err.code(), HandlerErrorCode::Cancelled as u32);
        assert_eq!(err.code(), 1);
    }

    #[test]
    fn handler_error_code_unknown() {
        let err = HandlerError::Unknown("unknown".into());
        assert_eq!(err.code(), HandlerErrorCode::Unknown as u32);
        assert_eq!(err.code(), 2);
    }

    #[test]
    fn handler_error_code_invalid_argument() {
        let err = HandlerError::InvalidArgument("bad arg".into());
        assert_eq!(err.code(), HandlerErrorCode::InvalidArgument as u32);
        assert_eq!(err.code(), 3);
    }

    #[test]
    fn handler_error_code_deadline_exceeded() {
        let err = HandlerError::DeadlineExceeded("timeout".into());
        assert_eq!(err.code(), HandlerErrorCode::DeadlineExceeded as u32);
        assert_eq!(err.code(), 4);
    }

    #[test]
    fn handler_error_code_not_found() {
        let err = HandlerError::NotFound("missing".into());
        assert_eq!(err.code(), HandlerErrorCode::NotFound as u32);
        assert_eq!(err.code(), 5);
    }

    #[test]
    fn handler_error_code_already_exists() {
        let err = HandlerError::AlreadyExists("exists".into());
        assert_eq!(err.code(), HandlerErrorCode::AlreadyExists as u32);
        assert_eq!(err.code(), 6);
    }

    #[test]
    fn handler_error_code_permission_denied() {
        let err = HandlerError::PermissionDenied("denied".into());
        assert_eq!(err.code(), HandlerErrorCode::PermissionDenied as u32);
        assert_eq!(err.code(), 7);
    }

    #[test]
    fn handler_error_code_resource_exhausted() {
        let err = HandlerError::ResourceExhausted("exhausted".into());
        assert_eq!(err.code(), HandlerErrorCode::ResourceExhausted as u32);
        assert_eq!(err.code(), 8);
    }

    #[test]
    fn handler_error_code_failed_precondition() {
        let err = HandlerError::FailedPrecondition("precondition".into());
        assert_eq!(err.code(), HandlerErrorCode::FailedPrecondition as u32);
        assert_eq!(err.code(), 9);
    }

    #[test]
    fn handler_error_code_aborted() {
        let err = HandlerError::Aborted("aborted".into());
        assert_eq!(err.code(), HandlerErrorCode::Aborted as u32);
        assert_eq!(err.code(), 10);
    }

    #[test]
    fn handler_error_code_out_of_range() {
        let err = HandlerError::OutOfRange("range".into());
        assert_eq!(err.code(), HandlerErrorCode::OutOfRange as u32);
        assert_eq!(err.code(), 11);
    }

    #[test]
    fn handler_error_code_unimplemented() {
        let err = HandlerError::Unimplemented("not impl".into());
        assert_eq!(err.code(), HandlerErrorCode::Unimplemented as u32);
        assert_eq!(err.code(), 12);
    }

    #[test]
    fn handler_error_code_internal() {
        let err = HandlerError::Internal("internal".into());
        assert_eq!(err.code(), HandlerErrorCode::Internal as u32);
        assert_eq!(err.code(), 13);
    }

    #[test]
    fn handler_error_code_unavailable() {
        let err = HandlerError::Unavailable("unavailable".into());
        assert_eq!(err.code(), HandlerErrorCode::Unavailable as u32);
        assert_eq!(err.code(), 14);
    }

    #[test]
    fn handler_error_code_data_loss() {
        let err = HandlerError::DataLoss("data lost".into());
        assert_eq!(err.code(), HandlerErrorCode::DataLoss as u32);
        assert_eq!(err.code(), 15);
    }

    #[test]
    fn handler_error_code_unauthenticated() {
        let err = HandlerError::Unauthenticated("unauth".into());
        assert_eq!(err.code(), HandlerErrorCode::Unauthenticated as u32);
        assert_eq!(err.code(), 16);
    }

    #[test]
    fn handler_error_code_consensus_error() {
        let consensus_error = ConsensusError::DefaultError;
        let err = HandlerError::StateTransitionConsensusError(consensus_error);
        // DefaultError has code 1
        assert_eq!(err.code(), 1);
    }

    #[test]
    fn handler_error_message_returns_inner_string() {
        let test_cases = vec![
            (
                HandlerError::Cancelled("cancelled msg".into()),
                "cancelled msg",
            ),
            (HandlerError::Unknown("unknown msg".into()), "unknown msg"),
            (HandlerError::InvalidArgument("arg msg".into()), "arg msg"),
            (
                HandlerError::DeadlineExceeded("deadline msg".into()),
                "deadline msg",
            ),
            (
                HandlerError::NotFound("not found msg".into()),
                "not found msg",
            ),
            (
                HandlerError::AlreadyExists("exists msg".into()),
                "exists msg",
            ),
            (
                HandlerError::PermissionDenied("denied msg".into()),
                "denied msg",
            ),
            (
                HandlerError::ResourceExhausted("exhausted msg".into()),
                "exhausted msg",
            ),
            (
                HandlerError::FailedPrecondition("precond msg".into()),
                "precond msg",
            ),
            (HandlerError::Aborted("aborted msg".into()), "aborted msg"),
            (HandlerError::OutOfRange("range msg".into()), "range msg"),
            (
                HandlerError::Unimplemented("unimpl msg".into()),
                "unimpl msg",
            ),
            (
                HandlerError::Internal("internal msg".into()),
                "internal msg",
            ),
            (
                HandlerError::Unavailable("unavail msg".into()),
                "unavail msg",
            ),
            (HandlerError::DataLoss("loss msg".into()), "loss msg"),
            (
                HandlerError::Unauthenticated("unauth msg".into()),
                "unauth msg",
            ),
        ];

        for (err, expected) in test_cases {
            assert_eq!(err.message(), expected, "message mismatch for {:?}", err);
        }
    }

    #[test]
    fn handler_error_message_consensus_error() {
        let consensus_error = ConsensusError::DefaultError;
        let err = HandlerError::StateTransitionConsensusError(consensus_error);
        // ConsensusError::DefaultError displays as "default error"
        assert_eq!(err.message(), "default error");
    }

    #[test]
    fn handler_error_response_info_returns_base64_encoded_cbor() {
        let err = HandlerError::Internal("some internal error".into());
        let info = err.response_info().expect("should produce response info");

        // The result should be a non-empty base64 string
        assert!(!info.is_empty());

        // It should be valid base64
        use dpp::platform_value::string_encoding::{decode, Encoding};
        let decoded = decode(&info, Encoding::Base64).expect("should be valid base64");
        assert!(!decoded.is_empty());
    }

    #[test]
    fn handler_error_display_matches_message() {
        let err = HandlerError::Internal("display test".into());
        assert_eq!(err.to_string(), "display test");

        let err = HandlerError::NotFound("missing item".into());
        assert_eq!(err.to_string(), "missing item");
    }

    #[test]
    fn handler_error_from_query_error_not_found() {
        let query_err = QueryError::NotFound("item not found".to_string());
        let handler_err = HandlerError::from(&query_err);
        assert!(matches!(handler_err, HandlerError::NotFound(_)));
        assert_eq!(handler_err.message(), "item not found");
    }

    #[test]
    fn handler_error_from_query_error_invalid_argument() {
        let query_err = QueryError::InvalidArgument("bad argument".to_string());
        let handler_err = HandlerError::from(&query_err);
        assert!(matches!(handler_err, HandlerError::InvalidArgument(_)));
        assert_eq!(handler_err.message(), "bad argument");
    }

    #[test]
    fn handler_error_from_query_error_query() {
        use drive::error::query::QuerySyntaxError;
        let query_err = QueryError::Query(QuerySyntaxError::InvalidSQL("bad sql".to_string()));
        let handler_err = HandlerError::from(&query_err);
        assert!(matches!(handler_err, HandlerError::InvalidArgument(_)));
    }

    #[test]
    fn handler_error_from_query_error_other_maps_to_unknown() {
        let query_err = QueryError::DecodingError("decoding failed".to_string());
        let handler_err = HandlerError::from(&query_err);
        assert!(matches!(handler_err, HandlerError::Unknown(_)));
    }

    #[test]
    fn handler_error_from_consensus_error() {
        let consensus_err = ConsensusError::DefaultError;
        let handler_err = HandlerError::from(&consensus_err);
        assert!(matches!(
            handler_err,
            HandlerError::StateTransitionConsensusError(_)
        ));
    }

    #[test]
    fn handler_error_from_error() {
        let error = Error::Execution(ExecutionError::CorruptedCodeExecution("test failure"));
        let handler_err = HandlerError::from(&error);
        assert!(matches!(handler_err, HandlerError::Internal(_)));
        assert!(handler_err.message().contains("test failure"));
    }

    #[test]
    fn error_into_exception_produces_response_exception() {
        let error = Error::Execution(ExecutionError::CorruptedCodeExecution("exception test"));
        let exception = error_into_exception(error);
        assert!(exception.error.contains("exception test"));
    }
}
