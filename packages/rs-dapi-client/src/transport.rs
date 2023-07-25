//! Transport options that DAPI requests use under the hood.

pub(crate) mod grpc;
pub(crate) mod json_rpc;

use futures::future::BoxFuture;
use http::Uri;

use crate::{request_settings::AppliedRequestSettings, RequestSettings};

/// Generic transport layer request.
/// Requires [Clone] as could be retried and a client in general consumes a request.
pub trait TransportRequest: Clone {
    /// A client specific to this type of transport.
    type Client: TransportClient;

    /// Transport layer response.
    type Response;

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
    type Error: CanRetry + Send;

    /// Build client using peer's url.
    fn with_uri(uri: Uri) -> Self;
}

/// Allows to flag the transport error variant how tolerant we are of it and whether we can
/// try to do a request again.
pub trait CanRetry {
    /// Get boolean flag that indicates if the error is retryable.
    fn can_retry(&self) -> bool;
}
