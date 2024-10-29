//! [DapiClient] definition.

use backon::{ConstantBuilder, Retryable};
use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
use std::fmt::Debug;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::Instrument;

use crate::address_list::AddressListError;
use crate::connection_pool::ConnectionPool;
#[cfg(feature = "mocks")]
use crate::Address;
use crate::{
    transport::{TransportClient, TransportError, TransportRequest},
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
    address_list: Arc<RwLock<AddressList>>,
    settings: RequestSettings,
    pool: ConnectionPool,
    #[cfg(feature = "dump")]
    pub(crate) dump_dir: Option<std::path::PathBuf>,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: RequestSettings) -> Self {
        // multiply by 3 as we need to store core and platform addresses, and we want some spare capacity just in case
        let address_count = 3 * address_list.len();

        Self {
            address_list: Arc::new(RwLock::new(address_list)),
            settings,
            pool: ConnectionPool::new(address_count),
            #[cfg(feature = "dump")]
            dump_dir: None,
        }
    }

    /// Return the [DapiClient] address list.
    pub fn address_list(&self) -> &Arc<RwLock<AddressList>> {
        &self.address_list
    }
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

        // Setup DAPI request execution routine future. It's a closure that will be called
        // more once to build new future on each retry.
        let routine = move || {
            let retries_counter = Arc::clone(retries_counter_arc_ref);

            // Try to get an address to initialize transport on:
            let address_list = self
                .address_list
                .read()
                .expect("can't get address list for read");

            let address_result = address_list
                .get_live_address()
                .cloned()
                .ok_or(DapiClientError::NoAvailableAddresses);

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
                let address = address_result.map_err(|inner| ExecutionError {
                    inner,
                    retries: retries_counter.load(std::sync::atomic::Ordering::Acquire),
                    address: None,
                })?;

                let pool = self.pool.clone();

                let mut transport_client = R::Client::with_uri_and_settings(
                    address.uri().clone(),
                    &applied_settings,
                    &pool,
                )
                .map_err(|error| ExecutionError {
                    inner: DapiClientError::Transport(error),
                    retries: retries_counter.load(std::sync::atomic::Ordering::Acquire),
                    address: Some(address.clone()),
                })?;

                let response = transport_request
                    .execute_transport(&mut transport_client, &applied_settings)
                    .await
                    .map_err(DapiClientError::Transport);

                match &response {
                    Ok(_) => {
                        // Unban the address if it was banned and node responded successfully this time
                        if address.is_banned() {
                            let mut address_list = self
                                .address_list
                                .write()
                                .expect("can't get address list for write");

                            address_list.unban_address(&address).map_err(|error| {
                                ExecutionError {
                                    inner: DapiClientError::AddressList(error),
                                    retries: retries_counter
                                        .load(std::sync::atomic::Ordering::Acquire),
                                    address: Some(address.clone()),
                                }
                            })?;
                        }

                        tracing::trace!(?response, "received {} response", response_name);
                    }
                    Err(error) => {
                        if error.can_retry() {
                            if applied_settings.ban_failed_address {
                                let mut address_list = self
                                    .address_list
                                    .write()
                                    .expect("can't get address list for write");

                                address_list.ban_address(&address).map_err(|error| {
                                    ExecutionError {
                                        inner: DapiClientError::AddressList(error),
                                        retries: retries_counter
                                            .load(std::sync::atomic::Ordering::Acquire),
                                        address: Some(address.clone()),
                                    }
                                })?;
                            }
                        } else {
                            tracing::trace!(?error, "received error");
                        }
                    }
                };

                let retries = retries_counter.load(std::sync::atomic::Ordering::Acquire);

                response
                    .map(|inner| ExecutionResponse {
                        inner,
                        retries,
                        address: address.clone(),
                    })
                    .map_err(|inner| ExecutionError {
                        inner,
                        retries,
                        address: Some(address),
                    })
            }
        };

        // Start the routine with retry policy applied:
        // We allow let_and_return because `result` is used later if dump feature is enabled
        let result = routine
            .retry(retry_settings)
            .notify(|error, duration| {
                let retries_counter = Arc::clone(&retries_counter_arc);
                retries_counter.fetch_add(1, std::sync::atomic::Ordering::AcqRel);

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
