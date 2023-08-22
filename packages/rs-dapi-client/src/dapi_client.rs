//! [DapiClient] definition.

use backon::{ExponentialBuilder, Retryable};
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
}

impl<TE: CanRetry> CanRetry for DapiClientError<TE> {
    fn can_retry(&self) -> bool {
        use DapiClientError::*;
        match self {
            NoAvailableAddresses => false,
            Transport(transport_error) => transport_error.can_retry(),
        }
    }
}

/// Access point to DAPI.
#[derive(Debug)]
pub struct DapiClient {
    address_list: AddressList,
    settings: RequestSettings,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        Self {
            address_list,
            settings,
        }
    }

    /// Execute the [DapiRequest].
    pub(crate) async fn execute<'c, R>(
        &'c mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>>
    where
        R: TransportRequest,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();

        // Setup retry policy:
        let retry_settings = ExponentialBuilder::default().with_max_times(applied_settings.retries);

        // Setup DAPI request execution routine future. It's a closure that will be called
        // more once to build new future on each retry.
        let routine = move || {
            // Try to get an address to initialize transport on:
            let address = self.address_list.get_live_address().ok_or(
                DapiClientError::<<R::Client as TransportClient>::Error>::NoAvailableAddresses,
            );

            // Get a transport client requried by the DAPI request from this DAPI client.
            // It stays wrapped in `Result` since we want to return
            // `impl Future<Output = Result<...>`, not a `Result` itself.
            let transport_client = address.map(|addr| R::Client::with_uri(addr.uri().clone()));

            let transport_request = request.clone();

            // Create a future using `async` block that will be returned from the closure on
            // each retry. Could be just a request future, but need to unpack client first.
            async move {
                transport_request
                    .execute_transport(&mut transport_client?, &applied_settings)
                    .await
                    .map_err(|e| {
                        DapiClientError::<<R::Client as TransportClient>::Error>::Transport(e)
                    })
            }
        };

        // Start the routine with retry policy applied:
        routine
            .retry(&retry_settings)
            .when(|e| e.can_retry())
            .instrument(tracing::info_span!("request routine"))
            .await
    }
}
