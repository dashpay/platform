//! Transport options that DAPI requests use under the hood.

pub(crate) mod grpc;

use std::fmt::Debug;

pub use futures::future::BoxFuture;
use http::Uri;

pub use crate::request_settings::AppliedRequestSettings;
use crate::{CanRetry, RequestSettings};

/// Generic transport layer request.
/// Requires [Clone] as could be retried and a client in general consumes a request.
pub trait TransportRequest: Clone + Send + Sync + Debug {
    /// A client specific to this type of transport.
    type Client: TransportClient;

    /// Transport layer response.
    type Response: Clone + Send + Sync + Debug;

    /// Settings that will override [DapiClient](crate::DapiClient)'s ones each time the request is executed.
    const SETTINGS_OVERRIDES: RequestSettings;

    /// Perform transport request asynchronously.
    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, <Self::Client as TransportClient>::Error>>;
}

/// Generic way to create a transport client from provided [Uri].
pub trait TransportClient: Send {
    /// Error type for the specific client.
    type Error: CanRetry + Send + Debug;

    /// Build client using peer's url.
    fn with_uri(uri: Uri) -> Self;
}
