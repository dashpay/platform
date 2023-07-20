//! This crate provides [DapiClient] --- client for a decentralized API for Dash.
//!
//! # Examples
//! ```no_run
//! # use rs_dapi_client::{Settings, platform::GetIdentity, AddressList, DapiClient, DapiError};
//! # let _ = async {
//! let mut client = DapiClient::new(AddressList::new(), Settings::default());
//! let request = GetIdentity { id: b"0".to_vec() };
//! let response = client.execute(request, Settings::default()).await?;
//!
//! # Ok::<(), DapiError<_, _>>(())
//! # };
//! ```

#![deny(missing_docs)]

mod address_list;
pub mod core;
pub mod platform;
mod settings;
mod transport;

use std::fmt;

use backon::{ExponentialBuilder, Retryable};
use tracing::Instrument;

pub use address_list::AddressList;
pub use settings::Settings;
use transport::{TransportClient, TransportRequest};

/// DAPI request.
/// Since each of DAPI requests goes through the same execution process is was generalized
/// with this trait to run requests the same way.
pub trait DapiRequest: fmt::Debug {
    /// Response type for the request.
    type DapiResponse;

    /// Settings that will override [DapiClient]'s ones each time the request is executed.
    const SETTINGS_OVERRIDES: Settings;

    /// Error that may happen during conversion from transport-specific response to the
    /// DAPI response.
    type Error: std::error::Error;

    /// 1 to 1 mapping from the DAPI request to a type that represents a way for the data
    /// to be fetched.
    type TransportRequest: TransportRequest;

    /// Get the transport layer request.
    fn to_transport_request(&self) -> Self::TransportRequest;

    /// Attempts to build DAPI response specific to this DAPI request from transport layer data.
    fn try_from_transport_response(
        transport_response: <Self::TransportRequest as TransportRequest>::Response,
    ) -> Result<Self::DapiResponse, Self::Error>;
}

/// Access point to DAPI.
#[derive(Debug)]
pub struct DapiClient {
    address_list: AddressList,
    settings: Settings,
}

impl DapiClient {
    /// Initialize new [DapiClient] and optionally override default settings.
    pub fn new(address_list: AddressList, settings: Settings) -> Self {
        Self {
            address_list,
            settings,
        }
    }
}

/// General DAPI request error type.
#[derive(Debug, thiserror::Error)]
pub enum DapiError<TE, PE> {
    /// The error happened on transport layer
    #[error("transport error: {0}")]
    Transport(TE),
    /// Successful transport execution, but was unable to make a conversion from
    /// transport response to DAPI response.
    #[error("response parse error: {0}")]
    ParseResponse(PE),
    /// There are no valid peer addresses to use.
    #[error("no available addresses to use")]
    NoAvailableAddresses,
}

impl DapiClient {
    /// Execute the [DapiRequest].
    #[tracing::instrument]
    pub async fn execute<'c, R>(
        &'c mut self,
        request: R,
        settings: Settings,
    ) -> Result<
        R::DapiResponse,
        DapiError<<R::TransportRequest as TransportRequest>::Error, R::Error>,
    >
    where
        R: DapiRequest,
    {
        // Join settings of different sources to get final version of the settings for this execution:
        let applied_settings = self
            .settings
            .override_by(R::SETTINGS_OVERRIDES)
            .override_by(settings)
            .finalize();

        // Setup retry policy:
        let retry_settings = ExponentialBuilder::default().with_max_times(applied_settings.retries);

        // Setup DAPI request execution routine future. It's a closure that will be called more than
        // once to build new future on each retry:
        let routine = || {
            // Try to get an address to initialize transport on:
            let address = self
                .address_list
                .get_live_address()
                .ok_or(DapiError::NoAvailableAddresses);

            // Get a transport client requried by the DAPI request from this DAPI client.
            // It stays wrapped in [Result] since wa want to return future of [Result], not a
            // [Result] itself.
            let transport_client = address.map(|addr| {
                <R::TransportRequest as TransportRequest>::Client::with_uri(addr.uri().clone())
            });

            let transport_request = request.to_transport_request();

            async move {
                // On a lower layer DAPI requests should be executed as transport requests first:
                let transport_response = transport_request
                    .execute(&mut transport_client?, &applied_settings)
                    .await
                    .map_err(|e| DapiError::<_, <R as DapiRequest>::Error>::Transport(e))?;

                // Next try to build a proper DAPI response if possible:
                let dapi_response =
                    R::try_from_transport_response(transport_response).map_err(|e| {
                        DapiError::<
                            <<R as DapiRequest>::TransportRequest as TransportRequest>::Error,
                            _,
                        >::ParseResponse(e)
                    })?;

                Ok::<_, DapiError<_, _>>(dapi_response)
            }
        };

        // Start the routine with retry policy applied:
        // TODO: define what is retryable and what's not
        routine
            .retry(&retry_settings)
            .when(|e| !matches!(e, DapiError::NoAvailableAddresses))
            .instrument(tracing::info_span!("request routine"))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example() {
        // This is not actually a test but a sandbox to check if our loosely coupled
        // thing would agree to compile.

        let mut address_list = AddressList::new();
        address_list.add_uri("http://127.0.0.1".parse().expect("seems legit"));

        let mut dapi_client = DapiClient::new(
            address_list,
            Settings {
                timeout: Some(std::time::Duration::from_secs(1)),
                retries: Some(3),
                ..Settings::default()
            },
        );
        let _ = async {
            dapi_client
                .execute(platform::GetIdentity { id: vec![] }, Settings::default())
                .await
                .unwrap();
            dapi_client
                .execute(core::GetStatus {}, Settings::default())
                .await
                .unwrap();
        };
    }
}
