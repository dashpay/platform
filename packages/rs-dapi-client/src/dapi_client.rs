//! [DapiClient] definition.

use dapi_grpc::mock::Mockable;
use dapi_grpc::tonic::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use dapi_grpc::tonic::transport::Certificate;
use std::fmt::{Debug, Display};
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
#[derive(Debug, thiserror::Error, Clone)]
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
    /// All available addresses have been exhausted (banned due to errors).
    /// Contains the last meaningful error that caused addresses to be banned.
    #[error("no available addresses to retry, last error: {0}")]
    NoAvailableAddressesToRetry(
        #[cfg_attr(feature = "mocks", serde(with = "dapi_grpc::mock::serde_mockable"))]
        Box<TransportError>,
    ),
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
            NoAvailableAddressesToRetry(_) => false,
            Transport(transport_error) => transport_error.can_retry(),
            AddressList(_) => false,
            #[cfg(feature = "mocks")]
            Mock(_) => false,
        }
    }

    fn is_no_available_addresses(&self) -> bool {
        matches!(
            self,
            DapiClientError::NoAvailableAddresses | DapiClientError::NoAvailableAddressesToRetry(_)
        )
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

    /// Get all non-banned addresses from the address list.
    ///
    /// Returns a vector of addresses that are not currently banned or whose ban period has expired.
    /// This is useful for diagnostics, monitoring, or when you need to know which DAPI nodes are
    /// currently available for making requests.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use rs_dapi_client::{DapiClient, AddressList, RequestSettings};
    ///
    /// let address_list = "http://127.0.0.1:3000,http://127.0.0.1:3001".parse().unwrap();
    /// let client = DapiClient::new(address_list, RequestSettings::default());
    ///
    /// // Get all currently available (non-banned) addresses
    /// let live_addresses = client.get_live_addresses();
    /// println!("Available DAPI nodes: {}", live_addresses.len());
    /// ```
    pub fn get_live_addresses(&self) -> Vec<crate::Address> {
        self.address_list.get_live_addresses()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_address() -> crate::Address {
        "http://127.0.0.1:3000".parse().expect("valid address")
    }

    fn make_applied_settings(ban: bool) -> AppliedRequestSettings {
        AppliedRequestSettings {
            connect_timeout: None,
            timeout: Duration::from_secs(10),
            retries: 5,
            ban_failed_address: ban,
            max_decoding_message_size: None,
            #[cfg(not(target_arch = "wasm32"))]
            ca_certificate: None,
        }
    }

    #[test]
    fn test_can_retry_no_available_addresses() {
        let err = DapiClientError::NoAvailableAddresses;
        assert!(!err.can_retry());
    }

    #[test]
    fn test_can_retry_no_available_addresses_to_retry() {
        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("gone"));
        let err = DapiClientError::NoAvailableAddressesToRetry(Box::new(transport_err));
        assert!(!err.can_retry());
    }

    #[test]
    fn test_can_retry_transport_retryable() {
        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let err = DapiClientError::Transport(transport_err);
        assert!(err.can_retry());
    }

    #[test]
    fn test_can_retry_transport_non_retryable() {
        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::not_found("permanent"));
        let err = DapiClientError::Transport(transport_err);
        assert!(!err.can_retry());
    }

    #[test]
    fn test_can_retry_address_list_error() {
        let err =
            DapiClientError::AddressList(AddressListError::InvalidAddressUri("bad".to_string()));
        assert!(!err.can_retry());
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_can_retry_mock_error() {
        let err = DapiClientError::Mock(crate::mock::MockError::MockExpectationNotFound(
            "test".to_string(),
        ));
        assert!(!err.can_retry());
    }

    #[test]
    fn test_is_no_available_addresses() {
        assert!(DapiClientError::NoAvailableAddresses.is_no_available_addresses());

        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("gone"));
        assert!(
            DapiClientError::NoAvailableAddressesToRetry(Box::new(transport_err))
                .is_no_available_addresses()
        );

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        assert!(!DapiClientError::Transport(transport_err).is_no_available_addresses());
    }

    #[test]
    fn test_update_address_ban_status_success_unbans() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());
        address_list.ban(&addr);
        assert!(address_list.is_banned(&addr));

        let result: ExecutionResult<i32, DapiClientError> = Ok(ExecutionResponse {
            inner: 42,
            retries: 0,
            address: addr.clone(),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(true));

        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_success_on_unbanned_is_noop() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let result: ExecutionResult<i32, DapiClientError> = Ok(ExecutionResponse {
            inner: 42,
            retries: 0,
            address: addr.clone(),
        });

        // Should not panic or change anything
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_retryable_error_bans_address() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: Some(addr.clone()),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
        assert!(address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_retryable_error_ban_disabled() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: Some(addr.clone()),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(false));
        // With ban disabled, the address should NOT be banned
        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_non_retryable_error_does_not_ban() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());

        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::NoAvailableAddresses,
            retries: 0,
            address: Some(addr.clone()),
        });

        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
        assert!(!address_list.is_banned(&addr));
    }

    #[test]
    fn test_update_address_ban_status_retryable_error_no_address() {
        let address_list = AddressList::new();

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: None,
        });

        // Should not panic when address is None
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
    }

    #[test]
    fn test_update_address_ban_status_unban_removed_address() {
        let mut address_list = AddressList::new();
        let addr = mock_address();
        address_list.add(addr.clone());
        address_list.ban(&addr);

        // Remove the address
        address_list.remove(&addr);

        let result: ExecutionResult<i32, DapiClientError> = Ok(ExecutionResponse {
            inner: 42,
            retries: 0,
            address: addr.clone(),
        });

        // Should not panic when trying to unban a removed address
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
    }

    #[test]
    fn test_update_address_ban_status_ban_removed_address() {
        let address_list = AddressList::new();
        let addr = mock_address();

        let transport_err =
            TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("temporary"));
        let result: ExecutionResult<i32, DapiClientError> = Err(ExecutionError {
            inner: DapiClientError::Transport(transport_err),
            retries: 0,
            address: Some(addr),
        });

        // Should not panic when trying to ban an address not in the list
        update_address_ban_status(&address_list, &result, &make_applied_settings(true));
    }

    #[test]
    fn test_dapi_client_new() {
        let address_list: AddressList = "http://127.0.0.1:3000,http://127.0.0.1:3001"
            .parse()
            .unwrap();
        let client = DapiClient::new(address_list, RequestSettings::default());
        assert_eq!(client.address_list().len(), 2);
    }

    #[test]
    fn test_dapi_client_get_live_addresses() {
        let address_list: AddressList = "http://127.0.0.1:3000,http://127.0.0.1:3001"
            .parse()
            .unwrap();
        let client = DapiClient::new(address_list, RequestSettings::default());
        let live = client.get_live_addresses();
        assert_eq!(live.len(), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_dapi_client_with_ca_certificate() {
        let address_list: AddressList = "http://127.0.0.1:3000".parse().unwrap();
        let client = DapiClient::new(address_list, RequestSettings::default());
        let cert = dapi_grpc::tonic::transport::Certificate::from_pem("fake-pem-data");
        let client = client.with_ca_certificate(cert);
        assert!(client.ca_certificate.is_some());
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_dapi_client_error_mock_serialize_deserialize() {
        use dapi_grpc::mock::Mockable;

        let err = DapiClientError::NoAvailableAddresses;
        let serialized = err.mock_serialize().expect("should serialize");
        let deserialized =
            DapiClientError::mock_deserialize(&serialized).expect("should deserialize");
        assert!(matches!(
            deserialized,
            DapiClientError::NoAvailableAddresses
        ));
    }

    #[cfg(feature = "mocks")]
    #[test]
    fn test_dapi_client_error_transport_mock_roundtrip() {
        use dapi_grpc::mock::Mockable;

        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("test"));
        let err = DapiClientError::Transport(transport_err);
        let serialized = err.mock_serialize().expect("should serialize");
        let deserialized =
            DapiClientError::mock_deserialize(&serialized).expect("should deserialize");
        assert!(matches!(deserialized, DapiClientError::Transport(_)));
    }

    #[test]
    fn test_dapi_client_error_display() {
        let err = DapiClientError::NoAvailableAddresses;
        let display = format!("{}", err);
        assert!(display.contains("no available addresses"));

        let transport_err = TransportError::Grpc(dapi_grpc::tonic::Status::unavailable("gone"));
        let err = DapiClientError::NoAvailableAddressesToRetry(Box::new(transport_err));
        let display = format!("{}", err);
        assert!(display.contains("no available addresses to retry"));

        let err =
            DapiClientError::AddressList(AddressListError::InvalidAddressUri("bad".to_string()));
        let display = format!("{}", err);
        assert!(display.contains("address list error"));
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
        #[cfg(not(target_arch = "wasm32"))]
        let applied_settings = applied_settings.with_ca_certificate(self.ca_certificate.clone());

        // Save dump dir for later use
        #[cfg(feature = "dump")]
        let dump_dir = self.dump_dir.clone();
        #[cfg(feature = "dump")]
        let dump_request = request.clone();

        let max_retries = applied_settings.retries;
        let retry_delay = Duration::from_millis(10);

        let mut retries: usize = 0;
        // Track the last transport error for when all addresses get exhausted
        let mut last_transport_error: Option<TransportError> = None;

        let result: ExecutionResult<R::Response, DapiClientError> = async {
            loop {
                // Try to get an address to initialize transport on:
                let Some(address) = self.address_list.get_live_address() else {
                    // No available addresses - wrap with last meaningful error if we have one
                    let error = if let Some(transport_error) = last_transport_error.take() {
                        tracing::debug!(
                            "no addresses available, returning last transport error"
                        );
                        DapiClientError::NoAvailableAddressesToRetry(Box::new(
                            transport_error,
                        ))
                    } else {
                        DapiClientError::NoAvailableAddresses
                    };

                    return Err(ExecutionError {
                        inner: error,
                        retries,
                        address: None,
                    });
                };

                tracing::trace!(
                    ?request,
                    "calling {} with {} request",
                    request.method_name(),
                    request.request_name(),
                );

                let transport_request = request.clone();
                let response_name = request.response_name();

                // Try to create transport client
                let transport_client_result = R::Client::with_uri_and_settings(
                    address.uri().clone(),
                    &applied_settings,
                    &self.pool,
                );

                let mut transport_client = match transport_client_result {
                    Ok(client) => client,
                    Err(transport_error) => {
                        let can_retry_error = transport_error.can_retry();

                        // Clone error before moving it
                        let cloned_error = transport_error.clone();

                        let execution_error = ExecutionError {
                            inner: DapiClientError::Transport(transport_error),
                            retries,
                            address: Some(address.clone()),
                        };

                        update_address_ban_status::<R::Response, DapiClientError>(
                            &self.address_list,
                            &Err(execution_error.clone()),
                            &applied_settings,
                        );

                        if can_retry_error && retries < max_retries {
                            // Store last transport error
                            last_transport_error = Some(cloned_error);

                            retries += 1;
                            tracing::warn!(
                                error = ?execution_error,
                                "retrying error with sleeping {} secs",
                                retry_delay.as_secs_f32()
                            );
                            transport::sleep(retry_delay).await;
                            continue;
                        }

                        return Err(execution_error);
                    }
                };

                // Execute the transport request
                let result = transport_request
                    .execute_transport(&mut transport_client, &applied_settings)
                    .instrument(tracing::trace_span!(
                        "execute_request",
                        ?address,
                        settings = ?applied_settings,
                        method = request.method_name(),
                    ))
                    .await;

                let execution_result = match result {
                    Ok(response) => {
                        tracing::trace!(response = ?response, "received {} response", response_name);
                        Ok(ExecutionResponse {
                            inner: response,
                            retries,
                            address: address.clone(),
                        })
                    }
                    Err(transport_error) => {
                        tracing::debug!(error = ?transport_error, "received error: {transport_error}");
                        Err(ExecutionError {
                            inner: DapiClientError::Transport(transport_error),
                            retries,
                            address: Some(address.clone()),
                        })
                    }
                };

                update_address_ban_status::<R::Response, DapiClientError>(
                    &self.address_list,
                    &execution_result,
                    &applied_settings,
                );

                match execution_result {
                    Ok(response) => return Ok(response),
                    Err(error) => {
                        if error.can_retry() && retries < max_retries {
                            // Store last transport error
                            if let DapiClientError::Transport(ref te) = error.inner {
                                last_transport_error = Some(te.clone());
                            }

                            retries += 1;
                            tracing::warn!(
                                ?error,
                                "retrying error with sleeping {} secs",
                                retry_delay.as_secs_f32()
                            );
                            transport::sleep(retry_delay).await;
                            continue;
                        }

                        return Err(error);
                    }
                }
            }
        }
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
