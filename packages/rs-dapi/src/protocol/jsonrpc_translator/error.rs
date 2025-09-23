use serde_json::Value;

use dapi_grpc::tonic::Code;

use crate::error::DapiError;

pub fn map_error(error: &DapiError) -> (i32, String, Option<Value>) {
    match error {
        DapiError::InvalidArgument(msg)
        | DapiError::InvalidData(msg)
        | DapiError::NotFound(msg)
        | DapiError::FailedPrecondition(msg)
        | DapiError::AlreadyExists(msg)
        | DapiError::NoValidTxProof(msg)
        | DapiError::Client(msg) => (-32602, msg.clone(), None),
        DapiError::ServiceUnavailable(msg)
        | DapiError::Unavailable(msg)
        | DapiError::Timeout(msg) => (-32003, msg.clone(), None),
        DapiError::MethodNotFound(msg) => (-32601, msg.clone(), None),
        DapiError::Status(status) => map_status(status),
        _ => (
            -32603,
            "Internal error".to_string(),
            Some(Value::String(error.to_string())),
        ),
    }
}

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
        | Code::NotFound
        | Code::Aborted
        | Code::ResourceExhausted => (-32602, normalized, None),
        Code::Unavailable | Code::DeadlineExceeded | Code::Cancelled => (-32003, normalized, None),
        _ => (
            -32603,
            "Internal error".to_string(),
            Some(Value::String(status.to_string())),
        ),
    }
}
