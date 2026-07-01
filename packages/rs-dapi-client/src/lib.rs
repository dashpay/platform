//! This crate provides [DapiClient] --- transport layer for a decentralized API for Dash.

#![deny(missing_docs)]

mod address_ban_info;
mod address_list;
mod connection_pool;
mod dapi_client;
#[cfg(feature = "dump")]
pub mod dump;
mod executor;
#[cfg(feature = "mocks")]
pub mod mock;
mod request_settings;
pub mod transport;

pub use address_ban_info::AddressBanInfo;
pub use address_list::Address;
pub use address_list::AddressList;
pub use address_list::AddressListError;
pub use address_list::AddressStatus;
pub use connection_pool::ConnectionPool;
pub use dapi_client::{update_address_ban_status, DapiClient, DapiClientError};
#[cfg(feature = "dump")]
pub use dump::DumpData;
pub use executor::{
    DapiRequestExecutor, ExecutionError, ExecutionResponse, ExecutionResult, InnerInto, IntoInner,
    WrapToExecutionResult,
};
use futures::{future::BoxFuture, FutureExt};
#[cfg(any(target_arch = "wasm32", not(feature = "mocks")))]
pub use http::Uri;
#[cfg(all(feature = "mocks", not(target_arch = "wasm32")))]
pub use http_serde::http::Uri;
pub use request_settings::RequestSettings;

/// A DAPI request could be executed with an initialized [DapiClient].
///
/// # Examples
/// Requires the `mocks` feature.
/// ```
/// # #[cfg(feature = "mocks")]
/// # {
/// use rs_dapi_client::{RequestSettings, AddressList, mock::MockDapiClient, DapiClientError, DapiRequest, ExecutionError};
/// use dapi_grpc::platform::v0::{self as proto};
///
/// # let _ = async {
/// let mut client = MockDapiClient::new();
/// let request: proto::GetIdentityRequest = proto::get_identity_request::GetIdentityRequestV0 { id: b"0".to_vec(), prove: true }.into();
/// let response = request.execute(&mut client, RequestSettings::default()).await?;
/// # Ok::<(), ExecutionError<DapiClientError>>(())
/// # };
/// # }
/// ```
pub trait DapiRequest {
    /// Response from DAPI for this specific request.
    type Response;

    /// Executes the request.
    fn execute<'c, D: DapiRequestExecutor>(
        self,
        dapi_client: &'c D,
        settings: RequestSettings,
    ) -> BoxFuture<'c, ExecutionResult<Self::Response, DapiClientError>>
    where
        Self: 'c;
}

/// The trait is intentionally made sealed since it defines what is possible to send to DAPI.
impl<T: transport::TransportRequest + Send> DapiRequest for T {
    type Response = T::Response;

    fn execute<'c, D: DapiRequestExecutor>(
        self,
        dapi_client: &'c D,
        settings: RequestSettings,
    ) -> BoxFuture<'c, ExecutionResult<Self::Response, DapiClientError>>
    where
        Self: 'c,
    {
        dapi_client.execute(self, settings).boxed()
    }
}

/// Returns true if the operation can be retried.
pub trait CanRetry {
    /// Returns true if the operation can be retried safely.
    fn can_retry(&self) -> bool;

    /// Returns true if this error represents a "no available addresses" condition.
    ///
    /// When all addresses have been banned due to errors, the client returns this error.
    /// Retry logic uses this to return the last meaningful error instead of this one.
    fn is_no_available_addresses(&self) -> bool {
        false
    }

    /// If this error is a gRPC `ResourceExhausted` (Envoy rate-limit) that
    /// carries a `RateLimit-Reset` metadata header, returns the server-advertised
    /// ban duration (clamped to a safe range).  Returns `None` for all other
    /// errors and for rate-limit errors that carry no usable header (the caller
    /// falls back to the normal exponential ban ladder in that case).
    fn rate_limit_ban_duration(&self) -> Option<std::time::Duration> {
        None
    }

    /// Get boolean flag that indicates if the error is retryable.
    ///
    /// Deprecated in favor of [CanRetry::can_retry].
    #[deprecated = "Use !can_retry() instead"]
    fn is_node_failure(&self) -> bool {
        !self.can_retry()
    }
}
