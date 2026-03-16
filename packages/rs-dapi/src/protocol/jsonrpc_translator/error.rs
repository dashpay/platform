use serde_json::Value;

use dapi_grpc::tonic::Code;

use crate::error::DapiError;

/// JSON-RPC error code for "not found" errors.
///
/// For backwards compatibility with existing clients, we use -32602 (Invalid params) for not found errors.
const ERR_NOT_FOUND: i32 = -32602;

/// Translate a `DapiError` into JSON-RPC error code, message, and optional data payload.
/// Collapses related client-side errors into shared codes and defers gRPC statuses for finer handling.
pub fn map_error(error: &DapiError) -> (i32, String, Option<Value>) {
    match error {
        DapiError::InvalidArgument(msg)
        | DapiError::InvalidData(msg)
        | DapiError::FailedPrecondition(msg)
        | DapiError::AlreadyExists(msg)
        | DapiError::NoValidTxProof(msg)
        | DapiError::Client(msg) => (-32602, msg.clone(), None),
        DapiError::ServiceUnavailable(msg)
        | DapiError::Unavailable(msg)
        | DapiError::Timeout(msg) => (-32003, msg.clone(), None),
        DapiError::MethodNotFound(msg) => (-32601, msg.clone(), None),
        DapiError::InvalidRequest(msg) => (-32600, msg.clone(), None),
        DapiError::NotFound(msg) => (ERR_NOT_FOUND, msg.clone(), None),
        DapiError::Status(status) => map_status(status),
        _ => (
            -32603,
            "Internal error".to_string(),
            Some(Value::String(error.to_string())),
        ),
    }
}

