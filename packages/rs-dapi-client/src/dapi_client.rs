//! [DapiClient] definition.

use backon::{ConstantBuilder, Retryable};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use dapi_grpc::tonic::transport::Certificate;
use std::fmt::{Debug, Display};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use tracing::Instrument;

use crate::address_list::AddressListError;
use crate::connection_pool::ConnectionPool;
use crate::request_settings::AppliedRequestSettings;
use crate::transport::{self, TransportError};
use crate::{
    transport::{TransportClient, TransportRequest},
    AddressList, CanRetry, DapiRequestExecutor, ExecutionError, ExecutionResponse, ExecutionResult,
    RequestSettings,
};

/// General DAPI request error type.
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "mocks", derive(serde::Serialize, serde::Deserialize))]
pub enum DapiClientError {
    /// The error happened on transport layer
    #[error("transport error: {0}")]
    Transport(
        #[cfg_attr(feature = "mocks", serde(with = "dapi_grpc::mock::serde_mockable"))]
        TransportError,
    ),
    /// There are no valid DAPI addresses to use.
    #[error("no available addresses to use")]
    NoAvailableAddresses,
    /// [AddressListError] errors
    #[error("address list error: {0}")]
    AddressList(AddressListError),

    #[cfg(feature = "mocks")]
    #[error("mock error: {0}")]
    /// Error happened in mock client
    Mock(#[from] crate::mock::MockError),
}

impl CanRetry for DapiClientError {
    fn can_retry(&self) -> bool {
        use DapiClientError::*;
        match self {
            NoAvailableAddresses => false,
            Transport(transport_error) => transport_error.can_retry(),
            AddressList(_) => false,
            #[cfg(feature = "mocks")]
            Mock(_) => false,
        }
    }
}

/// Serialization of [DapiClientError].
///
/// We need to do manual serialization because of the generic type parameter which doesn't support serde derive.
impl Mockable for DapiClientError {
    #[cfg(feature = "mocks")]
    fn mock_serialize(&self) -> Option<Vec<u8>> {
        Some(serde_json::to_vec(self).expect("serialize DAPI client error"))
    }

    #[cfg(feature = "mocks")]
    fn mock_deserialize(data: &[u8]) -> Option<Self> {
        Some(serde_json::from_slice(data).expect("deserialize DAPI client error"))
    }
}

/// Access point to DAPI.
#[derive(Debug, Clone)]
pub struct DapiClient {
    address_list: AddressList,
    settings: RequestSettings,
    pool: ConnectionPool,
    #[cfg(not(target_arch = "wasm32"))]
    /// Certificate Authority certificate to use for verifying the server's certificate.
    pub ca_certificate: Option<Certificate>,
    #[cfg(feature = "dump")]
    pub(crate) dump_dir: Option<std::path::PathBuf>,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        // multiply by 3 as we need to store core and platform addresses, and we want some spare capacity just in case
        let address_count = 3 * address_list.len();

        Self {
            address_list,
            settings,
            pool: ConnectionPool::new(address_count),
            #[cfg(feature = "dump")]
            dump_dir: None,
            #[cfg(not(target_arch = "wasm32"))]
            ca_certificate: None,
        }
    }

    /// Set CA certificate to use when verifying the server's certificate.
    ///
    /// # Arguments
    ///
    /// * `pem_ca_cert` - CA certificate in PEM format.
    ///
    /// # Returns
    /// [DapiClient] with CA certificate set.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_ca_certificate(mut self, ca_cert: Certificate) -> Self {
        self.ca_certificate = Some(ca_cert);

        self
    }

    /// Return the [DapiClient] address list.
    pub fn address_list(&self) -> &AddressList {
        &self.address_list
    }
}

/// Ban address in case of retryable error or unban it
/// if it was banned, and the request was successful.
pub fn update_address_ban_status<R, E>(
    address_list: &AddressList,
    result: &ExecutionResult<R, E>,
    applied_settings: &AppliedRequestSettings,
) where
    E: CanRetry + Display + Debug,
{
    match &result {
        Ok(response) => {
            // Unban the address if it was banned and node responded successfully this time
            if address_list.is_banned(&response.address) {
                if address_list.unban(&response.address) {
                    tracing::debug!(address = ?response.address, "unban successfully responded address {}", response.address);
                } else {
                    // The address might be already removed from the list
                    // by background process (i.e., SML update), and it's fine.
                    tracing::debug!(
                        address = ?response.address,
                        "unable to unban address {} because it's not in the list anymore",
                        response.address
                    );
                }
            }
        }
        Err(error) => {
            if error.can_retry() {
                if let Some(address) = error.address.as_ref() {
                    if applied_settings.ban_failed_address {
                        if address_list.ban(address) {
                            tracing::warn!(
                                ?address,
                                ?error,
                                "ban address {address} due to error: {error}"
                            );
                        } else {
                            // The address might be already removed from the list
                            // by background process (i.e., SML update), and it's fine.
                            tracing::debug!(
                                ?address,
                                ?error,
                                "unable to ban address {address} because it's not in the list anymore"
                            );
                        }
                    } else {
                        tracing::debug!(
                            ?error,
                            ?address,
                            "we should ban the address {address} due to the error but banning is disabled"
                        );
                    }
                } else {
                    tracing::debug!(
                        ?error,
                        "we should ban an address due to the error but address is absent"
                    );
                }
            }
        }
    };
}

