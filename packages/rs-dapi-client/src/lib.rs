//! This crate provides [DapiClient] --- transport layer for a decentralized API for Dash.

#![deny(missing_docs)]

mod address_list;
mod connection_pool;
mod dapi_client;
#[cfg(feature = "dump")]
pub mod dump;
#[cfg(feature = "mocks")]
pub mod mock;
mod request_settings;
pub mod transport;

pub use address_list::Address;
pub use address_list::AddressList;
pub use connection_pool::ConnectionPool;
pub use dapi_client::DapiRequestExecutor;
pub use dapi_client::{DapiClient, DapiClientError};
use dapi_grpc::mock::Mockable;
#[cfg(feature = "dump")]
pub use dump::DumpData;
use futures::{future::BoxFuture, FutureExt};
pub use request_settings::RequestSettings;
use std::error::Error;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

/// A DAPI request could be executed with an initialized [DapiClient].
///
/// # Examples
/// ```
/// use std::sync::Arc;
/// use rs_dapi_client::{RequestSettings, AddressList, mock::MockDapiClient, DapiClientError, DapiRequest};
/// use dapi_grpc::platform::v0::{self as proto, GetIdentityResponse};
/// use rs_dapi_client::mock::DummyProcessingError;
///
/// # let _ = async {
/// let mut client = MockDapiClient::new();
/// let request: proto::GetIdentityRequest = proto::get_identity_request::GetIdentityRequestV0 { id: b"0".to_vec(), prove: true }.into();
/// let process_response = Arc::new(|response| async move { Ok::<GetIdentityResponse, DummyProcessingError>(response) });
/// let response = request.execute(&mut client, process_response, RequestSettings::default()).await?;
/// # Ok::<(), DapiClientError<_, _>>(())
/// # };
/// ```
pub trait DapiRequest {
    /// Response from DAPI for this specific request.
    type Response: Send;
    /// An error type for the transport this request uses.
    type TransportError: Mockable;

    /// Executes the request.
    fn execute<'c, D, O, PE, F, Fut>(
        self,
        dapi_client: &'c D,
        process_response: Arc<F>,
        settings: RequestSettings,
    ) -> BoxFuture<'c, Result<O, DapiClientError<Self::TransportError, PE>>>
    where
        D: DapiRequestExecutor,
        PE: Error + Mockable + CanRetry + Send + Sync + 'static,
        O: Debug + Mockable + Send + Sync + 'static,
        F: Fn(Self::Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, PE>> + Send + 'static,
        Self: 'c;
}

/// The trait is intentionally made sealed since it defines what is possible to send to DAPI.
impl<T: transport::TransportRequest + Send> DapiRequest for T {
    type Response = T::Response;

    type TransportError = <T::Client as transport::TransportClient>::Error;

    fn execute<'c, D, O, PE, F, Fut>(
        self,
        dapi_client: &'c D,
        process_response: Arc<F>,
        settings: RequestSettings,
    ) -> BoxFuture<'c, Result<O, DapiClientError<Self::TransportError, PE>>>
    where
        D: DapiRequestExecutor,
        PE: Error + Mockable + CanRetry + Send + 'static,
        O: Debug + Mockable + Send + 'static,
        F: Fn(Self::Response) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<O, PE>> + Send + 'static,
        Self: 'c,
    {
        dapi_client
            .execute(self, process_response, settings)
            .boxed()
    }
}

/// Returns true if the operation can be retried.
pub trait CanRetry {
    /// Returns true if the operation can be retried safely.
    fn can_retry(&self) -> bool;

    /// Get boolean flag that indicates if the error is retryable.
    ///
    /// Depreacted in favor of [CanRetry::can_retry].
    #[deprecated = "Use !can_retry() instead"]
    fn is_node_failure(&self) -> bool {
        !self.can_retry()
    }
}
