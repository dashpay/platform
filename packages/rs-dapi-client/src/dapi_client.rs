//! [DapiClient] definition.

use backon::{ExponentialBuilder, Retryable};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
use std::sync::RwLock;
use std::time::Duration;
use tracing::Instrument;

use crate::address_list::AddressListError;
use crate::{
    transport::{TransportClient, TransportRequest},
    Address, AddressList, CanRetry, RequestSettings,
};

/// General DAPI request error type.
#[derive(Debug, thiserror::Error)]
pub enum DapiClientError<TE> {
    /// The error happened on transport layer
    #[error("transport error with {1}: {0}")]
    Transport(TE, Address),
    /// There are no valid DAPI addresses to use.
    #[error("no available addresses to use")]
    NoAvailableAddresses,
    /// [AddressListError] errors
    #[error("address list error: {0}")]
    AddressList(AddressListError),
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
            Transport(transport_error, _) => transport_error.can_retry(),
            AddressList(_) => false,
            #[cfg(feature = "mocks")]
            MockExpectationNotFound(_) => false,
        }
    }
}

#[async_trait]
/// DAPI client executor trait.
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
    address_list: RwLock<AddressList>,
    settings: RequestSettings,
    #[cfg(feature = "dump")]
    pub(crate) dump_dir: Option<std::path::PathBuf>,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        Self {
            address_list: RwLock::new(address_list),
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
        let retry_settings = ExponentialBuilder::default()
            .with_max_times(applied_settings.retries)
            // backon doesn't accept 1.0
            .with_factor(1.001)
            .with_min_delay(Duration::from_secs(0))
            .with_max_delay(Duration::from_secs(0));

        // Save dump dir for later use, as self is moved into routine
        #[cfg(feature = "dump")]
        let dump_dir = self.dump_dir.clone();
        #[cfg(feature = "dump")]
        let dump_request = request.clone();

        // Setup DAPI request execution routine future. It's a closure that will be called
        // more once to build new future on each retry.
        let routine = move || {
            // Try to get an address to initialize transport on:

            let address_list = self
                .address_list
                .read()
                .expect("can't get address list for read");

            let address_result = address_list.get_live_address().cloned().ok_or(
                DapiClientError::<<R::Client as TransportClient>::Error>::NoAvailableAddresses,
            );

            drop(address_list);

            let _span = tracing::trace_span!(
                "execute request",
                address = ?address_result,
                settings = ?applied_settings,
                method = request.method_name(),
            )
            .entered();

            tracing::trace!(
                ?request,
                "calling {} with {} request",
                request.method_name(),
                request.request_name(),
            );

            let transport_request = request.clone();
            let response_name = request.response_name();

            // Create a future using `async` block that will be returned from the closure on
            // each retry. Could be just a request future, but need to unpack client first.
            async move {
                // It stays wrapped in `Result` since we want to return
                // `impl Future<Output = Result<...>`, not a `Result` itself.
                let address = address_result?;

                let mut transport_client =
                    R::Client::with_uri_and_settings(address.uri().clone(), &applied_settings);

                let response = transport_request
                    .execute_transport(&mut transport_client, &applied_settings)
                    .await
                    .map_err(|e| {
                        DapiClientError::<<R::Client as TransportClient>::Error>::Transport(
                            e,
                            address.clone(),
                        )
                    });

                match &response {
                    Ok(_) => {
                        if address.is_banned() {
                            let mut address_list = self
                                .address_list
                                .write()
                                .expect("can't get address list for write");

                            address_list.unban_address(&address)
                                .map_err(DapiClientError::<<R::Client as TransportClient>::Error>::AddressList)?;
                        }

                        tracing::trace!(?response, "received {} response", response_name);
                    }
                    Err(error) => {
                        if error.can_retry() {
                            // let mut address_list = self
                            //     .address_list
                            //     .write()
                            //     .expect("can't get address list for write");
                            //
                            // address_list.ban_address(&address)
                            //     .map_err(DapiClientError::<<R::Client as TransportClient>::Error>::AddressList)?;
                        } else {
                            tracing::trace!(?response, ?error, "received error");
                        }
                    }
                };

                response
            }
        };

        // Start the routine with retry policy applied:
        // We allow let_and_return because `result` is used later if dump feature is enabled
        #[allow(clippy::let_and_return)]
        let result = routine
            .retry(&retry_settings)
            .notify(|error, duration| {
                tracing::warn!(
                    ?error,
                    "retrying error with sleeping {} secs",
                    duration.as_secs_f32()
                )
            })
            .when(|e| e.can_retry())
            .instrument(tracing::info_span!("request routine"))
            .await;

        if let Err(error) = &result {
            if error.can_retry() {
                tracing::error!(?error, "request failed");
            }
        }

        // Dump request and response to disk if dump_dir is set:
        #[cfg(feature = "dump")]
        if let Ok(result) = &result {
            Self::dump_request_response(&dump_request, result, dump_dir);
        }

        result
    }
}