#[async_trait]
impl DapiRequestExecutor for DapiClient {
    /// Execute the [DapiRequest](crate::DapiRequest).
    async fn execute<R>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> ExecutionResult<R::Response, DapiClientError>
    where
        R: TransportRequest + Mockable,
        R::Response: Mockable,
        TransportError: Mockable,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();
        #[cfg(not(target_arch = "wasm32"))]
        let applied_settings = applied_settings.with_ca_certificate(self.ca_certificate.clone());

        // Setup retry policy:
        let retry_settings = ConstantBuilder::default()
            .with_max_times(applied_settings.retries)
            .with_delay(Duration::from_millis(10));

        // Save dump dir for later use, as self is moved into routine
        #[cfg(feature = "dump")]
        let dump_dir = self.dump_dir.clone();
        #[cfg(feature = "dump")]
        let dump_request = request.clone();

        let retries_counter_arc = Arc::new(AtomicUsize::new(0));
        let retries_counter_arc_ref = &retries_counter_arc;

        // We need reference so that the closure is FnMut
        let applied_settings_ref = &applied_settings;

        // Setup DAPI request execution routine future. It's a closure that will be called
        // more once to build new future on each retry.
        let routine = move || {
            let retries_counter = Arc::clone(retries_counter_arc_ref);

            // Try to get an address to initialize transport on:
            let address_result = self
                .address_list
                .get_live_address()
                .ok_or(DapiClientError::NoAvailableAddresses);

            let _span = tracing::trace_span!(
                "execute request",
                address = ?address_result,
                settings = ?applied_settings_ref,
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
                let address = address_result.map_err(|inner| ExecutionError {
                    inner,
                    retries: retries_counter.load(std::sync::atomic::Ordering::Relaxed),
                    address: None,
                })?;

                let pool = self.pool.clone();

                let mut transport_client = R::Client::with_uri_and_settings(
                    address.uri().clone(),
                    applied_settings_ref,
                    &pool,
                )
                .map_err(|error| ExecutionError {
                    inner: DapiClientError::Transport(error),
                    retries: retries_counter.load(std::sync::atomic::Ordering::Relaxed),
                    address: Some(address.clone()),
                })?;

                let result = transport_request
                    .execute_transport(&mut transport_client, applied_settings_ref)
                    .await
                    .map_err(DapiClientError::Transport);

                let retries = retries_counter.load(std::sync::atomic::Ordering::Relaxed);

                let execution_result = result
                    .map(|inner| {
                        tracing::trace!(response = ?inner, "received {} response", response_name);

                        ExecutionResponse {
                            inner,
                            retries,
                            address: address.clone(),
                        }
                    })
                    .map_err(|inner| {
                        tracing::debug!(error = ?inner, "received error: {inner}");

                        ExecutionError {
                            inner,
                            retries,
                            address: Some(address.clone()),
                        }
                    });

                update_address_ban_status::<R::Response, DapiClientError>(
                    &self.address_list,
                    &execution_result,
                    applied_settings_ref,
                );

                execution_result
            }
        };

        let sleeper = transport::BackonSleeper::default();

        // Start the routine with retry policy applied:
        // We allow let_and_return because `result` is used later if dump feature is enabled
        let result: Result<
            ExecutionResponse<<R as TransportRequest>::Response>,
            ExecutionError<DapiClientError>,
        > = routine
            .retry(retry_settings)
            .sleep(sleeper)
            .notify(|error, duration| {
                let retries_counter = Arc::clone(&retries_counter_arc);
                retries_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                tracing::warn!(
                    ?error,
                    "retrying error with sleeping {} secs",
                    duration.as_secs_f32()
                );
            })
            .when(|e| e.can_retry())
            .instrument(tracing::info_span!("request routine"))
            .await;

        if let Err(error) = &result {
            if !error.can_retry() {
                tracing::error!(?error, "request failed");
            }
        }

        // Dump request and response to disk if dump_dir is set:
        #[cfg(feature = "dump")]
        Self::dump_request_response(&dump_request, &result, dump_dir);

        result
    }
}
