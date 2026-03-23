//! Transport options that DAPI requests use under the hood.

pub(crate) mod grpc;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod tonic_channel;
#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm_channel;

use crate::connection_pool::ConnectionPool;
pub use crate::request_settings::AppliedRequestSettings;
use crate::{CanRetry, RequestSettings, Uri};
use dapi_grpc::mock::Mockable;
pub use futures::future::BoxFuture;
use std::any;
use std::fmt::Debug;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub use tonic_channel::{
    create_channel, CoreGrpcClient, PlatformGrpcClient, TokioBackonSleeper as BackonSleeper,
};
#[cfg(target_arch = "wasm32")]
pub use wasm_channel::{
    create_channel, CoreGrpcClient, PlatformGrpcClient, WasmBackonSleeper as BackonSleeper,
};

/// Sleep for the given duration.
#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Sleep for the given duration.
#[cfg(target_arch = "wasm32")]
pub async fn sleep(duration: Duration) {
    wasm_channel::into_send_sleep(duration).await;
}

/// Generic transport layer request.
/// Requires [Clone] as could be retried and a client in general consumes a request.
pub trait TransportRequest: Clone + Send + Sync + Debug + Mockable {
    /// A client specific to this type of transport.
    type Client: TransportClient;

    /// Transport layer response.
    type Response: Mockable + Send + Debug;

    /// Settings that will override [DapiClient](crate::DapiClient)'s ones each time the request is executed.
    const SETTINGS_OVERRIDES: RequestSettings;

    /// gRPC request name
    fn request_name(&self) -> &'static str {
        any::type_name::<Self>()
    }

    /// gRPC response name
    fn response_name(&self) -> &'static str {
        any::type_name::<Self::Response>()
    }

    /// gRPC method name
    fn method_name(&self) -> &'static str;

    /// Perform transport request asynchronously.
    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>>;
}

/// Transport error type.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum TransportError {
    /// gRPC error
    #[error("grpc error: {0}")]
    Grpc(
        #[from]
        #[cfg_attr(feature = "mocks", serde(with = "dapi_grpc::mock::serde_mockable"))]
        dapi_grpc::tonic::Status,
    ),
}

impl Clone for TransportError {
    fn clone(&self) -> Self {
        match self {
            TransportError::Grpc(status) => {
                // tonic::Status doesn't implement Clone, so we reconstruct it
                // from its components. Note: this loses the original error source.
                let cloned_status = dapi_grpc::tonic::Status::with_details_and_metadata(
                    status.code(),
                    status.message(),
                    status.details().to_vec().into(),
                    status.metadata().clone(),
                );
                TransportError::Grpc(cloned_status)
            }
        }
    }
}

impl CanRetry for TransportError {
    fn can_retry(&self) -> bool {
        match self {
            TransportError::Grpc(status) => status.can_retry(),
        }
    }
}

/// Serialization of [TransportError].
///
/// We need to do manual serialization because of the generic type parameter which doesn't support serde derive.
impl Mockable for TransportError {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        Some(serde_json::to_vec(self).expect("serialize Transport error"))
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        Some(serde_json::from_slice(data).expect("deserialize Transport error"))
    }
}

/// Serialization of boxed [TransportError].
impl Mockable for Box<TransportError> {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        self.as_ref().mock_serialize()
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        TransportError::mock_deserialize(data).map(Box::new)
    }
}

/// Generic way to create a transport client from provided [Uri].
pub trait TransportClient: Send + Sized {
    /// Build client using node's url.
    fn with_uri(uri: Uri, pool: &ConnectionPool) -> Result<Self, TransportError>;

    /// Build client using node's url and [AppliedRequestSettings].
    fn with_uri_and_settings(
        uri: Uri,
        settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Result<Self, TransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use dapi_grpc::tonic::Code;

    #[test]
    fn test_tonic_status_can_retry_retryable_codes() {
        let retryable_codes = vec![
            Code::Ok,
            Code::DataLoss,
            Code::Cancelled,
            Code::Unknown,
            Code::DeadlineExceeded,
            Code::ResourceExhausted,
            Code::Aborted,
            Code::Internal,
            Code::Unavailable,
        ];

        for code in retryable_codes {
            let status = dapi_grpc::tonic::Status::new(code, "test");
            assert!(
                status.can_retry(),
                "Expected code {:?} to be retryable",
                code
            );
        }
    }

    #[test]
    fn test_tonic_status_can_retry_non_retryable_codes() {
        let non_retryable_codes = vec![
            Code::InvalidArgument,
            Code::NotFound,
            Code::AlreadyExists,
            Code::PermissionDenied,
            Code::FailedPrecondition,
            Code::OutOfRange,
            Code::Unimplemented,
            Code::Unauthenticated,
        ];

        for code in non_retryable_codes {
            let status = dapi_grpc::tonic::Status::new(code, "test");
            assert!(
                !status.can_retry(),
                "Expected code {:?} to be non-retryable",
                code
            );
        }
    }

    #[test]
    fn test_transport_error_can_retry() {
        let retryable = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        assert!(retryable.can_retry());

        let non_retryable = TransportError::Grpc(dapi_grpc::tonic::Status::not_found("permanent"));
        assert!(!non_retryable.can_retry());
    }

    #[test]
    fn test_transport_error_clone() {
        let original = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("test message"));

        let cloned = original.clone();

        match (&original, &cloned) {
            (TransportError::Grpc(orig), TransportError::Grpc(clone)) => {
                assert_eq!(orig.code(), clone.code());
                assert_eq!(orig.message(), clone.message());
            }
        }
    }

    #[test]
    fn test_transport_error_display() {
        let err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("service down"));
        let display = format!("{}", err);
        assert!(display.contains("service down"));
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_transport_error_mock_roundtrip() {
        let original =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("test roundtrip"));
        let serialized = original.mock_serialize().expect("should serialize");
        let deserialized =
            TransportError::mock_deserialize(&serialized).expect("should deserialize");

        match deserialized {
            TransportError::Grpc(status) => {
                assert_eq!(status.code(), Code::Unavailable);
            }
        }
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_boxed_transport_error_mock_roundtrip() {
        let original = Box::new(TransportError::Grpc(dapi_grpc::tonic::Status::internal(
            "boxed test",
        )));
        let serialized = original.mock_serialize().expect("should serialize");
        let deserialized =
            Box::<TransportError>::mock_deserialize(&serialized).expect("should deserialize");

        match *deserialized {
            TransportError::Grpc(status) => {
                assert_eq!(status.code(), Code::Internal);
            }
        }
    }
}
