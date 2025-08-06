use thiserror::Error;

#[derive(Error, Debug)]
pub enum DapiError {
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("HTTP error: {0}")]
    Http(#[from] axum::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

impl From<DapiError> for tonic::Status {
    fn from(err: DapiError) -> Self {
        match err {
            DapiError::InvalidArgument(msg) => tonic::Status::invalid_argument(msg),
            DapiError::NotFound(msg) => tonic::Status::not_found(msg),
            DapiError::ServiceUnavailable(msg) => tonic::Status::unavailable(msg),
            DapiError::Internal(msg) => tonic::Status::internal(msg),
            _ => tonic::Status::internal(err.to_string()),
        }
    }
}

pub type DapiResult<T> = Result<T, DapiError>;
