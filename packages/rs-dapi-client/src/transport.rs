//! Transport options that DAPI requests use under the hood.

pub(crate) mod grpc;

use futures::future::BoxFuture;

use crate::settings::AppliedSettings;

/// Generic transport layer request.
///
/// During execution a DAPI request goes through one of [TransportRequest]
/// implementations depending on a transport required for a specific DAPI request.
pub trait TransportRequest: Clone {
    /// A client specific to this type of transport.
    type Client;

    /// Transport layer response.
    type Response;

    /// Transport layer error.
    type Error;

    /// Perform transport request asynchronously.
    fn execute<'c>(
        self,
        client: &'c mut Self::Client,
        settings: &AppliedSettings,
    ) -> BoxFuture<'c, Result<Self::Response, Self::Error>>;
}
