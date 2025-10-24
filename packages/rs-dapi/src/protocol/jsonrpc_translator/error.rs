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
