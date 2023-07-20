//! Transport options that DAPI requests use under the hood.

pub(crate) mod grpc;
pub(crate) mod json_rpc;

use futures::future::BoxFuture;
use http::Uri;

use crate::settings::AppliedSettings;

/// Generic transport layer request.
///
/// During execution a DAPI request goes through one of [TransportRequest]
/// implementations depending on a transport required for a specific DAPI request.
pub trait TransportRequest: Clone {
    /// A client specific to this type of transport.
    type Client: TransportClient;

    /// Transport layer response.
    type Response;

    /// Transport layer error.
    type Error: std::error::Error;

    /// Perform transport request asynchronously.
    fn execute<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedSettings,
    ) -> BoxFuture<'c, Result<Self::Response, Self::Error>>;
}

/// Generic way to create a transport client from provided [Uri].
pub trait TransportClient {
    /// Build client using peer's url.
    fn with_uri(uri: Uri) -> Self;
}