/// Map a gRPC `Status` into JSON-RPC semantics with fallback messaging.
/// Normalizes empty status messages and groups transport vs validation failures.
fn map_status(status: &dapi_grpc::tonic::Status) -> (i32, String, Option<Value>) {
    let raw_message = status.message().to_string();
    let normalized = if raw_message.is_empty() {
        match status.code() {
            Code::InvalidArgument => "Invalid params".to_string(),
            Code::FailedPrecondition => "Failed precondition".to_string(),
            Code::AlreadyExists => "Already exists".to_string(),
            Code::NotFound => "Not found".to_string(),
            Code::Aborted => "Aborted".to_string(),
            Code::ResourceExhausted => "Resource exhausted".to_string(),
            Code::Unavailable => "Service unavailable".to_string(),
            _ => "Internal error".to_string(),
        }
    } else {
        raw_message
    };

    match status.code() {
        Code::InvalidArgument
        | Code::FailedPrecondition
        | Code::AlreadyExists
        | Code::Aborted
        | Code::ResourceExhausted => (-32602, normalized, None),
        Code::NotFound => (ERR_NOT_FOUND, normalized, None),
        Code::Unavailable | Code::DeadlineExceeded | Code::Cancelled => (-32003, normalized, None),
        _ => (
            -32603,
            "Internal error".to_string(),
            Some(Value::String(status.to_string())),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::tonic;

    // -- map_error additional coverage for variants already tested in parent mod --

    #[test]
    fn map_error_invalid_data() {
        let (code, msg, _) = map_error(&DapiError::InvalidData("bad data".into()));
        assert_eq!(code, -32602);
        assert_eq!(msg, "bad data");
    }

    #[test]
    fn map_error_failed_precondition() {
        let (code, msg, _) = map_error(&DapiError::FailedPrecondition("precond".into()));
        assert_eq!(code, -32602);
        assert_eq!(msg, "precond");
    }

    #[test]
    fn map_error_already_exists() {
        let (code, _, _) = map_error(&DapiError::AlreadyExists("exists".into()));
        assert_eq!(code, -32602);
    }

    #[test]
    fn map_error_no_valid_tx_proof() {
        let (code, msg, _) = map_error(&DapiError::NoValidTxProof("abc123".into()));
        assert_eq!(code, -32602);
        assert_eq!(msg, "abc123");
    }

    #[test]
    fn map_error_client() {
        let (code, _, _) = map_error(&DapiError::Client("client err".into()));
        assert_eq!(code, -32602);
    }

    #[test]
    fn map_error_unavailable() {
        let (code, _, _) = map_error(&DapiError::Unavailable("offline".into()));
        assert_eq!(code, -32003);
    }

    #[test]
    fn map_error_timeout() {
        let (code, _, _) = map_error(&DapiError::Timeout("timed out".into()));
        assert_eq!(code, -32003);
    }

    #[test]
    fn map_error_method_not_found() {
        let (code, msg, _) = map_error(&DapiError::MethodNotFound("no method".into()));
        assert_eq!(code, -32601);
        assert_eq!(msg, "no method");
    }

    #[test]
    fn map_error_invalid_request() {
        let (code, _, _) = map_error(&DapiError::InvalidRequest("bad req".into()));
        assert_eq!(code, -32600);
    }

    #[test]
    fn map_error_not_found() {
        let (code, msg, _) = map_error(&DapiError::NotFound("missing".into()));
        assert_eq!(code, -32602); // ERR_NOT_FOUND
        assert_eq!(msg, "missing");
    }

    #[test]
    fn map_error_fallthrough_returns_internal() {
        let (code, msg, data) = map_error(&DapiError::ConnectionClosed);
        assert_eq!(code, -32603);
        assert_eq!(msg, "Internal error");
        assert!(data.is_some());
    }

    // -- map_error with DapiError::Status delegates to map_status --

    #[test]
    fn map_error_status_delegates_to_map_status() {
        let status = tonic::Status::not_found("item gone");
        let (code, msg, _) = map_error(&DapiError::Status(status));
        assert_eq!(code, -32602); // ERR_NOT_FOUND
        assert_eq!(msg, "item gone");
    }

    // -- map_status tests with messages --

    #[test]
    fn map_status_invalid_argument_with_message() {
        let status = tonic::Status::new(Code::InvalidArgument, "bad param");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32602);
        assert_eq!(msg, "bad param");
    }

    #[test]
    fn map_status_failed_precondition_with_message() {
        let status = tonic::Status::new(Code::FailedPrecondition, "precond fail");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32602);
        assert_eq!(msg, "precond fail");
    }

    #[test]
    fn map_status_already_exists_with_message() {
        let status = tonic::Status::new(Code::AlreadyExists, "dup entry");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32602);
        assert_eq!(msg, "dup entry");
    }

    #[test]
    fn map_status_aborted_with_message() {
        let status = tonic::Status::new(Code::Aborted, "aborted op");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32602);
        assert_eq!(msg, "aborted op");
    }

    #[test]
    fn map_status_resource_exhausted_with_message() {
        let status = tonic::Status::new(Code::ResourceExhausted, "quota exceeded");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32602);
        assert_eq!(msg, "quota exceeded");
    }

    #[test]
    fn map_status_not_found_with_message() {
        let status = tonic::Status::not_found("object missing");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32602); // ERR_NOT_FOUND
        assert_eq!(msg, "object missing");
    }

    #[test]
    fn map_status_unavailable() {
        let status = tonic::Status::unavailable("service down");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32003);
        assert_eq!(msg, "service down");
    }

    #[test]
    fn map_status_deadline_exceeded() {
        let status = tonic::Status::new(Code::DeadlineExceeded, "too slow");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32003);
        assert_eq!(msg, "too slow");
    }

    #[test]
    fn map_status_cancelled() {
        let status = tonic::Status::cancelled("user cancel");
        let (code, msg, _) = map_status(&status);
        assert_eq!(code, -32003);
        assert_eq!(msg, "user cancel");
    }

    #[test]
    fn map_status_unknown_code_returns_internal() {
        let status = tonic::Status::new(Code::DataLoss, "data lost");
        let (code, msg, data) = map_status(&status);
        assert_eq!(code, -32603);
        assert_eq!(msg, "Internal error");
        assert!(data.is_some());
    }

    #[test]
    fn map_status_unimplemented() {
        let status = tonic::Status::new(Code::Unimplemented, "not implemented");
        let (code, msg, data) = map_status(&status);
        assert_eq!(code, -32603);
        assert_eq!(msg, "Internal error");
        assert!(data.is_some(), "fallthrough codes should include data");
    }

    // -- map_status with empty messages uses defaults --

    #[test]
    fn map_status_empty_message_invalid_argument() {
        let status = tonic::Status::new(Code::InvalidArgument, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Invalid params");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_failed_precondition() {
        let status = tonic::Status::new(Code::FailedPrecondition, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Failed precondition");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_already_exists() {
        let status = tonic::Status::new(Code::AlreadyExists, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Already exists");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_not_found() {
        let status = tonic::Status::new(Code::NotFound, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Not found");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_aborted() {
        let status = tonic::Status::new(Code::Aborted, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Aborted");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_resource_exhausted() {
        let status = tonic::Status::new(Code::ResourceExhausted, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Resource exhausted");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_unavailable() {
        let status = tonic::Status::new(Code::Unavailable, "");
        let (_, msg, data) = map_status(&status);
        assert_eq!(msg, "Service unavailable");
        assert!(data.is_none(), "matched codes should not include data");
    }

    #[test]
    fn map_status_empty_message_other_code() {
        let status = tonic::Status::new(Code::PermissionDenied, "");
        let (_, msg, data) = map_status(&status);
        // Other codes with empty message get "Internal error" as the normalized message
        assert_eq!(msg, "Internal error");
        assert!(data.is_some(), "fallthrough codes should include data");
    }
}
