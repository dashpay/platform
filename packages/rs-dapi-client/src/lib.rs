//! This crate provides [DapiClient] --- transport layer for a decentralized API for Dash.

#![deny(missing_docs)]

mod address_list;
mod dapi_client;
mod request_settings;
mod transport;

use futures::{future::BoxFuture, FutureExt};

pub use address_list::AddressList;
pub use dapi_client::{DapiClient, DapiClientError};
pub use request_settings::RequestSettings;

/// A DAPI request could be executed with an initialized [DapiClient].
///
/// # Examples
/// ```no_run
/// use rs_dapi_client::{RequestSettings, AddressList, DapiClient, DapiClientError, DapiRequest};
/// use dapi_grpc::platform::v0::{self as platform_proto};
///
/// # let _ = async {
/// let mut client = DapiClient::new(AddressList::new(), RequestSettings::default());
/// let request = platform_proto::GetIdentityRequest { id: b"0".to_vec(), prove: true };
/// let response = request.execute(&mut client, RequestSettings::default()).await?;
/// # Ok::<(), DapiClientError<_>>(())
/// # };
/// ```
pub trait DapiRequest {
    /// Response from DAPI for this specific request.
    type Response;
    /// An error type for the transport this request uses.
    type TransportError;

    /// Executes the request.
    fn execute<'c>(
        self,
        dapi_client: &'c mut DapiClient,
        settings: RequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, DapiClientError<Self::TransportError>>>
    where
        Self: 'c;
}

/// The trait is intentionally made sealed since it defines what is possible to send to DAPI.
impl<T: transport::TransportRequest + Send> DapiRequest for T {
    type Response = T::Response;

    type TransportError = <T::Client as transport::TransportClient>::Error;

    fn execute<'c>(
        self,
        dapi_client: &'c mut DapiClient,
        settings: RequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, DapiClientError<Self::TransportError>>>
    where
        Self: 'c,
    {
        dapi_client.execute(self, settings).boxed()
    }
}
