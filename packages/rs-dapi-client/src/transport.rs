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

#[cfg(not(target_arch = "wasm32"))]
pub use tonic_channel::{
    create_channel, CoreGrpcClient, PlatformGrpcClient, TokioBackonSleeper as BackonSleeper,
};
#[cfg(target_arch = "wasm32")]
pub use wasm_channel::{
    create_channel, CoreGrpcClient, PlatformGrpcClient, WasmBackonSleeper as BackonSleeper,
};

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
