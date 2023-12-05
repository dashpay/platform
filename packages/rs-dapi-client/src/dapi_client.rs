//! [DapiClient] definition.

use backon::{ExponentialBuilder, Retryable};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
use tracing::Instrument;

use crate::{
    transport::{TransportClient, TransportRequest},
    AddressList, CanRetry, RequestSettings,
};

/// General DAPI request error type.
#[derive(Debug, thiserror::Error)]
pub enum DapiClientError<TE> {
    /// The error happened on transport layer
    #[error("transport error: {0}")]
    Transport(TE),
    /// There are no valid peer addresses to use.
    #[error("no available addresses to use")]
    NoAvailableAddresses,
    #[cfg(feature = "mocks")]
    /// Expectation not found
    #[error("mock expectation not found for request: {0}")]
    MockExpectationNotFound(String),
}

impl<TE: CanRetry> CanRetry for DapiClientError<TE> {
    fn can_retry(&self) -> bool {
        use DapiClientError::*;
        match self {
            NoAvailableAddresses => false,
            Transport(transport_error) => transport_error.can_retry(),
            #[cfg(feature = "mocks")]
            MockExpectationNotFound(_) => false,
        }
    }
}

#[async_trait]
/// DAPI client trait.
pub trait Dapi {
    /// Execute request using this DAPI client.
    async fn execute<R>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable;
}

/// Access point to DAPI.
#[derive(Debug)]
pub struct DapiClient {
    address_list: AddressList,
    settings: RequestSettings,
    #[cfg(feature = "dump")]
    pub(crate) dump_dir: Option<std::path::PathBuf>,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        Self {
            address_list,
            settings,
            #[cfg(feature = "dump")]
            dump_dir: None,
        }
    }
}

#[async_trait]
impl Dapi for DapiClient {
    /// Execute the [DapiRequest](crate::DapiRequest).
    async fn execute<R>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();

        // Setup retry policy:
        let retry_settings = ExponentialBuilder::default().with_max_times(applied_settings.retries);

        // Save dump dir for later use, as self is moved into routine
        #[cfg(feature = "dump")]
        let dump_dir = self.dump_dir.clone();
        #[cfg(feature = "dump")]
        let dump_request = request.clone();

        // Setup DAPI request execution routine future. It's a closure that will be called
        // more once to build new future on each retry.
        let routine = move || {
            // Try to get an address to initialize transport on:
            let address = self.address_list.get_live_address().ok_or(
                DapiClientError::<<R::Client as TransportClient>::Error>::NoAvailableAddresses,
            );

            let _span = tracing::trace_span!(
                "execute request",
                ?address,
                settings = ?applied_settings,
                method = request.method_name(),
            );

            tracing::trace!(
                ?request,
                "calling {} with {} request",
                request.method_name(),
                request.request_name(),
            );

            // Get a transport client requried by the DAPI request from this DAPI client.
            // It stays wrapped in `Result` since we want to return
            // `impl Future<Output = Result<...>`, not a `Result` itself.
            let transport_client = address.map(|addr| R::Client::with_uri(addr.uri().clone()));

            let transport_request = request.clone();

            // Create a future using `async` block that will be returned from the closure on
            // each retry. Could be just a request future, but need to unpack client first.
            let response_name = request.response_name();
            async move {
                let response = transport_request
                    .execute_transport(&mut transport_client?, &applied_settings)
                    .await
                    .map_err(|e| {
                        DapiClientError::<<R::Client as TransportClient>::Error>::Transport(e)
                    });

                tracing::trace!(?response, "received {} response", response_name);

                response
            }
        };

        // Start the routine with retry policy applied:
        // We allow let_and_return because `result` is used later if dump feature is enabled
        #[allow(clippy::let_and_return)]
        let result = routine
            .retry(&retry_settings)
            .when(|e| e.can_retry())
            .instrument(tracing::info_span!("request routine"))
            .await;

        // Dump request and response to disk if dump_dir is set:
        #[cfg(feature = "dump")]
        if let Ok(result) = &result {
            Self::dump_request_response(&dump_request, result, dump_dir);
        }

        result
    }
}
