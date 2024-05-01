//! Transport options that DAPI requests use under the hood.

pub(crate) mod grpc;

use crate::connection_pool::ConnectionPool;
pub use crate::request_settings::AppliedRequestSettings;
use crate::{CanRetry, RequestSettings};
use dapi_grpc::mock::Mockable;
pub use futures::future::BoxFuture;
pub use grpc::{CoreGrpcClient, PlatformGrpcClient};
use http::Uri;
use std::any;
use std::fmt::Debug;

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
    ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>>;
}

/// Generic way to create a transport client from provided [Uri].
pub trait TransportClient: Send + Sized {
    /// Error type for the specific client.
    type Error: CanRetry + Send + Debug;

    /// Build client using node's url.
    fn with_uri(uri: Uri, pool: &ConnectionPool) -> Self;

    /// Build client using node's url and [AppliedRequestSettings].
    fn with_uri_and_settings(
        uri: Uri,
        settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Self;
}
